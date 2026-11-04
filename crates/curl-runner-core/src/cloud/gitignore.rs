//! A pure `.gitignore` check: nested files apply to their own directory.
//! Supports `*`, `**`, a leading `/`, a trailing `/`, `!` and `#` comments.

use std::collections::HashMap;
use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};

struct Rule {
    matcher: GlobMatcher,
    negate: bool,
    dir_only: bool,
}

fn parse_line(line: &str) -> Option<Rule> {
    let line = line.trim_end_matches(['\r', ' ']);
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (negate, line) = match line.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    let (dir_only, line) = match line.strip_suffix('/') {
        Some(rest) => (true, rest),
        None => (false, line),
    };
    let anchored = line.contains('/');
    let body = line.trim_start_matches('/');
    if body.is_empty() {
        return None;
    }
    let pattern = if anchored {
        body.to_string()
    } else {
        format!("**/{body}")
    };
    let matcher = GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()
        .ok()?
        .compile_matcher();
    Some(Rule {
        matcher,
        negate,
        dir_only,
    })
}

/// The `.gitignore` files under one root, read on first use.
pub(super) struct Ignores<'a> {
    root: &'a Path,
    /// Rules per directory (`""` for the root, else `a/b`).
    by_dir: HashMap<String, Vec<Rule>>,
}

impl<'a> Ignores<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self {
            root,
            by_dir: HashMap::new(),
        }
    }

    fn rules(&mut self, dir: &str) -> &[Rule] {
        let root = self.root;
        self.by_dir.entry(dir.to_string()).or_insert_with(|| {
            std::fs::read_to_string(root.join(dir).join(".gitignore"))
                .map(|t| t.lines().filter_map(parse_line).collect())
                .unwrap_or_default()
        })
    }

    /// Whether git ignores the file at `rel` (`/`-separated, relative to the
    /// root). A file under an ignored directory is ignored.
    pub fn is_ignored(&mut self, rel: &str) -> bool {
        let parts: Vec<&str> = rel.split('/').collect();
        for end in 1..=parts.len() {
            let is_dir = end < parts.len();
            let mut ignored = false;
            for depth in 0..end {
                let dir = parts[..depth].join("/");
                let below = parts[depth..end].join("/");
                for rule in self.rules(&dir) {
                    if (is_dir || !rule.dir_only) && rule.matcher.is_match(&below) {
                        ignored = !rule.negate;
                    }
                }
            }
            if ignored {
                return true;
            }
        }
        false
    }
}
