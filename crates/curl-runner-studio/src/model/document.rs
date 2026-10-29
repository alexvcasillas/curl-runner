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
        })
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.diagnostics = diagnose(&self.text);
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn save(&mut self, root: &Path) -> io::Result<()> {
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
        if self.is_dirty() {
            self.disk_conflict = true;
            self.disk_text = Some(disk);
        } else {
            self.saved_text = disk.clone();
            self.set_text(disk);
        }
    }

    /// Settle a conflict: keep the buffer (it stays dirty against the new
    /// disk text) or take the disk text.
    pub fn resolve_conflict(&mut self, keep_mine: bool) {
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
}
