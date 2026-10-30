//! Resolving a sync conflict: the two texts side by side as a line diff,
//! and what each choice does on disk and asks of the server.
//!
//! - **Keep mine**: the disk is left as it is; the local text (or the
//!   local deletion) goes up with the server's current hash as its base,
//!   so the server takes it as a normal update.
//! - **Take theirs**: the server's text is written exactly (or, when the
//!   server deleted the file, the local file is removed); nothing goes up.
//! - **Keep both**: the server's text is written next to the file as
//!   `{stem}.server.yaml` (`.server-2.yaml`… when taken; never over another
//!   file), then it is Keep mine. When this side deleted the file, the
//!   server's text lands in that copy and the deletion goes up. When the
//!   server deleted it there is only one text, so it is not offered.
//!
//! Nothing happens when the disk no longer holds what the conflict
//! recorded as local ([`ResolveError::ChangedSince`]): a fresh sync shows
//! the conflict again with the new text.

use super::backend::{Conflict, SyncDeletion, SyncFile};
use super::shared_copy::{is_received, resolves_inside, writable};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    KeepMine,
    TakeTheirs,
    KeepBoth,
}

impl Choice {
    /// Whether `c` offers this choice. A received copy takes the owner's
    /// version only; with the server's copy deleted there is nothing to
    /// keep both of.
    pub fn offered(self, c: &Conflict, read_only: bool) -> bool {
        match self {
            Choice::TakeTheirs => true,
            Choice::KeepMine => !read_only,
            Choice::KeepBoth => !read_only && !c.server_deleted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The file changed on disk since the conflict was recorded.
    ChangedSince,
    /// The path is not a visible YAML file inside the collection.
    Refused,
    Io(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::ChangedSince => {
                f.write_str("This file changed since the conflict — sync again")
            }
            ResolveError::Refused => f.write_str("This file can't be written here"),
            ResolveError::Io(e) => write!(f, "Couldn't write the file: {e}"),
        }
    }
}

/// What goes up to the server for a resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Push {
    File(SyncFile),
    Delete(SyncDeletion),
}

/// What a resolution did on disk and still needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// To push before the conflict is settled (Keep mine, Keep both).
    pub push: Option<Push>,
    /// The file's synced hash once settled; `None` drops it from the base.
    pub base: Option<u64>,
    /// Files written or removed (open tabs re-read them).
    pub touched: Vec<PathBuf>,
    /// Keep both's copy of the server's text.
    pub copy: Option<PathBuf>,
}

/// The disk text of `path`: `Ok(None)` when it is not there.
fn read(path: &Path) -> Result<Option<String>, ResolveError> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ResolveError::Io(e.to_string())),
    }
}

/// Whether the disk still holds what `c` recorded as local.
pub fn unchanged_since(root: &Path, c: &Conflict) -> Result<bool, ResolveError> {
    let disk = read(&root.join(&c.rel))?;
    Ok(match disk {
        None => c.local_deleted,
        Some(d) => !c.local_deleted && SyncFile::hash_of(&d) == SyncFile::hash_of(&c.local),
    })
}

/// `rel` under `root`, only as a visible YAML file whose folder resolves
/// inside the collection (the checks pulls get), its folder made.
fn target(root: &Path, rel: &Path) -> Result<PathBuf, ResolveError> {
    let real_root = root
        .canonicalize()
        .map_err(|e| ResolveError::Io(e.to_string()))?;
    let path = root.join(rel);
    let dir = path.parent().ok_or(ResolveError::Refused)?;
    if !writable(rel) || !resolves_inside(dir, &real_root) {
        return Err(ResolveError::Refused);
    }
    std::fs::create_dir_all(dir).map_err(|e| ResolveError::Io(e.to_string()))?;
    // Again once it exists: nothing may have swapped in a link.
    if !dir.canonicalize().is_ok_and(|d| d.starts_with(&real_root)) {
        return Err(ResolveError::Refused);
    }
    Ok(path)
}

/// `rates/get.yaml` → `rates/get.server.yaml`, then `get.server-2.yaml`…
/// (`.yml` stays `.yml`).
pub fn server_copy_name(rel: &Path, n: usize) -> PathBuf {
    let stem = rel
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = rel
        .extension()
        .map_or_else(|| "yaml".into(), |e| e.to_string_lossy().into_owned());
    let name = match n {
        0 | 1 => format!("{stem}.server.{ext}"),
        n => format!("{stem}.server-{n}.{ext}"),
    };
    rel.with_file_name(name)
}

/// Writes `text` as a new file beside `rel`: the first free
/// `{stem}.server.yaml` name, or one that already holds exactly `text`
/// (a retried Keep both). Never replaces another file.
fn write_copy(root: &Path, rel: &Path, text: &str) -> Result<PathBuf, ResolveError> {
    use std::io::Write as _;
    for n in 1..1000 {
        let copy = server_copy_name(rel, n);
        let path = target(root, &copy)?;
        if read(&path)?.is_some_and(|t| t == text) {
            return Ok(copy);
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(text.as_bytes())
                    .and_then(|()| f.sync_all())
                    .map_err(|e| ResolveError::Io(e.to_string()))?;
                return Ok(copy);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(ResolveError::Io(e.to_string())),
        }
    }
    Err(ResolveError::Io(
        "no free name for the server's copy".into(),
    ))
}

/// The local side going up, based on the server's current copy.
fn keep_mine(c: &Conflict) -> Push {
    let server = (!c.server_deleted).then(|| SyncFile::hash_of(&c.server));
    if c.local_deleted {
        Push::Delete(SyncDeletion {
            rel: c.rel.clone(),
            // A deletion of a file the server lacks is a no-op anyway.
            base: server.unwrap_or_default(),
        })
    } else {
        Push::File(SyncFile::new(c.rel.clone(), c.local.clone()).with_base(server))
    }
}

/// Does `choice`'s disk part for `c` under `root`, after checking the disk
/// still holds `c.local`. Blocking: run it off the UI thread.
///
/// A read-only (received) collection takes the owner's copy only: `Keep
/// mine` and `Keep both` (which writes a `.server.yaml` copy) are refused
/// when `read_only` says so, or when the folder itself is a received copy
/// ([`is_received`] under `base`), whatever the caller thought.
pub fn resolve_on_disk(
    root: &Path,
    c: &Conflict,
    choice: Choice,
    read_only: bool,
    base: Option<&Path>,
) -> Result<Resolved, ResolveError> {
    if !writable(&c.rel) {
        return Err(ResolveError::Refused);
    }
    let read_only = read_only || is_received(base, root);
    if !choice.offered(c, read_only) {
        return Err(ResolveError::Refused);
    }
    if !unchanged_since(root, c)? {
        return Err(ResolveError::ChangedSince);
    }
    let mine = || (!c.local_deleted).then(|| SyncFile::hash_of(&c.local));
    match choice {
        Choice::KeepMine => Ok(Resolved {
            push: Some(keep_mine(c)),
            base: mine(),
            touched: Vec::new(),
            copy: None,
        }),
        Choice::KeepBoth => {
            if c.server_deleted {
                return Err(ResolveError::Refused);
            }
            let copy = write_copy(root, &c.rel, &c.server)?;
            Ok(Resolved {
                push: Some(keep_mine(c)),
                base: mine(),
                touched: vec![copy.clone()],
                copy: Some(copy),
            })
        }
        Choice::TakeTheirs => {
            let path = target(root, &c.rel)?;
            if c.server_deleted {
                match std::fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(ResolveError::Io(e.to_string())),
                }
                return Ok(Resolved {
                    push: None,
                    base: None,
                    touched: vec![c.rel.clone()],
                    copy: None,
                });
            }
            super::write_atomic(&path, &c.server).map_err(|e| ResolveError::Io(e.to_string()))?;
            Ok(Resolved {
                push: None,
                base: Some(SyncFile::hash_of(&c.server)),
                touched: vec![c.rel.clone()],
                copy: None,
            })
        }
    }
}

/// One side of a [`DiffRow`]: a 1-based line number and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffCell {
    pub line: usize,
    pub text: String,
    /// Removed (left) or added (right); else the same on both sides.
    pub changed: bool,
}

/// One row of the two columns. A side is `None` where the other side has
/// lines this one lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    pub left: Option<DiffCell>,
    pub right: Option<DiffCell>,
}

/// Above this many line pairs, the changed middle is shown as removed then
/// added rather than diffed (keeps big files cheap: the table is 4 MB).
const LCS_LIMIT: usize = 1_000_000;

/// The line diff of `old` (left) against `new` (right), as aligned rows:
/// unchanged lines side by side; a run of removals beside the run of
/// additions that replaced it. Common ends are trimmed before the LCS.
pub fn line_diff(old: &str, new: &str) -> Vec<DiffRow> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    // Edit script for the middle: (in a, in b) per step.
    let mut ops: Vec<(bool, bool)> = Vec::with_capacity(ma.len() + mb.len());
    if ma.len().saturating_mul(mb.len()) > LCS_LIMIT {
        ops.extend(std::iter::repeat_n((true, false), ma.len()));
        ops.extend(std::iter::repeat_n((false, true), mb.len()));
    } else {
        // lcs[i][j]: the LCS length of ma[i..] and mb[j..].
        let w = mb.len() + 1;
        let mut lcs = vec![0u32; (ma.len() + 1) * w];
        for i in (0..ma.len()).rev() {
            for j in (0..mb.len()).rev() {
                lcs[i * w + j] = if ma[i] == mb[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < ma.len() || j < mb.len() {
            if i < ma.len() && j < mb.len() && ma[i] == mb[j] {
                ops.push((true, true));
                (i, j) = (i + 1, j + 1);
            } else if j == mb.len() || (i < ma.len() && lcs[(i + 1) * w + j] >= lcs[i * w + j + 1])
            {
                ops.push((true, false));
                i += 1;
            } else {
                ops.push((false, true));
                j += 1;
            }
        }
    }
    let cell = |lines: &[&str], ix: usize, changed: bool| DiffCell {
        line: ix + 1,
        text: lines[ix].to_string(),
        changed,
    };
    let mut rows = Vec::with_capacity(a.len().max(b.len()));
    for ix in 0..prefix {
        rows.push(DiffRow {
            left: Some(cell(&a, ix, false)),
            right: Some(cell(&b, ix, false)),
        });
    }
    let (mut i, mut j) = (prefix, prefix);
    let mut k = 0;
    while k < ops.len() {
        if ops[k] == (true, true) {
            rows.push(DiffRow {
                left: Some(cell(&a, i, false)),
                right: Some(cell(&b, j, false)),
            });
            (i, j, k) = (i + 1, j + 1, k + 1);
            continue;
        }
        // A changed run: removals and additions until the next same line.
        let (mut dels, mut adds) = (Vec::new(), Vec::new());
        while k < ops.len() && ops[k] != (true, true) {
            if ops[k].0 {
                dels.push(cell(&a, i, true));
                i += 1;
            } else {
                adds.push(cell(&b, j, true));
                j += 1;
            }
            k += 1;
        }
        let n = dels.len().max(adds.len());
        let (mut dels, mut adds) = (dels.into_iter(), adds.into_iter());
        for _ in 0..n {
            rows.push(DiffRow {
                left: dels.next(),
                right: adds.next(),
            });
        }
    }
    for _ in 0..suffix {
        rows.push(DiffRow {
            left: Some(cell(&a, i, false)),
            right: Some(cell(&b, j, false)),
        });
        (i, j) = (i + 1, j + 1);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conflict(rel: &str, local: &str, server: &str) -> Conflict {
        Conflict {
            rel: rel.into(),
            local: local.into(),
            server: server.into(),
            server_author: "Sam".into(),
            server_at: chrono::Utc::now(),
            local_deleted: false,
            server_deleted: false,
        }
    }

    /// `(left, right)` per row: `-x`/`+x` changed, ` x` same, `.` none.
    fn shape(rows: &[DiffRow]) -> Vec<(String, String)> {
        let side = |c: &Option<DiffCell>, mark: char| match c {
            None => ".".to_string(),
            Some(c) if c.changed => format!("{mark}{}", c.text),
            Some(c) => format!(" {}", c.text),
        };
        rows.iter()
            .map(|r| (side(&r.left, '-'), side(&r.right, '+')))
            .collect()
    }

    #[test]
    fn diff_pairs_changes_and_keeps_line_numbers() {
        let old = "a\nb\nc\nd\n";
        let new = "a\nB\nc\nd\ne\n";
        let rows = line_diff(old, new);
        let s = |l: &str, r: &str| (l.to_string(), r.to_string());
        assert_eq!(
            shape(&rows),
            [
                s(" a", " a"),
                s("-b", "+B"),
                s(" c", " c"),
                s(" d", " d"),
                s(".", "+e")
            ]
        );
        assert_eq!(rows[4].right.as_ref().unwrap().line, 5);
        assert_eq!(line_diff("x\n", "x\n").len(), 1);
        assert!(line_diff("", "").is_empty());
        // Removed only, inserted in the middle.
        let rows = line_diff("a\nb\nc\n", "a\nc\n");
        assert_eq!(shape(&rows)[1], s("-b", "."));
        let rows = line_diff("a\nc\n", "a\nb\nc\n");
        assert_eq!(shape(&rows)[1], s(".", "+b"));
        assert_eq!(rows[2].left.as_ref().unwrap().line, 2);
        assert_eq!(rows[2].right.as_ref().unwrap().line, 3);
    }

    #[test]
    fn diff_of_huge_middles_falls_back_without_lcs() {
        let old: String = (0..3000).map(|i| format!("a{i}\n")).collect();
        let new: String = (0..3000).map(|i| format!("b{i}\n")).collect();
        let rows = line_diff(&old, &new);
        assert_eq!(rows.len(), 3000);
        assert!(rows.iter().all(|r| r.left.as_ref().unwrap().changed));
        // Just over a million pairs: no LCS either. One shared line in the
        // middle would pair up under an LCS; here it shows removed, then
        // added.
        let side = |p: &str| -> String {
            (0..1001)
                .map(|i| {
                    if i == 500 {
                        "same\n".into()
                    } else {
                        format!("{p}{i}\n")
                    }
                })
                .collect()
        };
        let rows = line_diff(&side("a"), &side("b"));
        assert!(!rows.iter().any(|r| {
            r.left
                .as_ref()
                .is_some_and(|c| !c.changed && c.text == "same")
        }));
    }

    #[test]
    fn keep_mine_pushes_local_on_the_servers_hash() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), "a: mine\n").unwrap();
        let c = conflict("a.yaml", "a: mine\n", "a: theirs\n");
        let r = resolve_on_disk(d.path(), &c, Choice::KeepMine, false, None).unwrap();
        let Some(Push::File(f)) = r.push else {
            panic!()
        };
        assert_eq!(f.text, "a: mine\n");
        assert_eq!(f.base, Some(SyncFile::hash_of("a: theirs\n")));
        assert_eq!(r.base, Some(SyncFile::hash_of("a: mine\n")));
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "a: mine\n"
        );
    }

    #[test]
    fn take_theirs_writes_the_server_text_exactly() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), "a: mine\n").unwrap();
        let theirs = "a: theirs\r\n# no trailing newline";
        let c = conflict("a.yaml", "a: mine\n", theirs);
        let r = resolve_on_disk(d.path(), &c, Choice::TakeTheirs, false, None).unwrap();
        assert_eq!(r.push, None);
        assert_eq!(r.base, Some(SyncFile::hash_of(theirs)));
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            theirs
        );
    }

    #[test]
    fn keep_both_never_overwrites_a_file() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("rates")).unwrap();
        std::fs::write(d.path().join("rates/get.yaml"), "a: mine\n").unwrap();
        std::fs::write(d.path().join("rates/get.server.yaml"), "other\n").unwrap();
        let c = conflict("rates/get.yaml", "a: mine\n", "a: theirs\n");
        let r = resolve_on_disk(d.path(), &c, Choice::KeepBoth, false, None).unwrap();
        assert_eq!(r.copy, Some(PathBuf::from("rates/get.server-2.yaml")));
        let read = |rel: &str| std::fs::read_to_string(d.path().join(rel)).unwrap();
        assert_eq!(read("rates/get.server.yaml"), "other\n");
        assert_eq!(read("rates/get.server-2.yaml"), "a: theirs\n");
        assert_eq!(read("rates/get.yaml"), "a: mine\n");
        assert!(matches!(r.push, Some(Push::File(_))));
        // Again (a retry after a failed push): the same copy, no third.
        let r = resolve_on_disk(d.path(), &c, Choice::KeepBoth, false, None).unwrap();
        assert_eq!(r.copy, Some(PathBuf::from("rates/get.server-2.yaml")));
        assert!(!d.path().join("rates/get.server-3.yaml").exists());
        assert_eq!(
            server_copy_name(Path::new("b.yml"), 1),
            PathBuf::from("b.server.yml")
        );
    }

    #[test]
    fn a_file_changed_since_the_conflict_is_left_alone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), "a: newer\n").unwrap();
        let c = conflict("a.yaml", "a: mine\n", "a: theirs\n");
        for choice in [Choice::KeepMine, Choice::TakeTheirs, Choice::KeepBoth] {
            assert_eq!(
                resolve_on_disk(d.path(), &c, choice, false, None),
                Err(ResolveError::ChangedSince)
            );
        }
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "a: newer\n"
        );
        assert!(!d.path().join("a.server.yaml").exists());
        assert_eq!(
            ResolveError::ChangedSince.to_string(),
            "This file changed since the conflict — sync again"
        );
        // Deleted here, but back on disk since.
        let gone = Conflict {
            local_deleted: true,
            local: String::new(),
            ..c
        };
        assert_eq!(
            resolve_on_disk(d.path(), &gone, Choice::KeepMine, false, None),
            Err(ResolveError::ChangedSince)
        );
    }

    #[test]
    fn local_deletion_cases() {
        let d = tempfile::tempdir().unwrap();
        let c = Conflict {
            local_deleted: true,
            ..conflict("a.yaml", "", "a: theirs\n")
        };
        let r = resolve_on_disk(d.path(), &c, Choice::KeepMine, false, None).unwrap();
        assert_eq!(
            r.push,
            Some(Push::Delete(SyncDeletion {
                rel: "a.yaml".into(),
                base: SyncFile::hash_of("a: theirs\n")
            }))
        );
        assert_eq!(r.base, None);
        assert!(!d.path().join("a.yaml").exists());
        let r = resolve_on_disk(d.path(), &c, Choice::KeepBoth, false, None).unwrap();
        assert!(matches!(r.push, Some(Push::Delete(_))));
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.server.yaml")).unwrap(),
            "a: theirs\n"
        );
        assert!(!d.path().join("a.yaml").exists());
        std::fs::remove_file(d.path().join("a.server.yaml")).unwrap();
        resolve_on_disk(d.path(), &c, Choice::TakeTheirs, false, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            "a: theirs\n"
        );
    }

    #[test]
    fn server_deletion_cases() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.yaml"), "a: mine\n").unwrap();
        let c = Conflict {
            server_deleted: true,
            ..conflict("a.yaml", "a: mine\n", "")
        };
        assert!(!Choice::KeepBoth.offered(&c, false));
        let r = resolve_on_disk(d.path(), &c, Choice::KeepMine, false, None).unwrap();
        let Some(Push::File(f)) = r.push else {
            panic!()
        };
        assert_eq!((f.text.as_str(), f.base), ("a: mine\n", None));
        assert!(d.path().join("a.yaml").exists());
        let r = resolve_on_disk(d.path(), &c, Choice::TakeTheirs, false, None).unwrap();
        assert_eq!((r.push, r.base), (None, None));
        assert!(!d.path().join("a.yaml").exists());
    }

    #[test]
    fn read_only_copies_only_take_theirs() {
        let c = conflict("a.yaml", "a\n", "b\n");
        assert!(Choice::TakeTheirs.offered(&c, true));
        assert!(!Choice::KeepMine.offered(&c, true));
        assert!(!Choice::KeepBoth.offered(&c, true));
        assert!(Choice::KeepBoth.offered(&c, false));
    }

    #[test]
    fn read_only_collections_refuse_keep_both_and_mine_on_disk() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("shared/acme-shipping-api");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.yaml"), "a: mine\n").unwrap();
        let c = conflict("a.yaml", "a: mine\n", "a: theirs\n");
        // Told it is read only, or found under `{base}/shared/`.
        for (read_only, base) in [(true, None), (false, Some(base.path()))] {
            for choice in [Choice::KeepBoth, Choice::KeepMine] {
                assert_eq!(
                    resolve_on_disk(&root, &c, choice, read_only, base),
                    Err(ResolveError::Refused),
                    "{choice:?} {read_only}"
                );
            }
        }
        assert!(!root.join("a.server.yaml").exists(), "no copy written");
        assert_eq!(
            std::fs::read_to_string(root.join("a.yaml")).unwrap(),
            "a: mine\n"
        );
        let r = resolve_on_disk(&root, &c, Choice::TakeTheirs, true, Some(base.path())).unwrap();
        assert_eq!(r.push, None);
        assert_eq!(
            std::fs::read_to_string(root.join("a.yaml")).unwrap(),
            "a: theirs\n"
        );
    }

    #[test]
    fn unsafe_paths_are_refused() {
        let d = tempfile::tempdir().unwrap();
        for rel in [".env", "../x.yaml", "a/.git/x.yaml"] {
            let c = Conflict {
                local_deleted: true,
                ..conflict(rel, "", "x: 1\n")
            };
            for choice in [Choice::TakeTheirs, Choice::KeepBoth] {
                assert_eq!(
                    resolve_on_disk(d.path(), &c, choice, false, None),
                    Err(ResolveError::Refused),
                    "{rel} {choice:?}"
                );
            }
        }
        assert!(!d.path().join(".env").exists());
        assert!(!d.path().join(".env.server.yaml").exists());
    }
}
