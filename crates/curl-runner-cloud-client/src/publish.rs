//! Publishing a collection folder: the local checks and the request. The CLI
//! and the studio run this same code; each reports [`Problem`]s its own way.

use curl_runner_cloud_proto::{BundleFile, FileChanges, GitMeta, Input, PublishRequest};
use curl_runner_core::cloud::{
    SkipReason, find_secret_literals, load_cloud_config, scan_bundle, valid_slug,
};
use curl_runner_core::config::CONFIG_FILE_PATHS;
use curl_runner_core::lint::{Issue, LintOutcome, ROOT_PATH, Severity, is_valid, lint_text};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// One collection, checked and ready to send.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub collection: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub inputs: Vec<Input>,
    pub files: Vec<BundleFile>,
    pub hash: String,
}

impl Target {
    pub fn request(&self, git: Option<GitMeta>) -> PublishRequest {
        PublishRequest {
            files: self.files.clone(),
            hash: self.hash.clone(),
            inputs: self.inputs.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            git,
        }
    }
}

/// Something the checks found. Paths are relative to the folder.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// A dotted config (`.curl-runner.yaml`): the bundle leaves dotfiles
    /// out, so it would go missing without a word.
    DottedConfig(String),
    /// `curl-runner.yaml` does not parse or its `cloud:` block is wrong.
    Config(String),
    /// No `cloud.collection` and no slug was given.
    NoCollection,
    InvalidSlug(String),
    /// The folder can't be read, or the bundle breaks a server limit.
    Scan(String),
    TooBig(String),
    Unreadable(String),
    /// A link that resolves outside the folder. Left out; not a failure.
    OutsideRoot(String),
    NoFiles,
    /// A request file with lint issues; `valid` when none is an error.
    Lint {
        path: String,
        issues: Vec<Issue>,
        valid: bool,
    },
    /// The linter gave up on a file.
    LintCrash {
        path: String,
        message: String,
    },
    /// A secret input with a literal value in a file.
    Secret {
        file: String,
        name: String,
    },
}

impl Problem {
    /// Whether this stops the publish.
    pub fn fails(&self) -> bool {
        !matches!(
            self,
            Problem::OutsideRoot(_) | Problem::Lint { valid: true, .. }
        )
    }
}

/// What [`check`] found: the target when nothing fails, and every problem
/// in the order it was found.
#[derive(Debug, Default)]
pub struct Checked {
    pub target: Option<Target>,
    pub problems: Vec<Problem>,
}

/// Check `root` the way the server will, and more: config, slug, file
/// limits, lint and secret literals. `slug` overrides `cloud.collection`.
pub fn check(root: &Path, slug: Option<&str>) -> Checked {
    let mut out = Checked::default();
    let stop = |out: &mut Checked, p: Problem| {
        out.problems.push(p);
        std::mem::take(out)
    };
    if let Some(name) = CONFIG_FILE_PATHS
        .iter()
        .find(|n| root.join(n).is_file())
        .filter(|n| n.starts_with('.'))
    {
        return stop(&mut out, Problem::DottedConfig((*name).to_string()));
    }
    let config = match load_cloud_config(root) {
        Ok(c) => c,
        Err(e) => return stop(&mut out, Problem::Config(e.to_string())),
    };
    let (collection, name, description, inputs) = match (config, slug) {
        (Some(c), over) => (
            over.map_or(c.collection, str::to_string),
            c.name,
            c.description,
            c.inputs,
        ),
        (None, Some(s)) => (s.to_string(), None, None, Vec::new()),
        (None, None) => return stop(&mut out, Problem::NoCollection),
    };
    if !valid_slug(&collection) {
        out.problems.push(Problem::InvalidSlug(collection.clone()));
    }
    let bundle = match scan_bundle(root) {
        Ok(b) => b,
        Err(e) => return stop(&mut out, Problem::Scan(e.to_string())),
    };
    for (path, why) in &bundle.skipped {
        let p = path.display().to_string();
        out.problems.push(match why {
            SkipReason::TooBig => Problem::TooBig(p),
            SkipReason::Unreadable => Problem::Unreadable(p),
            SkipReason::OutsideRoot => Problem::OutsideRoot(p),
        });
    }
    if bundle.files.is_empty() {
        out.problems.push(Problem::NoFiles);
    }
    for f in &bundle.files {
        // The config is not a request file; `load_cloud_config` parsed it.
        if CONFIG_FILE_PATHS.contains(&f.path.as_str()) {
            continue;
        }
        match lint_issues(&f.text) {
            Ok(issues) if issues.is_empty() => {}
            Ok(issues) => out.problems.push(Problem::Lint {
                path: f.path.clone(),
                valid: is_valid(&issues),
                issues,
            }),
            Err(message) => out.problems.push(Problem::LintCrash {
                path: f.path.clone(),
                message,
            }),
        }
    }
    for leak in find_secret_literals(&bundle, &inputs) {
        out.problems.push(Problem::Secret {
            file: leak.file,
            name: leak.name,
        });
    }
    if !out.problems.iter().any(Problem::fails) {
        out.target = Some(Target {
            collection,
            name,
            description,
            inputs,
            files: bundle.files,
            hash: bundle.hash,
        });
    }
    out
}

/// A file's lint issues; a parse failure is one error at the root.
fn lint_issues(text: &str) -> Result<Vec<Issue>, String> {
    match lint_text(text).map_err(|c| c.0)? {
        LintOutcome::ParseError(message) => Ok(vec![Issue {
            path: ROOT_PATH.into(),
            severity: Severity::Error,
            message,
            fix: None,
        }]),
        LintOutcome::Linted { issues, .. } => Ok(issues),
    }
}

/// What publishing `local` over `remote` adds, changes and removes.
pub fn diff(local: &[BundleFile], remote: &[BundleFile]) -> FileChanges {
    let old: HashMap<&str, &str> = remote
        .iter()
        .map(|f| (f.path.as_str(), f.text.as_str()))
        .collect();
    let mut out = FileChanges::default();
    for f in local {
        match old.get(f.path.as_str()) {
            None => out.added.push(f.path.clone()),
            Some(t) if *t != f.text => out.changed.push(f.path.clone()),
            Some(_) => {}
        }
    }
    let new: HashSet<&str> = local.iter().map(|f| f.path.as_str()).collect();
    out.removed = remote
        .iter()
        .filter(|f| !new.contains(f.path.as_str()))
        .map(|f| f.path.clone())
        .collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for (p, t) in files {
            let path = d.path().join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, t).unwrap();
        }
        d
    }

    const REQ: &str = "request:\n  url: https://api.example.com/x\n  method: GET\n";

    fn f(path: &str, text: &str) -> BundleFile {
        BundleFile {
            path: path.into(),
            text: text.into(),
        }
    }

    #[test]
    fn clean_folder_gives_a_target() {
        let d = folder(&[
            (
                "curl-runner.yaml",
                "cloud:\n  collection: payments-api\n  name: Payments\n",
            ),
            ("a.yaml", REQ),
        ]);
        let c = check(d.path(), None);
        assert_eq!(c.problems, []);
        let t = c.target.unwrap();
        assert_eq!(t.collection, "payments-api");
        assert_eq!(t.name.as_deref(), Some("Payments"));
        assert_eq!(t.files.len(), 2);
        let r = t.request(None);
        assert_eq!((r.hash, r.files.len()), (t.hash, 2));
    }

    #[test]
    fn slug_overrides_and_stands_in_for_the_config() {
        let d = folder(&[("a.yaml", REQ)]);
        assert_eq!(check(d.path(), None).problems, [Problem::NoCollection]);
        let t = check(d.path(), Some("mine")).target.unwrap();
        assert_eq!((t.collection.as_str(), t.name), ("mine", None));
        assert_eq!(
            check(d.path(), Some("Bad Slug")).problems,
            [Problem::InvalidSlug("Bad Slug".into())]
        );
    }

    #[test]
    fn dotted_config_stops_early() {
        let d = folder(&[(".curl-runner.yaml", "cloud:\n  collection: x\n")]);
        let c = check(d.path(), None);
        assert!(c.target.is_none());
        assert_eq!(
            c.problems,
            [Problem::DottedConfig(".curl-runner.yaml".into())]
        );
    }

    #[test]
    fn lint_errors_fail_and_parse_errors_are_lint_errors() {
        let d = folder(&[
            ("a.yaml", "request:\n  method: GET\n"),
            ("b.yaml", ":\n- ["),
        ]);
        let c = check(d.path(), Some("x"));
        assert!(c.target.is_none());
        let paths: Vec<_> = c
            .problems
            .iter()
            .map(|p| match p {
                Problem::Lint { path, valid, .. } => (path.as_str(), *valid),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(paths, [("a.yaml", false), ("b.yaml", false)]);
    }

    #[test]
    fn secret_literal_fails() {
        let d = folder(&[
            (
                "curl-runner.yaml",
                "cloud:\n  collection: x\n  inputs:\n    - name: SECRET_KEY\n",
            ),
            (
                "a.yaml",
                "global:\n  variables:\n    SECRET_KEY: hunter2\nrequest:\n  url: https://api.example.com/x\n  method: GET\n",
            ),
        ]);
        let c = check(d.path(), None);
        assert!(c.target.is_none());
        assert!(
            c.problems.contains(&Problem::Secret {
                file: "a.yaml".into(),
                name: "SECRET_KEY".into()
            }),
            "{:?}",
            c.problems
        );
    }

    #[test]
    fn empty_folder_has_no_files() {
        let d = folder(&[]);
        assert_eq!(check(d.path(), Some("x")).problems, [Problem::NoFiles]);
    }

    #[test]
    fn only_outside_root_and_valid_lint_pass() {
        assert!(!Problem::OutsideRoot("a".into()).fails());
        let lint = |valid| Problem::Lint {
            path: "a".into(),
            issues: Vec::new(),
            valid,
        };
        assert!(!lint(true).fails());
        assert!(lint(false).fails());
        assert!(Problem::NoFiles.fails());
    }

    #[test]
    fn diff_counts_added_changed_removed() {
        let local = [f("a", "1"), f("b", "2"), f("c", "3")];
        let remote = [f("a", "1"), f("b", "old"), f("z", "9")];
        let d = diff(&local, &remote);
        assert_eq!(
            (d.added, d.changed, d.removed),
            (
                vec!["c".to_string()],
                vec!["b".to_string()],
                vec!["z".to_string()]
            )
        );
    }
}
