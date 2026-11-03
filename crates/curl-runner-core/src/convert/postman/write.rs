//! Plan a converted collection's files and write them without overwriting.
//!
//! Every path is built from slugged names only, so nothing in a collection
//! can point a file outside the target folder.

use super::{ImportReport, ImportedVar, emit_request, map_collection, parse_collection};
use crate::convert::ConvertError;
use crate::convert::secrets::{unique_path, write_new};
use crate::env::parse_env_content;
use std::collections::HashSet;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PostmanImport {
    pub name: String,
    pub files: Vec<PostmanFile>,
    pub variables: Vec<ImportedVar>,
    pub readme: Option<String>,
    pub report: ImportReport,
}

/// One request file. `rel_path` is relative, made of slugged `Normal`
/// components only, and unique within the import.
#[derive(Debug, Clone)]
pub struct PostmanFile {
    pub rel_path: PathBuf,
    pub name: String,
    pub yaml: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WriteOutcome {
    /// Relative to the root.
    pub written: Vec<PathBuf>,
    /// `.env` keys that already had a different value and were left alone.
    pub kept_env: Vec<String>,
    pub readme: bool,
}

/// Parse, map and emit a Postman collection.
pub fn convert_postman(json: &str) -> Result<PostmanImport, ConvertError> {
    let collection = parse_collection(json)?;
    let name = collection.info.name.clone();
    let mapped = map_collection(&collection);
    let mut taken = HashSet::new();
    let files = mapped
        .requests
        .iter()
        .map(|r| {
            let mut planned: PathBuf = r.folders.iter().map(|f| slug(f)).collect();
            planned.push(format!("{}.yaml", slug(&r.name)));
            let rel_path = unique_path(&planned, |p| taken.contains(p));
            taken.insert(rel_path.clone());
            PostmanFile {
                rel_path,
                name: r.name.clone(),
                yaml: emit_request(r, &name),
            }
        })
        .collect();
    let heading = name.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(PostmanImport {
        readme: mapped.description.map(|d| format!("# {heading}\n\n{d}\n")),
        name,
        files,
        variables: mapped.variables,
        report: mapped.report,
    })
}

/// Lowercase ASCII letters and digits; any run of other characters becomes
/// `-`, trimmed; empty falls back to `request`. `.`, `/` and `\` are among
/// the "other" characters, so a slug is always a single plain component.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "request".into()
    } else {
        out.to_string()
    }
}

/// Write `import` under `root/folder`, append new variables to `root/.env`
/// and add `root/folder/README.md` if it is missing. Existing files are never
/// overwritten. `folder` must be relative and free of `..`.
pub fn write_postman(
    root: &Path,
    folder: &Path,
    import: &PostmanImport,
    secret_note: &str,
) -> io::Result<WriteOutcome> {
    let folder = safe_folder(folder)?;
    let mut out = WriteOutcome::default();

    for file in &import.files {
        let rel = unique_path(&folder.join(&file.rel_path), |p| {
            out.written.iter().any(|w| w == p) || root.join(p).symlink_metadata().is_ok()
        });
        write_new(&root.join(&rel), file.yaml.as_bytes())?;
        out.written.push(rel);
    }

    out.kept_env = append_env(&root.join(".env"), import, secret_note)?;

    if let Some(readme) = &import.readme {
        out.readme = match write_new(&root.join(&folder).join("README.md"), readme.as_bytes()) {
            Ok(()) => true,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
            Err(e) => return Err(e),
        };
    }
    Ok(out)
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

/// Append the keys `.env` doesn't have yet; returns the keys it already had
/// with a different value.
fn append_env(path: &Path, import: &PostmanImport, secret_note: &str) -> io::Result<Vec<String>> {
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
    let mut lines = Vec::new();
    for v in &import.variables {
        if let Some(value) = present.get(&v.name) {
            if !v.secret && *value != v.value {
                kept.push(v.name.clone());
            }
        } else if v.secret {
            if !placeholders.contains(v.name.as_str()) {
                lines.push(format!("# {} is a secret: {secret_note}", v.name));
                lines.push(format!("# {}=", v.name));
            }
        } else {
            lines.push(format!("{}={}", v.name, env_value(&v.name, &v.value)));
        }
    }
    if lines.is_empty() {
        return Ok(kept);
    }

    let name = import.name.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text = String::new();
    if !existing.is_empty() && !existing.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("# Imported from Postman: {name}\n"));
    for l in lines {
        text.push_str(&l);
        text.push('\n');
    }
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    io::Write::write_all(&mut f, text.as_bytes())?;
    Ok(kept)
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
