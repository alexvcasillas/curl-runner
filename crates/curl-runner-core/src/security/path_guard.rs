//! Filesystem path confinement (port of `core/security/path-guard.ts`).

use std::path::{Component, Path, PathBuf};

use super::GuardError;

/// Lexically normalise (`.` and `..`), without touching the filesystem.
/// Like Node's `path.resolve`, symlinks are NOT resolved.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                // `..` at the root stays at the root.
                if !matches!(
                    out.components().next_back(),
                    Some(Component::RootDir | Component::Prefix(_)) | None
                ) {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve `path` (relative to `base_dir`, which is itself relative to `cwd`
/// when not absolute) and require the result to be `cwd` or inside it,
/// unless `allow` (`--allow-path`). Purely lexical, like TS `path.resolve`
/// plus a prefix check; `cwd` must be absolute when confining.
///
/// With `allow`, the path is returned joined but not normalised.
pub fn resolve_safe(
    path: &str,
    base_dir: &Path,
    cwd: &Path,
    allow: bool,
) -> Result<PathBuf, GuardError> {
    let joined = cwd.join(base_dir).join(path);
    if allow {
        return Ok(joined);
    }
    if !cwd.is_absolute() {
        return Err(GuardError::new(format!(
            "Cannot confine path \"{path}\": working directory \"{}\" is not absolute",
            cwd.display()
        )));
    }
    let resolved = normalize(&joined);
    // Component-wise prefix: exact match or strictly inside (`/a/b` is not
    // inside `/a/bc`).
    if !resolved.starts_with(normalize(cwd)) {
        return Err(GuardError::new(format!(
            "Path \"{path}\" escapes the working directory. Use --allow-path to permit it."
        )));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CWD: &str = "/work/proj";
    fn rs(p: &str) -> Result<PathBuf, GuardError> {
        resolve_safe(p, Path::new(CWD), Path::new(CWD), false)
    }
    fn msg(p: &str) -> String {
        rs(p).unwrap_err().to_string()
    }

    #[test]
    fn in_cwd_paths_pass() {
        assert_eq!(
            rs("output.json"),
            Ok(PathBuf::from("/work/proj/output.json"))
        );
        assert_eq!(
            rs("subdir/nested/output.json"),
            Ok(PathBuf::from("/work/proj/subdir/nested/output.json"))
        );
        assert_eq!(rs("."), Ok(PathBuf::from(CWD)));
        assert_eq!(
            rs("/work/proj/somefile.txt"),
            Ok(PathBuf::from("/work/proj/somefile.txt"))
        );
        assert_eq!(rs("a/../b.txt"), Ok(PathBuf::from("/work/proj/b.txt")));
    }

    #[test]
    fn escaping_paths_are_rejected() {
        for p in [
            "../escape.json",
            "../../etc/passwd",
            "/etc/passwd",
            "/tmp/output.json",
            "subdir/../../escape.json",
            "/work/proj/../other/x",
        ] {
            let e = msg(p);
            assert!(e.contains("escapes the working directory"), "{p}: {e}");
            assert!(e.contains("--allow-path"), "{p}: {e}");
        }
    }

    #[test]
    fn error_message_is_exact_and_includes_input() {
        assert_eq!(
            msg("../secret.key"),
            "Path \"../secret.key\" escapes the working directory. Use --allow-path to permit it."
        );
    }

    #[test]
    fn sibling_prefix_is_not_inside() {
        assert!(rs("/work/proj-evil/x").is_err());
        assert!(rs("../proj-evil/x").is_err());
    }

    #[test]
    fn allow_bypasses_confinement() {
        let a = |p: &str| resolve_safe(p, Path::new(CWD), Path::new(CWD), true);
        assert_eq!(a("/etc/passwd"), Ok(PathBuf::from("/etc/passwd")));
        assert_eq!(a("/tmp/output.json"), Ok(PathBuf::from("/tmp/output.json")));
        assert_eq!(
            a("../escape.json"),
            Ok(PathBuf::from("/work/proj/../escape.json"))
        );
    }

    #[test]
    fn custom_cwd() {
        let r = |p: &str, c: &str| resolve_safe(p, Path::new(c), Path::new(c), false);
        assert_eq!(r("data.json", "/tmp"), Ok(PathBuf::from("/tmp/data.json")));
        assert!(r("../escape.json", "/tmp/myapp").is_err());
        assert_eq!(
            r("/tmp/myapp/out.json", "/tmp/myapp"),
            Ok(PathBuf::from("/tmp/myapp/out.json"))
        );
    }

    #[test]
    fn base_dir_inside_cwd_resolves_relative_to_base() {
        let r = resolve_safe(
            "./a.txt",
            Path::new("/work/proj/sub"),
            Path::new(CWD),
            false,
        );
        assert_eq!(r, Ok(PathBuf::from("/work/proj/sub/a.txt")));
        // relative base_dir is taken relative to cwd
        let r = resolve_safe("a.txt", Path::new("sub"), Path::new(CWD), false);
        assert_eq!(r, Ok(PathBuf::from("/work/proj/sub/a.txt")));
    }

    #[test]
    fn base_dir_outside_cwd_is_rejected() {
        assert!(resolve_safe("a.txt", Path::new("/elsewhere"), Path::new(CWD), false).is_err());
        assert!(resolve_safe("../x", Path::new("/work/proj/sub"), Path::new(CWD), false).is_ok());
        assert!(
            resolve_safe(
                "../../x",
                Path::new("/work/proj/sub"),
                Path::new(CWD),
                false
            )
            .is_err()
        );
    }

    #[test]
    fn symlinks_are_not_resolved_like_ts() {
        // Lexical check only (matches TS `path.resolve`): a symlink inside
        // cwd pointing outside is NOT detected. Documented in CHANGES.
        let d = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path(), d.path().join("link")).unwrap();
            assert!(resolve_safe("link/x", d.path(), d.path(), false).is_ok());
        }
    }

    #[test]
    fn relative_cwd_is_rejected_when_confining() {
        assert!(resolve_safe("a", Path::new("."), Path::new("."), false).is_err());
    }
}
