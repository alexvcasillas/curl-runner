//! Watches the collection folder for changes that matter to the tree or an
//! open tab: YAML files and folders, outside hidden directories (`.git`).

use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use notify::event::{CreateKind, RemoveKind};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::path::{Path, PathBuf};

/// Whether a change at `rel` (relative to the root) can change the tree or
/// an open file: not inside a hidden directory, and a folder or a YAML file.
pub fn relevant(rel: &Path, is_dir: bool) -> bool {
    let hidden = rel
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
    !hidden && (is_dir || is_yaml(rel))
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"))
}

/// Indexes of open files (`open`, relative paths) touched by `changed`: the
/// file itself, or a folder above it that moved or went away.
pub fn tabs_to_reread<'a>(
    open: impl IntoIterator<Item = &'a Path>,
    changed: &[PathBuf],
) -> Vec<usize> {
    open.into_iter()
        .enumerate()
        .filter(|(_, rel)| changed.iter().any(|c| rel.starts_with(c)))
        .map(|(i, _)| i)
        .collect()
}

/// Starts watching `root`. Each message is a batch of relevant changed
/// paths, relative to `root`. `None` when the OS watcher can't start.
pub fn start(root: &Path) -> (Option<RecommendedWatcher>, UnboundedReceiver<Vec<PathBuf>>) {
    let (tx, rx) = unbounded();
    // GPUI's test scheduler fails a test when another thread (FSEvents)
    // wakes it, so tests that write files run without the OS watcher.
    if cfg!(test) {
        return (None, rx);
    }
    // FSEvents reports resolved paths (`/private/var/..` for `/var/..`).
    let roots = [
        root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        root.to_path_buf(),
    ];
    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else { return };
        if event.kind.is_access() {
            return;
        }
        let folder_event = matches!(
            event.kind,
            EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder)
        );
        let changed: Vec<PathBuf> = event
            .paths
            .iter()
            .filter_map(|p| {
                let rel = roots.iter().find_map(|r| p.strip_prefix(r).ok())?;
                let is_dir = folder_event || looks_like_dir(p);
                relevant(rel, is_dir).then(|| rel.to_path_buf())
            })
            .collect();
        if !changed.is_empty() {
            tx.unbounded_send(changed).ok();
        }
    })
    .and_then(|mut w| w.watch(root, RecursiveMode::Recursive).map(|()| w));
    match watcher {
        Ok(w) => (Some(w), rx),
        Err(e) => {
            log::warn!("not watching {}: {e}", root.display());
            (None, rx)
        }
    }
}

/// A folder now, or a vanished path without an extension (a removed or
/// renamed-away folder).
fn looks_like_dir(path: &Path) -> bool {
    path.is_dir() || (!path.exists() && path.extension().is_none())
}

#[cfg(test)]
mod tests {
    use super::{relevant, tabs_to_reread};
    use std::path::{Path, PathBuf};

    #[test]
    fn yaml_files_and_folders_are_relevant() {
        assert!(relevant(Path::new("charges/create.yaml"), false));
        assert!(relevant(Path::new("a.YML"), false));
        assert!(relevant(Path::new("charges"), true));
        assert!(relevant(Path::new(""), true));
    }

    #[test]
    fn hidden_dirs_and_other_files_are_not() {
        assert!(!relevant(Path::new(".git/index"), false));
        assert!(!relevant(Path::new(".git/refs"), true));
        assert!(!relevant(Path::new("a/.cache/x.yaml"), false));
        assert!(!relevant(Path::new("README.md"), false));
        assert!(!relevant(Path::new("charges/notes.txt"), false));
    }

    #[test]
    fn rereads_only_changed_tabs_and_tabs_under_changed_folders() {
        let open = [
            Path::new("auth/login.yaml"),
            Path::new("charges/create.yaml"),
            Path::new("smoke.yaml"),
        ];
        let changed = [PathBuf::from("smoke.yaml")];
        assert_eq!(tabs_to_reread(open, &changed), [2]);
        let changed = [PathBuf::from("auth"), PathBuf::from("other.yaml")];
        assert_eq!(tabs_to_reread(open, &changed), [0]);
        assert!(tabs_to_reread(open, &[PathBuf::from("auth2")]).is_empty());
    }
}
