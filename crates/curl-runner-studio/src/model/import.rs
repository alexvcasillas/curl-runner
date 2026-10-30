//! What the Import dialog writes: request files from curl commands, and
//! the YAML files of another folder copied into the collection. Nothing
//! here overwrites a file; a name that is taken gets `-2`, `-3`, ….

use super::templates::{FromCurl, file_name_for, request_from_curl, request_name_for};
use std::collections::HashSet;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// The comment an imported request file starts with.
pub const IMPORTED_HEADER: &str = "# Imported from curl\n";

/// One pasted command and what it converts to.
#[derive(Debug, Clone, PartialEq)]
pub struct CurlImport {
    pub command: String,
    /// The request's name, from its method and path.
    pub name: String,
    pub converted: Result<FromCurl, String>,
}

/// Converts each command in `text` (see
/// [`split_curl_commands`](super::templates::split_curl_commands)).
pub fn convert_commands(text: &str) -> Vec<CurlImport> {
    super::templates::split_curl_commands(text)
        .into_iter()
        .map(|command| {
            let converted = request_from_curl("", &command).and_then(|unnamed| {
                let (method, url) = method_and_url(&unnamed.yaml);
                let name = request_name_for(&method, &url);
                request_from_curl(&name, &command).map(|c| (name, c))
            });
            match converted {
                Ok((name, mut c)) => {
                    c.yaml.insert_str(0, IMPORTED_HEADER);
                    CurlImport {
                        command,
                        name,
                        converted: Ok(c),
                    }
                }
                Err(e) => CurlImport {
                    command,
                    name: String::new(),
                    converted: Err(e),
                },
            }
        })
        .collect()
}

/// `request.method` (GET when absent) and `request.url` of a converted file.
fn method_and_url(yaml: &str) -> (String, String) {
    let field = |key: &str| {
        yaml.lines().find_map(|l| {
            l.strip_prefix(&format!("  {key}: "))
                .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
        })
    };
    (
        field("method").unwrap_or_else(|| "GET".into()),
        field("url").unwrap_or_default(),
    )
}

/// `path` if `taken` says it is free, else `stem-2.ext`, `stem-3.ext`, ….
pub fn unique_path(path: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf {
    if !taken(path) {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| path.with_file_name(format!("{stem}-{n}{ext}")))
        .find(|p| !taken(p))
        .expect("an unbounded range finds a free name")
}

/// Where each convertible command's file goes, relative to `root`: in
/// `folder`, named after the request, never over an existing file or
/// another command's (`root: None`: nothing on disk yet). `None` for a
/// command that failed to convert.
pub fn curl_targets(
    root: Option<&Path>,
    folder: &Path,
    imports: &[CurlImport],
) -> Vec<Option<PathBuf>> {
    let mut planned = HashSet::new();
    imports
        .iter()
        .map(|i| {
            i.converted.as_ref().ok()?;
            let rel = unique_path(&folder.join(file_name_for(&i.name)), |p| {
                planned.contains(p) || on_disk(root, p)
            });
            planned.insert(rel.clone());
            Some(rel)
        })
        .collect()
}

/// `rel` exists under `root` (`None`: no folder yet, nothing to clash with).
fn on_disk(root: Option<&Path>, rel: &Path) -> bool {
    root.is_some_and(|r| r.join(rel).symlink_metadata().is_ok())
}

/// One file the Folder of YAML import copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyPlan {
    /// The file, absolute.
    pub from: PathBuf,
    /// Its path inside the picked folder.
    pub rel: PathBuf,
    /// Where it goes, relative to the collection root.
    pub to: PathBuf,
}

impl CopyPlan {
    /// Copied under another name, because the name was taken.
    pub fn renamed(&self, folder: &Path) -> bool {
        self.to != folder.join(&self.rel)
    }
}

/// The `*.yaml` / `*.yml` files under `source`, depth first in name order,
/// skipping hidden entries, symlinks and a top-level `curl-runner.yaml`, and
/// where each goes under `root/folder`, keeping subfolders.
pub fn plan_yaml_copy(
    source: &Path,
    root: Option<&Path>,
    folder: &Path,
) -> io::Result<Vec<CopyPlan>> {
    let mut files = Vec::new();
    collect_yaml(source, Path::new(""), &mut files)?;
    let mut planned = HashSet::new();
    Ok(files
        .into_iter()
        .map(|rel| {
            let to = unique_path(&folder.join(&rel), |p| {
                planned.contains(p) || on_disk(root, p)
            });
            planned.insert(to.clone());
            CopyPlan {
                from: source.join(&rel),
                rel,
                to,
            }
        })
        .collect())
}

fn collect_yaml(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        let path = rel.join(&name);
        if kind.is_dir() {
            collect_yaml(&entry.path(), &path, out)?;
        } else if kind.is_file()
            && (name_str.ends_with(".yaml") || name_str.ends_with(".yml"))
            && !(rel.as_os_str().is_empty()
                && matches!(name_str.as_ref(), "curl-runner.yaml" | "curl-runner.yml"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Copies the planned files into `root`, never over an existing file.
/// Returns how many were copied; stops at the first failure.
pub fn copy_planned(root: &Path, plan: &[CopyPlan]) -> io::Result<usize> {
    for (n, file) in plan.iter().enumerate() {
        let text = std::fs::read(&file.from)?;
        write_new(&root.join(&file.to), &text).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("{} (after {n} copied): {e}", file.to.display()),
            )
        })?;
    }
    Ok(plan.len())
}

/// Writes a new file, creating its folders; an existing one fails with
/// `AlreadyExists`.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::diagnostics::diagnose;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn copy_folder_of_yaml_skips_hidden_and_non_yaml() {
        let src = tempfile::tempdir().unwrap();
        let s = src.path();
        for f in [
            "curl-runner.yaml",
            "a.yaml",
            "b.yml",
            "notes.md",
            ".hidden.yaml",
            ".git/config.yaml",
            "nested/c.yaml",
            "nested/curl-runner.yaml",
            "nested/.d.yaml",
        ] {
            write(s, f, "request:\n  url: https://api.example.com\n");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(s.join("a.yaml"), s.join("link.yaml")).unwrap();

        let dst = tempfile::tempdir().unwrap();
        let d = dst.path();
        write(d, "imported/a.yaml", "kept\n");
        let plan = plan_yaml_copy(s, Some(d), Path::new("imported")).unwrap();
        let summary: Vec<(String, String)> = plan
            .iter()
            .map(|p| (p.rel.display().to_string(), p.to.display().to_string()))
            .collect();
        assert_eq!(
            summary,
            [
                ("a.yaml".into(), "imported/a-2.yaml".into()),
                ("b.yml".into(), "imported/b.yml".into()),
                ("nested/c.yaml".into(), "imported/nested/c.yaml".into()),
                (
                    "nested/curl-runner.yaml".into(),
                    "imported/nested/curl-runner.yaml".into()
                ),
            ]
        );
        assert!(plan[0].renamed(Path::new("imported")));
        assert!(!plan[1].renamed(Path::new("imported")));

        assert_eq!(copy_planned(d, &plan).unwrap(), 4);
        assert_eq!(
            std::fs::read_to_string(d.join("imported/a.yaml")).unwrap(),
            "kept\n"
        );
        assert!(d.join("imported/a-2.yaml").is_file());
        assert!(d.join("imported/nested/c.yaml").is_file());
        // Copying the same plan again never overwrites.
        let again = copy_planned(d, &plan).unwrap_err();
        assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn unique_paths_count_up() {
        let taken = [PathBuf::from("x/a.yaml"), PathBuf::from("x/a-2.yaml")];
        let free = |p: &Path| taken.iter().any(|t| t == p);
        assert_eq!(
            unique_path(Path::new("x/a.yaml"), free),
            Path::new("x/a-3.yaml")
        );
        assert_eq!(
            unique_path(Path::new("x/b.yaml"), free),
            Path::new("x/b.yaml")
        );
    }

    #[test]
    fn commands_convert_with_derived_names_and_unique_files() {
        let text = "curl -X POST https://api.example.com/v1/customers -d name=Ada\n\n\
                    curl https://api.example.com/v1/customers/cus_8f2k1Abc\n\
                    curl -X POST https://api.example.com/v1/customers -d name=Bob\n\n\
                    hello world";
        let imports = convert_commands(text);
        assert_eq!(imports.len(), 4);
        let names: Vec<&str> = imports.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["create-customer", "get-customer", "create-customer", ""]
        );
        let first = imports[0].converted.as_ref().unwrap();
        assert!(first.yaml.starts_with(IMPORTED_HEADER), "{}", first.yaml);
        assert!(
            first.yaml.contains("name: create-customer"),
            "{}",
            first.yaml
        );
        assert_eq!(diagnose(&first.yaml), []);
        assert!(imports[3].converted.is_err());

        let root = tempfile::tempdir().unwrap();
        write(root.path(), "customers/get-customer.yaml", "x\n");
        let targets = curl_targets(Some(root.path()), Path::new("customers"), &imports);
        assert_eq!(
            targets,
            [
                Some(PathBuf::from("customers/create-customer.yaml")),
                Some(PathBuf::from("customers/get-customer-2.yaml")),
                Some(PathBuf::from("customers/create-customer-2.yaml")),
                None,
            ]
        );
    }
}
