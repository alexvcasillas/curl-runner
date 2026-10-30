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
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&tmp, text)?;
    // The replaced file's permissions carry over before it is replaced.
    let kept = match std::fs::metadata(path) {
        Ok(m) => std::fs::set_permissions(&tmp, m.permissions()),
        Err(_) => Ok(()),
    };
    kept.and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
}
