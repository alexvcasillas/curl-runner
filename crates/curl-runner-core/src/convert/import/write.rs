//! Plan an import's files and write them without overwriting.
//!
//! Every path is built from slugged names only, so nothing in a collection
//! can point a file outside the target folder.

use super::{ImportReport, ImportedVar, Mapped, emit_request, is_line_break, one_line, slug};
use crate::convert::secrets::{unique_path, write_new};
use crate::env::parse_env_content;
use std::collections::HashSet;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Import {
    pub name: String,
    /// The format imported from, as the `.env` header names it.
    pub source: String,
    pub files: Vec<ImportFile>,
    pub variables: Vec<ImportedVar>,
    /// `.env` comment lines per variable, as in [`Mapped::env_comments`].
    pub env_comments: Vec<(String, String)>,
    pub readme: Option<String>,
    pub report: ImportReport,
}

/// One request file. `rel_path` is relative, made of slugged `Normal`
/// components only, and unique within the import.
#[derive(Debug, Clone)]
pub struct ImportFile {
    pub rel_path: PathBuf,
    pub name: String,
    /// Folders and name joined with `/`, as the report's `request` names it.
    pub request: String,
    pub yaml: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WriteOutcome {
    /// Relative to the root.
    pub written: Vec<PathBuf>,
    /// `.env` keys that already had a different value and were left alone.
    pub kept_env: Vec<String>,
    /// `.env` keys whose value lost line breaks or control characters.
    pub cleaned_env: Vec<String>,
    pub readme: bool,
}

/// Plan the files and README of a mapped import. `source` names the format
/// in the files' headers.
pub fn plan_import(name: String, mapped: Mapped, source: &str) -> Import {
    let mut taken = HashSet::new();
    let files = mapped
        .requests
        .iter()
        .map(|r| {
            let mut planned: PathBuf = r.folders.iter().map(|f| slug(f, "folder")).collect();
            planned.push(format!("{}.yaml", slug(&r.name, "request")));
            let rel_path = unique_path(&planned, |p| taken.contains(p));
            taken.insert(rel_path.clone());
            ImportFile {
                rel_path,
                name: r.name.clone(),
                request: r.path(),
                yaml: emit_request(r, source, &name),
            }
        })
        .collect();
    let heading = name.split_whitespace().collect::<Vec<_>>().join(" ");
    Import {
        readme: mapped.description.map(|d| format!("# {heading}\n\n{d}\n")),
        name,
        source: source.to_string(),
        files,
        variables: mapped.variables,
        env_comments: mapped.env_comments,
        report: mapped.report,
    }
}

/// Write `import` under `root/folder`, append new variables to `root/.env`
/// and add `root/folder/README.md` if it is missing. Existing files are never
/// overwritten. `folder` must be relative and free of `..`.
pub fn write_import(
    root: &Path,
    folder: &Path,
    import: &Import,
    secret_note: &str,
) -> io::Result<WriteOutcome> {
    write_import_noting(root, folder, import, &|_| secret_note.to_string())
}

/// [`write_import`] with a note per secret: `secret_note` says where each
/// one's value is set, after `NAME is a secret: `.
pub fn write_import_noting(
    root: &Path,
    folder: &Path,
    import: &Import,
    secret_note: &dyn Fn(&ImportedVar) -> String,
) -> io::Result<WriteOutcome> {
    let folder = safe_folder(folder)?;
    // Checked before the first write, so a bad entry writes nothing.
    if let Some(bad) = import.files.iter().find(|f| !plain_relative(&f.rel_path)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "An imported file must stay inside the import folder: {}",
                bad.rel_path.display()
            ),
        ));
    }
    let mut out = WriteOutcome::default();

    let mut taken = HashSet::new();
    for file in &import.files {
        let rel = unique_path(&folder.join(&file.rel_path), |p| {
            taken.contains(p) || root.join(p).symlink_metadata().is_ok()
        });
        let path = root.join(&rel);
        write_new(&path, file.yaml.as_bytes()).map_err(|e| naming(&path, e))?;
        taken.insert(rel.clone());
        out.written.push(rel);
    }

    let env = root.join(".env");
    (out.kept_env, out.cleaned_env) =
        append_env(&env, import, secret_note).map_err(|e| naming(&env, e))?;

    if let Some(readme) = &import.readme {
        let path = root.join(&folder).join("README.md");
        out.readme = match write_new(&path, readme.as_bytes()) {
            Ok(()) => true,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
            Err(e) => return Err(naming(&path, e)),
        };
    }
    Ok(out)
}

/// `e` with the file it happened on, keeping its kind.
fn naming(path: &Path, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("Couldn't write {}: {e}", path.display()))
}

/// At least one component, and only `Normal` ones.
fn plain_relative(path: &Path) -> bool {
    path.components().next().is_some()
        && path.components().all(|c| matches!(c, Component::Normal(_)))
}

/// `folder` with `.` dropped; an absolute path or `..` is refused.
fn safe_folder(folder: &Path) -> io::Result<PathBuf> {
    let mut safe = PathBuf::new();
    for c in folder.components() {
        match c {
            Component::Normal(s) => safe.push(s),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "The import folder must be inside the project: {}",
                        folder.display()
                    ),
                ));
            }
        }
    }
    Ok(safe)
}

/// Append the keys `.env` doesn't have yet. Returns the keys it already had
/// with a different value, and the keys whose value lost characters that
/// could break its line.
fn append_env(
    path: &Path,
    import: &Import,
    secret_note: &dyn Fn(&ImportedVar) -> String,
) -> io::Result<(Vec<String>, Vec<String>)> {
    let existing = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let present = parse_env_content(&existing);
    let placeholders: HashSet<&str> = existing
        .lines()
        .filter_map(|l| l.trim().strip_prefix('#')?.trim().strip_suffix('='))
        .collect();

    let mut kept = Vec::new();
    let mut cleaned = Vec::new();
    let mut lines = Vec::new();
    for v in &import.variables {
        let notes = import
            .env_comments
            .iter()
            .filter(|(name, _)| *name == v.name)
            .flat_map(|(_, line)| comment(line));
        if let Some(value) = present.get(&v.name) {
            if !v.secret && *value != v.value {
                kept.push(v.name.clone());
            }
        } else if v.secret {
            if !placeholders.contains(v.name.as_str()) {
                lines.extend(notes);
                lines.extend(comment(&format!(
                    "{} is a secret: {}",
                    v.name,
                    secret_note(v)
                )));
                lines.push(format!("# {}=", v.name));
            }
        } else {
            lines.extend(notes);
            let mut value = env_value(&v.name, &v.value);
            if value.chars().any(|c| c.is_control() || is_line_break(c)) {
                value = env_value(&v.name, &one_line(&v.value));
                cleaned.push(v.name.clone());
            }
            lines.push(format!("{}={value}", v.name));
        }
    }
    if lines.is_empty() {
        return Ok((kept, cleaned));
    }

    let name = import.name.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text = String::new();
    if !existing.is_empty() && !existing.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("# Imported from {}: {name}\n", import.source));
    for l in lines {
        text.push_str(&l);
        text.push('\n');
    }
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    io::Write::write_all(&mut f, text.as_bytes())?;
    Ok((kept, cleaned))
}

/// `text` as `.env` comment lines: one `# ` line per line of text, so
/// nothing in it can start a `KEY=VALUE` line.
fn comment(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(is_line_break)
        .filter(|l| !l.is_empty())
        .map(|l| format!("# {l}"))
}

/// `value` as the runner's `.env` loader reads it back: bare when that is
/// safe, else double-quoted, else single-quoted.
fn env_value(name: &str, value: &str) -> String {
    let reads_back = |enc: &str| {
        parse_env_content(&format!("{name}={enc}"))
            .get(name)
            .is_some_and(|v| v == value)
    };
    let needs_quotes =
        value.trim() != value || value.contains([' ', '#', '"', '\'', '\n', '\r', '\t']);
    if !needs_quotes && reads_back(value) {
        return value.to_string();
    }
    let double = format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    );
    let single = format!("'{value}'");
    if reads_back(&double) || !reads_back(&single) {
        double
    } else {
        single
    }
}
