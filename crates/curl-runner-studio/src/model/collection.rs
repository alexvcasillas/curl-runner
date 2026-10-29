//! A folder of YAML request files, shown as a tree.

use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::parser::parse_raw_stream;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum TreeNode {
    Folder {
        name: String,
        children: Vec<TreeNode>,
    },
    File {
        name: String,
        rel_path: PathBuf,
        /// `request.method` of single-request files; `None` for `requests:`.
        method: Option<String>,
        parse_ok: bool,
    },
}

impl TreeNode {
    fn name(&self) -> &str {
        match self {
            TreeNode::Folder { name, .. } | TreeNode::File { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub root: PathBuf,
    pub name: String,
    pub tree: Vec<TreeNode>,
}

impl Collection {
    pub fn open(root: &Path) -> io::Result<Collection> {
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "collection root is not a directory",
            ));
        }
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let mut c = Collection {
            root: root.to_path_buf(),
            name,
            tree: Vec::new(),
        };
        c.refresh();
        Ok(c)
    }

    pub fn refresh(&mut self) {
        let files: Vec<PathBuf> = find_yaml_files(&[], true, &self.root)
            .into_iter()
            .map(|f| PathBuf::from(f.strip_prefix("./").unwrap_or(&f)))
            .filter(|p| p != Path::new("curl-runner.yaml"))
            .collect();
        let mut tree = Vec::new();
        for rel in files {
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let (method, parse_ok) = read_meta(&self.root.join(&rel));
            insert(&mut tree, &parts, &rel, method, parse_ok);
        }
        sort(&mut tree);
        self.tree = tree;
    }

    /// Writes a new file; an existing `rel` fails with `AlreadyExists`.
    pub fn create_file(&mut self, rel: &Path, contents: &str) -> io::Result<()> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(contents.as_bytes())?;
        self.refresh();
        Ok(())
    }

    /// Moves `from` to `to`; an existing `to` fails with `AlreadyExists`,
    /// unless it is `from` itself under another letter case (a case-only
    /// rename on a case-insensitive disk).
    pub fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        let src = self.root.join(from);
        let dest = self.root.join(to);
        if dest.symlink_metadata().is_ok() && !is_same_file(&src, &dest) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", to.display()),
            ));
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(src, dest)?;
        self.refresh();
        Ok(())
    }

    pub fn delete(&mut self, rel: &Path) -> io::Result<()> {
        let path = self.root.join(rel);
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        self.refresh();
        Ok(())
    }
}

#[cfg(unix)]
fn is_same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (a.metadata(), b.metadata()) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

/// Windows disks are case-insensitive: an existing destination that matches
/// the source ignoring case is the source.
#[cfg(not(unix))]
fn is_same_file(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

/// `(method, parse_ok)` for a file. Only a mapping root is ok: an empty
/// file, a scalar or a list parses but holds no request.
fn read_meta(path: &Path) -> (Option<String>, bool) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, false);
    };
    let Ok(doc) = parse_raw_stream(&text) else {
        return (None, false);
    };
    if !doc.is_object() {
        return (None, false);
    }
    let method = doc.get("request").map(|r| {
        r.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET")
            .to_string()
    });
    (method, true)
}

fn insert(
    level: &mut Vec<TreeNode>,
    parts: &[String],
    rel: &Path,
    method: Option<String>,
    parse_ok: bool,
) {
    let [first, rest @ ..] = parts else { return };
    if rest.is_empty() {
        level.push(TreeNode::File {
            name: first.clone(),
            rel_path: rel.to_path_buf(),
            method,
            parse_ok,
        });
        return;
    }
    let existing = level
        .iter()
        .position(|n| matches!(n, TreeNode::Folder { name, .. } if name == first));
    let idx = existing.unwrap_or_else(|| {
        level.push(TreeNode::Folder {
            name: first.clone(),
            children: Vec::new(),
        });
        level.len() - 1
    });
    if let TreeNode::Folder { children, .. } = &mut level[idx] {
        insert(children, rest, rel, method, parse_ok);
    }
}

fn sort(level: &mut [TreeNode]) {
    level.sort_by_key(|n| (matches!(n, TreeNode::File { .. }), n.name().to_lowercase()));
    for n in level {
        if let TreeNode::Folder { children, .. } = n {
            sort(children);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &[u8]) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn file<'a>(tree: &'a [TreeNode], name: &str) -> &'a TreeNode {
        tree.iter().find(|n| n.name() == name).unwrap()
    }

    #[test]
    fn tree_orders_folders_first_and_skips_root_config() {
        let d = tempfile::tempdir().unwrap();
        for f in ["a.yaml", "B.yaml", "curl-runner.yaml", "auth/login.yaml"] {
            write(d.path(), f, b"request:\n  url: https://api.example.com\n");
        }
        let c = Collection::open(d.path()).unwrap();
        let names: Vec<&str> = c.tree.iter().map(TreeNode::name).collect();
        assert_eq!(names, ["auth", "a.yaml", "B.yaml"]);
    }

    #[test]
    fn method_read_from_single_request_files() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "p.yaml",
            b"request:\n  url: https://x.example.com\n  method: POST\n",
        );
        write(
            d.path(),
            "g.yaml",
            b"request:\n  url: https://x.example.com\n",
        );
        write(
            d.path(),
            "m.yaml",
            b"requests:\n  - url: https://x.example.com\n",
        );
        let c = Collection::open(d.path()).unwrap();
        let method = |n| match file(&c.tree, n) {
            TreeNode::File { method, .. } => method.clone(),
            _ => panic!(),
        };
        assert_eq!(method("p.yaml").as_deref(), Some("POST"));
        assert_eq!(method("g.yaml").as_deref(), Some("GET"));
        assert_eq!(method("m.yaml"), None);
    }

    #[test]
    fn unparseable_and_non_utf8_files_listed_with_parse_ok_false() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "bad.yaml", b"a: [unclosed\n");
        write(d.path(), "bin.yaml", &[0xff, 0xfe, 0x00]);
        // Parses, but not to a mapping: no request can come from it.
        write(d.path(), "empty.yaml", b"");
        write(d.path(), "scalar.yaml", b"just text\n");
        write(d.path(), "list.yaml", b"- url: https://api.example.com\n");
        let c = Collection::open(d.path()).unwrap();
        for n in [
            "bad.yaml",
            "bin.yaml",
            "empty.yaml",
            "scalar.yaml",
            "list.yaml",
        ] {
            assert!(
                matches!(
                    file(&c.tree, n),
                    TreeNode::File {
                        parse_ok: false,
                        ..
                    }
                ),
                "{n}"
            );
        }
    }

    #[test]
    fn create_file_refuses_existing_destination() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.yaml", b"original\n");
        let mut c = Collection::open(d.path()).unwrap();
        let err = c.create_file(Path::new("a.yaml"), "new\n").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "original\n"
        );
    }

    #[test]
    fn rename_refuses_existing_destination() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.yaml", b"a\n");
        write(d.path(), "b.yaml", b"b\n");
        let mut c = Collection::open(d.path()).unwrap();
        let err = c
            .rename(Path::new("a.yaml"), Path::new("b.yaml"))
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "a\n"
        );
        assert_eq!(
            std::fs::read_to_string(d.path().join("b.yaml")).unwrap(),
            "b\n"
        );
    }

    #[test]
    fn rename_allows_case_only_change() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.yaml", b"a\n");
        let mut c = Collection::open(d.path()).unwrap();
        c.rename(Path::new("a.yaml"), Path::new("A.yaml")).unwrap();
        assert!(c.tree.iter().any(|n| n.name() == "A.yaml"));
    }

    #[test]
    fn create_rename_delete_refresh_tree() {
        let d = tempfile::tempdir().unwrap();
        let mut c = Collection::open(d.path()).unwrap();
        assert!(c.tree.is_empty());
        c.create_file(
            Path::new("x/a.yaml"),
            "request:\n  url: https://a.example.com\n",
        )
        .unwrap();
        assert!(matches!(&c.tree[0], TreeNode::Folder { name, .. } if name == "x"));
        c.rename(Path::new("x/a.yaml"), Path::new("b.yaml"))
            .unwrap();
        assert!(c.tree.iter().any(|n| n.name() == "b.yaml"));
        c.delete(Path::new("b.yaml")).unwrap();
        assert!(c.tree.iter().all(|n| n.name() != "b.yaml"));
    }
}
