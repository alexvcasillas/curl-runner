//! The studio's pure state model: no GPUI, so it compiles on every target.

pub mod collection;
pub mod collection_meta;
pub mod diagnostics;
pub mod document;
pub mod environments;
pub mod export;
// Parts of these (history, body diffs, Re-run failed, editing secrets) are
// picked up by later plan-2 tasks.
#[allow(dead_code)]
pub mod history;
#[allow(dead_code)]
pub mod result_view;
#[allow(dead_code)]
pub mod run;
#[allow(dead_code)]
pub mod secrets;
pub mod templates;

/// Writes `text` to `path` through a temp file in the same folder and a
/// rename, so a crash never leaves half a file.
pub fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    // The replaced file's permissions, from the start: the content is never
    // on disk under a wider mode than the file it replaces.
    let kept = std::fs::metadata(path).ok().map(|m| m.permissions());
    let _ = std::fs::remove_file(&tmp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if let Some(p) = &kept {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        options.mode(p.mode() & 0o7777);
    }
    let written = options.open(&tmp).and_then(|mut f| {
        f.write_all(text.as_bytes())?;
        // The umask may have narrowed it: exactly the replaced file's.
        match kept {
            Some(p) => std::fs::set_permissions(&tmp, p),
            None => Ok(()),
        }
    });
    written
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
}

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn write_atomic_keeps_a_private_mode() {
        use std::os::unix::fs::PermissionsExt as _;
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join(".env");
        std::fs::write(&p, "A=1\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        super::write_atomic(&p, "A=2\n").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "A=2\n");
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
        // A new file gets the default mode.
        super::write_atomic(&d.path().join("new"), "x").unwrap();
    }
}
