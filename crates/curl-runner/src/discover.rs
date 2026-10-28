//! YAML file discovery (TS `findYamlFiles`, Bun `Glob.scan('.')`).
//!
//! - No patterns: `*.yaml`/`*.yml` in `cwd` (`**/` too with `--all`).
//! - A directory: its `*.yaml`/`*.yml` (recursively with `--all`).
//! - A file, or a glob: matched as given; results keep the pattern's
//!   literal prefix (`./a.yaml` stays `./a.yaml`).
//! - Hidden entries are not matched by wildcards; symlinked directories are
//!   not followed (Bun defaults). Only `.yaml`/`.yml` files are kept; the
//!   result is de-duplicated and sorted.
//! - Absolute paths work (Bun's scan never matched them; logged).

use globset::{GlobBuilder, GlobMatcher};
use std::collections::BTreeSet;
use std::path::Path;

fn is_glob(s: &str) -> bool {
    s.contains(['*', '?', '[', '{'])
}

/// Find the YAML files for `patterns` relative to `cwd`.
pub fn find_yaml_files(patterns: &[String], all: bool, cwd: &Path) -> Vec<String> {
    let mut search: Vec<String> = Vec::new();
    if patterns.is_empty() {
        search.extend(["*.yaml", "*.yml"].map(String::from));
        if all {
            search.extend(["**/*.yaml", "**/*.yml"].map(String::from));
        }
    } else {
        for p in patterns {
            let path = cwd.join(p);
            if !is_glob(p) && path.is_dir() {
                let dir = p.trim_end_matches('/');
                let dir = if dir.is_empty() { "/" } else { dir };
                let join = |tail: &str| {
                    if dir == "/" {
                        format!("/{tail}")
                    } else {
                        format!("{dir}/{tail}")
                    }
                };
                search.push(join("*.yaml"));
                search.push(join("*.yml"));
                if all {
                    search.push(join("**/*.yaml"));
                    search.push(join("**/*.yml"));
                }
            } else {
                search.push(p.clone());
            }
        }
    }

    let mut found = BTreeSet::new();
    for pattern in &search {
        scan(pattern, cwd, &mut found);
    }
    found
        .into_iter()
        .filter(|f| f.ends_with(".yaml") || f.ends_with(".yml"))
        .collect()
}

fn scan(pattern: &str, cwd: &Path, found: &mut BTreeSet<String>) {
    if !is_glob(pattern) {
        if cwd.join(pattern).is_file() {
            found.insert(pattern.to_string());
        }
        return;
    }
    // Split into the literal directory prefix and the glob remainder.
    let parts: Vec<&str> = pattern.split('/').collect();
    let first_glob = parts.iter().position(|p| is_glob(p)).unwrap_or(parts.len());
    let prefix = parts[..first_glob].join("/");
    let rest: Vec<&str> = parts[first_glob..]
        .iter()
        .copied()
        .filter(|p| !p.is_empty())
        .collect();
    let rest_pat = rest.join("/");
    let Ok(glob) = GlobBuilder::new(&rest_pat).literal_separator(true).build() else {
        return;
    };
    let matcher = glob.compile_matcher();
    let max_depth = if rest.contains(&"**") {
        usize::MAX
    } else {
        rest.len()
    };
    let base = if prefix.is_empty() && pattern.starts_with('/') {
        Path::new("/").to_path_buf()
    } else {
        cwd.join(&prefix)
    };
    let shown_prefix = if prefix.is_empty() && pattern.starts_with('/') {
        "/".to_string()
    } else if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}/")
    };
    walk(&base, "", 1, max_depth, &matcher, &mut |rel| {
        found.insert(format!("{shown_prefix}{rel}"));
    });
}

fn walk(
    dir: &Path,
    rel: &str,
    depth: usize,
    max_depth: usize,
    m: &GlobMatcher,
    hit: &mut dyn FnMut(&str),
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let Ok(ft) = e.file_type() else { continue };
        let is_file = ft.is_file() || (ft.is_symlink() && e.path().is_file());
        if is_file && m.is_match(&path) {
            hit(&path);
        } else if ft.is_dir() && depth < max_depth {
            walk(&e.path(), &path, depth + 1, max_depth, m, hit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let p = t.path();
        for f in [
            "a.yaml",
            "b.yml",
            "c.json",
            ".hidden.yaml",
            "tests/t1.yaml",
            "tests/t2.yml",
            "tests/nested/n1.yaml",
            "tests/.dot/x.yaml",
            "other/o.yaml",
        ] {
            let path = p.join(f);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "request: {url: x}").unwrap();
        }
        t
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_patterns() {
        let t = tree();
        assert_eq!(
            find_yaml_files(&[], false, t.path()),
            s(&["a.yaml", "b.yml"])
        );
        assert_eq!(
            find_yaml_files(&[], true, t.path()),
            s(&[
                "a.yaml",
                "b.yml",
                "other/o.yaml",
                "tests/nested/n1.yaml",
                "tests/t1.yaml",
                "tests/t2.yml"
            ])
        );
    }

    #[test]
    fn directories() {
        let t = tree();
        assert_eq!(
            find_yaml_files(&s(&["tests/"]), false, t.path()),
            s(&["tests/t1.yaml", "tests/t2.yml"])
        );
        assert_eq!(
            find_yaml_files(&s(&["./tests"]), true, t.path()),
            s(&[
                "./tests/nested/n1.yaml",
                "./tests/t1.yaml",
                "./tests/t2.yml"
            ])
        );
    }

    #[test]
    fn files_and_globs_keep_their_spelling() {
        let t = tree();
        assert_eq!(
            find_yaml_files(
                &s(&["./a.yaml", "tests/*.yaml", "missing.yaml", "c.json"]),
                false,
                t.path()
            ),
            s(&["./a.yaml", "tests/t1.yaml"])
        );
        assert_eq!(
            find_yaml_files(&s(&["**/n1.yaml"]), false, t.path()),
            s(&["tests/nested/n1.yaml"])
        );
        assert_eq!(
            find_yaml_files(&s(&["t*/t1.yaml"]), false, t.path()),
            s(&["tests/t1.yaml"])
        );
    }

    #[test]
    fn absolute_paths_and_dedup() {
        let t = tree();
        let abs = t.path().join("a.yaml").to_string_lossy().into_owned();
        assert_eq!(
            find_yaml_files(&[abs.clone(), abs.clone()], false, t.path()),
            vec![abs]
        );
        let abs_glob = format!("{}/tests/*.yml", t.path().display());
        assert_eq!(
            find_yaml_files(&[abs_glob], false, t.path()),
            vec![format!("{}/tests/t2.yml", t.path().display())]
        );
    }
}
