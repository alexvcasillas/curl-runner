//! A folder of YAML request files, shown as a tree.

use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::parser::parse_raw_stream;
use serde_json::Value as Json;
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
        /// Every request of a file without a single `request:` (a
        /// `requests:` list or a `collection:`), in run order; empty
        /// otherwise.
        requests: Vec<RequestEntry>,
        parse_ok: bool,
    },
}

/// One request of a multi-request file.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestEntry {
    pub method: String,
    /// `name`, else `url`, else `Request <n>`.
    pub name: String,
    /// Zero-based line of the request's list item, when found.
    pub line: Option<u32>,
}

/// What the tree shows for one file.
#[derive(Debug, Default, PartialEq)]
struct FileMeta {
    method: Option<String>,
    requests: Vec<RequestEntry>,
    parse_ok: bool,
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
    /// A received copy: the org that shared it. Its files are never
    /// created, renamed or deleted here (see [`read_only_error`]).
    pub read_only: Option<String>,
}

/// `Read only · shared by Acme`: what a refused write of a received copy
/// says.
pub fn read_only_text(org: &str) -> String {
    format!("Read only · shared by {org}")
}

/// The error a write to a received copy fails with.
pub fn read_only_error(org: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, read_only_text(org))
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
            read_only: None,
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
            let meta = std::fs::read_to_string(self.root.join(&rel))
                .map(|text| file_meta(&text))
                .unwrap_or_default();
            insert(&mut tree, &parts, &rel, meta);
        }
        sort(&mut tree);
        self.tree = tree;
    }

    /// Writes a new file; an existing `rel` fails with `AlreadyExists`.
    pub fn create_file(&mut self, rel: &Path, contents: &str) -> io::Result<()> {
        self.writable()?;
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
        self.writable()?;
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
        self.writable()?;
        let path = self.root.join(rel);
        if path.is_dir() {
            std::fs::remove_dir_all(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        self.refresh();
        Ok(())
    }

    /// Fails for a received copy.
    fn writable(&self) -> io::Result<()> {
        match &self.read_only {
            Some(org) => Err(read_only_error(org)),
            None => Ok(()),
        }
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

/// The tree's view of a file. Only a mapping root is ok: an empty file, a
/// scalar or a list parses but holds no request. A file whose only request
/// is `request:` gets its method; any other gets its request list, in the
/// order the runner runs them (`request`, `requests`, `collection.requests`).
fn file_meta(text: &str) -> FileMeta {
    let Ok(doc) = parse_raw_stream(text) else {
        return FileMeta::default();
    };
    if !doc.is_object() {
        return FileMeta::default();
    }
    let collection = doc.get("collection");
    let default_method = [
        collection.and_then(|c| c.get("defaults")),
        doc.get("global").and_then(|g| g.get("defaults")),
    ]
    .into_iter()
    .flatten()
    .find_map(|d| d.get("method").and_then(|m| m.as_str()))
    .unwrap_or("GET");
    let method_of = |r: &Json| {
        r.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or(default_method)
            .to_string()
    };
    let single = doc.get("request").filter(|r| r.is_object());
    let lines: Vec<&str> = text.lines().collect();
    let mut requests = Vec::new();
    for (list, key_line) in [
        (doc.get("requests"), list_key_line(&lines, None)),
        (
            collection.and_then(|c| c.get("requests")),
            list_key_line(&lines, Some("collection")),
        ),
    ] {
        let Some(items) = list.and_then(Json::as_array) else {
            continue;
        };
        let item_lines =
            key_line.map_or_else(Vec::new, |(at, indent)| list_item_lines(&lines, at, indent));
        for (i, r) in items.iter().enumerate() {
            requests.push(RequestEntry {
                method: method_of(r),
                name: request_name(r, requests.len()),
                line: item_lines.get(i).copied(),
            });
        }
    }
    match single {
        Some(r) if requests.is_empty() => FileMeta {
            method: Some(method_of(r)),
            requests,
            parse_ok: true,
        },
        Some(r) => {
            let line = lines
                .iter()
                .position(|l| is_key(l, "request"))
                .map(|n| n as u32);
            requests.insert(
                0,
                RequestEntry {
                    method: method_of(r),
                    name: request_name(r, 0),
                    line,
                },
            );
            FileMeta {
                method: None,
                requests,
                parse_ok: true,
            }
        }
        None => FileMeta {
            method: None,
            requests,
            parse_ok: true,
        },
    }
}

/// The zero-based line of each request of `text` in run order (`None` where
/// it was not found): one entry for a lone `request:`, none when the file
/// does not parse.
pub fn request_lines(text: &str) -> Vec<Option<u32>> {
    let meta = file_meta(text);
    if meta.method.is_some() {
        let line = text
            .lines()
            .position(|l| is_key(l, "request"))
            .map(|n| n as u32);
        return vec![line];
    }
    meta.requests.iter().map(|r| r.line).collect()
}

/// The request whose item holds `line`: the last one starting at or above
/// it, else the first.
pub fn request_at_line(lines: &[Option<u32>], line: u32) -> usize {
    lines
        .iter()
        .rposition(|l| l.is_some_and(|l| l <= line))
        .unwrap_or(0)
}

fn request_name(r: &Json, index: usize) -> String {
    ["name", "url"]
        .into_iter()
        .find_map(|k| r.get(k).and_then(|v| v.as_str()).filter(|v| !v.is_empty()))
        .map_or_else(|| format!("Request {}", index + 1), str::to_string)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// `line` is `key:` with a block value (nothing after the colon but a comment).
fn is_key(line: &str, key: &str) -> bool {
    line.trim_start()
        .strip_prefix(key)
        .and_then(|r| r.strip_prefix(':'))
        .is_some_and(|r| {
            let r = r.trim();
            r.is_empty() || r.starts_with('#')
        })
}

/// `(line, indent)` of the block `requests:` key at the root, or directly
/// under the root key `parent`.
fn list_key_line(lines: &[&str], parent: Option<&str>) -> Option<(usize, usize)> {
    let Some(parent) = parent else {
        return lines
            .iter()
            .position(|l| indent_of(l) == 0 && is_key(l, "requests"))
            .map(|n| (n, 0));
    };
    let start = lines
        .iter()
        .position(|l| indent_of(l) == 0 && is_key(l, parent))?;
    let mut child_indent = None;
    for (n, l) in lines.iter().enumerate().skip(start + 1) {
        if is_blank(l) {
            continue;
        }
        let indent = indent_of(l);
        if indent == 0 {
            return None;
        }
        let child = *child_indent.get_or_insert(indent);
        if indent == child && is_key(l, "requests") {
            return Some((n, indent));
        }
    }
    None
}

/// Lines of the items of the block list under the key at `key_line`.
fn list_item_lines(lines: &[&str], key_line: usize, key_indent: usize) -> Vec<u32> {
    let mut out = Vec::new();
    let mut item_indent = None;
    for (n, l) in lines.iter().enumerate().skip(key_line + 1) {
        if is_blank(l) {
            continue;
        }
        let indent = indent_of(l);
        let t = l.trim_start();
        let is_item = t == "-" || t.starts_with("- ");
        match item_indent {
            None if is_item && indent >= key_indent => {
                item_indent = Some(indent);
                out.push(n as u32);
            }
            None => break,
            Some(i) if indent < i || (indent == i && !is_item) => break,
            Some(i) if indent == i => out.push(n as u32),
            Some(_) => {}
        }
    }
    out
}

fn insert(level: &mut Vec<TreeNode>, parts: &[String], rel: &Path, meta: FileMeta) {
    let [first, rest @ ..] = parts else { return };
    if rest.is_empty() {
        level.push(TreeNode::File {
            name: first.clone(),
            rel_path: rel.to_path_buf(),
            method: meta.method,
            requests: meta.requests,
            parse_ok: meta.parse_ok,
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
        insert(children, rest, rel, meta);
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

    #[test]
    fn request_lines_follow_the_cursor() {
        let flow = "# flow\nrequests:\n  - name: One\n    url: a\n  - name: Two\n    url: b\n";
        let lines = request_lines(flow);
        assert_eq!(lines, [Some(2), Some(4)]);
        assert_eq!(request_at_line(&lines, 0), 0, "above the first: the first");
        assert_eq!(request_at_line(&lines, 3), 0);
        assert_eq!(request_at_line(&lines, 4), 1);
        assert_eq!(request_at_line(&lines, 99), 1);
        assert_eq!(request_lines("request:\n  url: a\n"), [Some(0)]);
        assert!(request_lines("[").is_empty());
        assert_eq!(request_at_line(&[], 3), 0);
    }

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

    fn entries(meta: &FileMeta) -> Vec<(&str, &str, Option<u32>)> {
        meta.requests
            .iter()
            .map(|r| (r.method.as_str(), r.name.as_str(), r.line))
            .collect()
    }

    #[test]
    fn requests_list_gives_entries_with_lines() {
        let meta = file_meta(
            "# flow\nrequests:\n  - name: Get user\n    url: https://x.example.com/u\n\n  - url: https://x.example.com/p\n    method: POST\n  -\n    method: PUT\n",
        );
        assert!(meta.parse_ok);
        assert_eq!(meta.method, None);
        assert_eq!(
            entries(&meta),
            [
                ("GET", "Get user", Some(2)),
                ("POST", "https://x.example.com/p", Some(5)),
                ("PUT", "Request 3", Some(7)),
            ]
        );
    }

    #[test]
    fn unindented_list_items_are_found() {
        let meta =
            file_meta("requests:\n- name: a\n  url: u\n- name: b\n  url: u\nglobal:\n  x: 1\n");
        assert_eq!(
            entries(&meta),
            [("GET", "a", Some(1)), ("GET", "b", Some(3))]
        );
    }

    #[test]
    fn collection_requests_use_collection_default_method() {
        let meta = file_meta(
            "collection:\n  name: S\n  defaults:\n    method: POST\n  requests:\n    - name: OK\n      url: u\n    - name: Get\n      method: GET\n      url: u\n",
        );
        assert_eq!(
            entries(&meta),
            [("POST", "OK", Some(5)), ("GET", "Get", Some(7))]
        );
    }

    #[test]
    fn request_plus_requests_lists_both() {
        let meta = file_meta("request:\n  url: a\nrequests:\n  - url: b\n    method: DELETE\n");
        assert_eq!(meta.method, None);
        assert_eq!(
            entries(&meta),
            [("GET", "a", Some(0)), ("DELETE", "b", Some(3))]
        );
    }

    #[test]
    fn single_request_keeps_method_and_no_entries() {
        let meta = file_meta("request:\n  url: a\n  method: PATCH\n");
        assert_eq!(meta.method.as_deref(), Some("PATCH"));
        assert!(meta.requests.is_empty());
    }

    #[test]
    fn flow_list_has_entries_without_lines() {
        let meta = file_meta("requests: [{url: a}, {url: b, name: B}]\n");
        assert_eq!(entries(&meta), [("GET", "a", None), ("GET", "B", None)]);
    }

    #[test]
    fn a_read_only_collection_refuses_every_write() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.yaml"), "request:\n  url: a\n").unwrap();
        let mut c = Collection::open(dir.path()).unwrap();
        c.read_only = Some("Acme".into());
        for e in [
            c.create_file(Path::new("b.yaml"), "x").unwrap_err(),
            c.rename(Path::new("a.yaml"), Path::new("c.yaml"))
                .unwrap_err(),
            c.delete(Path::new("a.yaml")).unwrap_err(),
        ] {
            assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(e.to_string(), "Read only · shared by Acme");
        }
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["a.yaml"]);
    }
}
