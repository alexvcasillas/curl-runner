//! The studio's pure state model: no GPUI, so it compiles on every target.

// Sign-in, accounts, sync and the Share dialog use it now; what is left
// unused is the receiving half (link parsing, resolving, received shares,
// the shipping fixture), for plan 3's Tasks 5–7.
#[cfg_attr(not(test), allow(dead_code))]
pub mod backend;
pub mod collection;
pub mod collection_meta;
pub mod conflict;
pub mod diagnostics;
pub mod document;
pub mod environments;
pub mod export;
pub mod history;
pub mod import;
pub mod result_view;
pub mod run;
pub mod secrets;
pub mod settings;
pub mod shared_copy;
pub mod sync_base;
pub mod templates;

/// A duration as every screen and report shows it: `196 ms`, `4.8 s`,
/// `2 min 4 s`.
pub fn duration_text(ms: f64) -> String {
    if ms < 1000. {
        format!("{} ms", ms.round() as u64)
    } else if ms < 60_000. {
        format!("{:.1} s", ms / 1000.)
    } else {
        let s = (ms / 1000.).round() as u64;
        format!("{} min {} s", s / 60, s % 60)
    }
}

/// Writes `text` to `path` through a temp file in the same folder and a
/// rename, so a crash never leaves half a file.
pub fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    // Unique per process and call: concurrent writers never share one.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{name}.{}.{seq}.tmp", std::process::id()));
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

#[cfg(test)]
#[test]
fn duration_text_reads_like_the_boards() {
    assert_eq!(duration_text(196.4), "196 ms");
    assert_eq!(duration_text(1240.), "1.2 s");
    assert_eq!(duration_text(4_800.), "4.8 s");
    assert_eq!(duration_text(124_000.), "2 min 4 s");
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
    #[test]
    fn concurrent_atomic_writes_never_collide() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("base.json");
        let texts: Vec<String> = (0..16).map(|i| format!("{i}\n").repeat(500)).collect();
        std::thread::scope(|s| {
            for t in &texts {
                let p = &p;
                s.spawn(move || super::write_atomic(p, t).unwrap());
            }
        });
        let end = std::fs::read_to_string(&p).unwrap();
        assert!(texts.contains(&end), "one whole write wins");
        let left = std::fs::read_dir(d.path()).unwrap().count();
        assert_eq!(left, 1, "no temp files left");
    }
}
