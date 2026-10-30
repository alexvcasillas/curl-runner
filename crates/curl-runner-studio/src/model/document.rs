//! An open file: buffer text, saved text, diagnostics and disk-conflict state.

use super::diagnostics::{StudioDiagnostic, diagnose};
use curl_runner_core::lint::edit::fix_edit;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixError {
    /// The diagnostic index is out of range or the diagnostic has no fix.
    NoFix,
    /// The fix can't be expressed as a single text edit, or doesn't apply.
    NotApplicable,
    /// The document is a received copy's: it never changes here.
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    Close,
    Ask,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub rel_path: PathBuf,
    pub text: String,
    pub saved_text: String,
    pub disk_conflict: bool,
    pub diagnostics: Vec<StudioDiagnostic>,
    /// The disk text that conflicted with the dirty buffer.
    disk_text: Option<String>,
    /// `text` changed since `diagnostics` were computed (see `set_buffer`).
    stale: bool,
    /// A received copy's file: the buffer takes no edits, nothing is
    /// saved, and the disk always wins.
    read_only: bool,
}

impl Document {
    pub fn open(root: &Path, rel: &Path) -> io::Result<Document> {
        let text = std::fs::read_to_string(root.join(rel))?;
        let diagnostics = diagnose(&text);
        Ok(Document {
            rel_path: rel.to_path_buf(),
            saved_text: text.clone(),
            text,
            disk_conflict: false,
            diagnostics,
            disk_text: None,
            stale: false,
            read_only: false,
        })
    }

    /// Locks (or unlocks) the document: see [`Document::is_read_only`].
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// A received copy's file: [`set_buffer`](Self::set_buffer) ignores
    /// edits, [`save`](Self::save) and [`apply_fix`](Self::apply_fix) fail,
    /// and a disk change always reloads.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.diagnostics = diagnose(&self.text);
        self.stale = false;
    }

    /// Takes the editor's text without diagnosing it, so typing stays cheap;
    /// `rediagnose` catches up (debounced by the editor).
    pub fn set_buffer(&mut self, text: String) {
        if !self.read_only && text != self.text {
            self.text = text;
            self.stale = true;
        }
    }

    /// Whether `diagnostics` lag behind `text`.
    pub fn is_stale(&self) -> bool {
        self.stale
    }

    /// Re-diagnoses the text if it changed since the last diagnosis.
    pub fn rediagnose(&mut self) {
        if self.stale {
            self.diagnostics = diagnose(&self.text);
            self.stale = false;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn save(&mut self, root: &Path) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "read only: a shared collection's file",
            ));
        }
        std::fs::write(root.join(&self.rel_path), &self.text)?;
        self.saved_text = self.text.clone();
        self.disk_conflict = false;
        self.disk_text = None;
        Ok(())
    }

    /// The file changed on disk. A clean buffer reloads; a dirty one keeps
    /// its text and flags a conflict.
    pub fn on_disk_changed(&mut self, disk: String) {
        if disk == self.saved_text {
            return;
        }
        if self.is_dirty() && !self.read_only {
            self.disk_conflict = true;
            self.disk_text = Some(disk);
        } else {
            self.saved_text = disk.clone();
            self.set_text(disk);
        }
    }

    /// Settle a conflict: keep the buffer (it stays dirty against the new
    /// disk text) or take the disk text.
    /// A read-only document always takes the disk text.
    pub fn resolve_conflict(&mut self, keep_mine: bool) {
        let keep_mine = keep_mine && !self.read_only;
        if let Some(disk) = self.disk_text.take() {
            self.saved_text = disk.clone();
            if !keep_mine {
                self.set_text(disk);
            }
        }
        self.disk_conflict = false;
    }

    /// Apply diagnostic `idx`'s quick fix to the current text. The edit is
    /// computed now, so it always matches the buffer.
    pub fn apply_fix(&mut self, idx: usize) -> Result<(), FixError> {
        if self.read_only {
            return Err(FixError::ReadOnly);
        }
        let issue = self
            .diagnostics
            .get(idx)
            .and_then(|d| d.issue.as_ref())
            .filter(|i| i.fix.is_some())
            .ok_or(FixError::NoFix)?;
        let edited = fix_edit(&self.text, issue)
            .and_then(|e| e.apply(&self.text))
            .ok_or(FixError::NotApplicable)?;
        self.set_text(edited);
        Ok(())
    }

    pub fn close_decision(&self) -> CloseDecision {
        if self.is_dirty() {
            CloseDecision::Ask
        } else {
            CloseDecision::Close
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with(text: &str) -> (tempfile::TempDir, Document) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.yaml"), text).unwrap();
        let doc = Document::open(dir.path(), Path::new("a.yaml")).unwrap();
        (dir, doc)
    }

    const TYPO: &str = "request:\n  url: https://api.example.com\n  mehtod: GET\n";

    #[test]
    fn apply_fix_renames_key_and_clears_issue() {
        let (_d, mut doc) = doc_with(TYPO);
        let idx = doc
            .diagnostics
            .iter()
            .position(|d| d.fix.is_some())
            .unwrap();
        doc.apply_fix(idx).unwrap();
        assert!(doc.text.contains("method: GET"), "{}", doc.text);
        assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
        assert!(doc.is_dirty());
    }

    #[test]
    fn set_buffer_defers_diagnosis_until_rediagnose() {
        let (_d, mut doc) = doc_with("request:\n  url: https://api.example.com\n");
        assert!(doc.diagnostics.is_empty() && !doc.is_stale());
        doc.set_buffer(TYPO.into());
        assert!(doc.is_stale());
        assert!(doc.diagnostics.is_empty(), "set_buffer must not diagnose");
        assert_eq!(doc.text, TYPO);
        doc.rediagnose();
        assert!(!doc.is_stale());
        assert_eq!(doc.diagnostics.len(), 1, "{:?}", doc.diagnostics);
    }

    #[test]
    fn set_buffer_with_the_same_text_stays_current() {
        let (_d, mut doc) = doc_with(TYPO);
        doc.set_buffer(TYPO.into());
        assert!(!doc.is_stale());
        assert_eq!(doc.diagnostics.len(), 1);
    }

    #[test]
    fn apply_fix_without_fix_leaves_text() {
        let (_d, mut doc) = doc_with("hello\n");
        assert_eq!(doc.apply_fix(0), Err(FixError::NoFix));
        assert_eq!(doc.apply_fix(9), Err(FixError::NoFix));
        assert_eq!(doc.text, "hello\n");
    }

    #[test]
    fn disk_change_reloads_clean_buffer() {
        let (_d, mut doc) = doc_with("request:\n  url: https://a.example.com\n");
        doc.on_disk_changed("request:\n  url: https://b.example.com\n".into());
        assert!(doc.text.contains("b.example"));
        assert!(!doc.is_dirty() && !doc.disk_conflict);
    }

    #[test]
    fn disk_change_on_dirty_buffer_sets_conflict_and_keeps_text() {
        let (_d, mut doc) = doc_with("request:\n  url: https://a.example.com\n");
        doc.set_text("request:\n  url: https://mine.example.com\n".into());
        doc.on_disk_changed("request:\n  url: https://b.example.com\n".into());
        assert!(doc.disk_conflict);
        assert!(doc.text.contains("mine"));
        doc.resolve_conflict(false);
        assert!(!doc.disk_conflict && !doc.is_dirty());
        assert!(doc.text.contains("b.example"));
    }

    #[test]
    fn keep_mine_stays_dirty_against_new_disk_text() {
        let (_d, mut doc) = doc_with("request:\n  url: https://a.example.com\n");
        doc.set_text("request:\n  url: https://mine.example.com\n".into());
        doc.on_disk_changed("request:\n  url: https://b.example.com\n".into());
        doc.resolve_conflict(true);
        assert!(!doc.disk_conflict && doc.is_dirty());
        assert!(doc.text.contains("mine"));
    }

    #[test]
    fn close_dirty_document_asks() {
        let (d, mut doc) = doc_with("request:\n  url: https://a.example.com\n");
        assert_eq!(doc.close_decision(), CloseDecision::Close);
        doc.set_text("request:\n  url: https://c.example.com\n".into());
        assert_eq!(doc.close_decision(), CloseDecision::Ask);
        doc.save(d.path()).unwrap();
        assert_eq!(doc.close_decision(), CloseDecision::Close);
    }

    #[test]
    fn read_only_document_takes_no_edit_and_saves_nothing() {
        let (d, mut doc) = doc_with(TYPO);
        doc.set_read_only(true);
        doc.set_buffer("hacked\n".into());
        assert_eq!(doc.text, TYPO);
        let fix = doc
            .diagnostics
            .iter()
            .position(|d| d.fix.is_some())
            .unwrap();
        assert_eq!(doc.apply_fix(fix), Err(FixError::ReadOnly));
        assert_eq!(doc.text, TYPO);
        assert!(doc.save(d.path()).is_err());
        assert_eq!(
            std::fs::read_to_string(d.path().join("a.yaml")).unwrap(),
            TYPO
        );
        // The disk always wins, even against a buffer set from code.
        doc.set_text("mine\n".into());
        doc.on_disk_changed("theirs\n".into());
        assert!(!doc.disk_conflict);
        assert_eq!(doc.text, "theirs\n");
        doc.resolve_conflict(true);
        assert!(!doc.is_dirty());
    }
}
