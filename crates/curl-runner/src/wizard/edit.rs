//! Safe saving: merging an edited request back into its document, checking
//! that the text we are about to write reads back as the intended data, and
//! writing atomically. TS rewrote the file from the edited request alone
//! (`{ request: ... }`), which dropped every other request, `global`,
//! `collection`, and every request key the wizard does not ask about.

use super::parser::Loaded;
use super::types::WizardAnswers;
use super::yaml_builder::{build_request_config, same_data, to_yaml_string};
use curl_runner_core::model::Json;
use curl_runner_core::parser;
use serde_json::Map;
use std::io::Write;
use std::path::{Path, PathBuf};

fn same_opt(a: Option<&Json>, b: Option<&Json>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => same_data(a, b),
        _ => false,
    }
}

/// Set `key` to `value` (in place if present, else last), or drop it.
fn set_or_remove(map: &mut Map<String, Json>, key: &str, value: Option<&Json>) {
    match value {
        Some(v) => {
            map.insert(key.to_string(), v.clone());
        }
        None => {
            map.shift_remove(key);
        }
    }
}

impl Loaded {
    /// The document with the edited request updated: only what the wizard
    /// changed. Every key whose built value is the same before and after the
    /// prompts is left exactly as in the file (so keys the wizard cannot
    /// represent, `followRedirects: true`, an unusual `auth`, key order and
    /// all other requests survive); changed keys are replaced in place, new
    /// ones go last, cleared ones go away. `expect` is merged by
    /// `status`/`responseTime`, so `expect.body` and friends stay.
    pub fn apply(&self, edited: &WizardAnswers) -> Json {
        let before = build_request_config(&self.answers);
        let after = build_request_config(edited);
        let (before, after) = (before.as_object().unwrap(), after.as_object().unwrap());

        let mut doc = self.root.clone();
        let Some(Json::Object(req)) = self.loc.get_mut(&mut doc) else {
            return doc;
        };

        let mut keys: Vec<&String> = before.keys().collect();
        keys.extend(after.keys().filter(|k| !before.contains_key(*k)));
        for key in keys {
            if key == "expect" {
                continue;
            }
            let (b, a) = (before.get(key), after.get(key));
            if !same_opt(b, a) {
                set_or_remove(req, key, a);
            }
        }

        let sub = |m: &Map<String, Json>, k: &str| m.get("expect").and_then(|e| e.get(k)).cloned();
        for k in ["status", "responseTime"] {
            let (b, a) = (sub(before, k), sub(after, k));
            if same_opt(b.as_ref(), a.as_ref()) {
                continue;
            }
            match a {
                Some(v) => {
                    if !req.get("expect").is_some_and(Json::is_object) {
                        req.insert("expect".into(), Json::Object(Map::new()));
                    }
                    if let Some(Json::Object(e)) = req.get_mut("expect") {
                        e.insert(k.to_string(), v);
                    }
                }
                None => {
                    if let Some(Json::Object(e)) = req.get_mut("expect") {
                        e.shift_remove(k);
                        if e.is_empty() {
                            req.shift_remove("expect");
                        }
                    }
                }
            }
        }
        doc
    }
}

/// The text for `data`, checked: it must parse back to the same data.
/// Nothing that fails this is ever written.
pub fn render_checked(data: &Json) -> Result<String, String> {
    check_text(to_yaml_string(data), data)
}

/// `text` if it parses back to `data`.
fn check_text(text: String, data: &Json) -> Result<String, String> {
    match parser::parse_raw_stream(&text) {
        Ok(back) if same_data(&back, data) => Ok(text),
        Ok(_) => Err("the YAML to write does not read back as the intended data".into()),
        Err(e) => Err(format!("the YAML to write does not parse: {e}")),
    }
}

/// Write `text` to `path` through a temp file in the same directory and a
/// rename, so a crash or a full disk leaves the old file intact. A symlink is
/// followed (the target is replaced, the link stays) and an existing file's
/// permissions are kept. Missing parent directories are created.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let target: PathBuf = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => path.to_path_buf(),
    };
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    std::fs::create_dir_all(&dir)?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".into());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        drop(f);
        if let Ok(meta) = std::fs::metadata(&target) {
            std::fs::set_permissions(&tmp, meta.permissions())?;
        }
        std::fs::rename(&tmp, &target)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wizard::parser::load_str;
    use crate::wizard::types::{AuthType, BodyType, ExpectStatus};
    use serde_json::json;

    const FILE: &str = "\
global:
  variables:
    BASE: https://api.example.com
request:
  method: POST
  url: ${BASE}/users
  name: Create
  headers:
    X-Trace: abc
  body:
    n: 1
  auth:
    type: oauth2
    clientId: c
  followRedirects: true
  expect:
    status: 201
    body:
      id: \"@\"
  store:
    id: body.id
";

    fn edit(src: &str, f: impl FnOnce(&mut WizardAnswers)) -> (Loaded, Json) {
        let l = load_str(src).unwrap();
        let mut a = l.answers.clone();
        f(&mut a);
        let doc = l.apply(&a);
        (l, doc)
    }

    #[test]
    fn untouched_edit_changes_nothing() {
        let (l, doc) = edit(FILE, |_| {});
        assert_eq!(doc, l.root);
        assert!(l.root["request"]["store"].is_object());
    }

    #[test]
    fn changes_only_what_changed_and_keeps_the_rest() {
        let (l, doc) = edit(FILE, |a| {
            a.method = "PUT".into();
            a.timeout = Some(5000.0);
            a.expect_status = Some(ExpectStatus::One(200.0));
        });
        let r = &doc["request"];
        assert_eq!(r["method"], "PUT");
        assert_eq!(r["timeout"], 5000);
        assert_eq!(r["expect"]["status"], 200);
        // everything else, including keys the wizard has no prompt for
        assert_eq!(r["expect"]["body"], l.root["request"]["expect"]["body"]);
        assert_eq!(r["store"], l.root["request"]["store"]);
        assert_eq!(r["auth"], l.root["request"]["auth"]);
        assert_eq!(r["followRedirects"], true);
        assert_eq!(doc["global"], l.root["global"]);
        let keys: Vec<_> = r.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys[..2], ["method", "url"]); // order kept, `timeout` last-ish
        assert!(keys.contains(&"timeout"));
    }

    #[test]
    fn other_requests_and_collection_survive() {
        let src = "requests:\n  - url: a\n    method: GET\n  - url: b\n    method: POST\nglobal:\n  timeout: 9\n";
        let (_, doc) = edit(src, |a| a.url = "https://a.test".into());
        assert_eq!(doc["requests"][0]["url"], "https://a.test");
        assert_eq!(doc["requests"][1], json!({"url": "b", "method": "POST"}));
        assert_eq!(doc["global"], json!({"timeout": 9}));
        let src = "collection:\n  name: C\n  requests:\n    - url: a\n    - url: b\n";
        let (_, doc) = edit(src, |a| a.method = "DELETE".into());
        assert_eq!(doc["collection"]["requests"][0]["method"], "DELETE");
        assert_eq!(doc["collection"]["requests"][1], json!({"url": "b"}));
        assert_eq!(doc["collection"]["name"], "C");
    }

    #[test]
    fn choosing_no_body_or_no_auth_removes_them() {
        let src = "request:\n  url: u\n  method: POST\n  body:\n    a: 1\n  auth:\n    type: bearer\n    token: t\n  expect:\n    status: 200\n    responseTime: \"< 5\"\n    body: {x: 1}\n";
        let (_, doc) = edit(src, |a| {
            a.body_type = Some(BodyType::None);
            a.auth_type = Some(AuthType::None);
            a.token = None;
            a.expect_status = None;
        });
        let r = doc["request"].as_object().unwrap();
        // the old body/auth are only removed when the wizard built them before
        // and not now: bearer auth was built, so it is gone with `authType: none`
        assert!(!r.contains_key("body"));
        assert!(!r.contains_key("auth"));
        assert_eq!(
            r["expect"],
            json!({"responseTime": "< 5", "body": {"x": 1}})
        );
    }

    #[test]
    fn expect_with_nothing_left_is_removed() {
        let (_, doc) = edit("request:\n  url: u\n  expect:\n    status: 200\n", |a| {
            a.expect_status = None;
        });
        assert!(doc["request"].get("expect").is_none());
    }

    #[test]
    fn clearing_to_default_follow_redirects_drops_the_key() {
        let (_, doc) = edit("request:\n  url: u\n  followRedirects: false\n", |a| {
            a.follow_redirects = Some(true);
        });
        assert!(doc["request"].get("followRedirects").is_none());
    }

    #[test]
    fn rendered_document_reads_back_as_the_document() {
        let (_, doc) = edit(FILE, |a| a.method = "PATCH".into());
        let text = render_checked(&doc).unwrap();
        assert!(same_data(&parser::parse_raw(&text).unwrap(), &doc));
    }

    #[test]
    fn check_text_refuses_text_that_does_not_read_back() {
        let data = json!({"request": {"url": "x"}});
        assert!(check_text("request:\n  url: x".into(), &data).is_ok());
        let wrong = check_text("request:\n  url: y".into(), &data).unwrap_err();
        assert!(wrong.contains("does not read back"), "{wrong}");
        let broken = check_text("request: [".into(), &data).unwrap_err();
        assert!(broken.contains("does not parse"), "{broken}");
        // the multi-document reading of text a rewrite must never produce
        assert!(check_text("a: 1\n---\nb: 2\n".into(), &json!({"a": 1})).is_err());
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a/b/f.yaml");
        write_atomic(&p, "one").unwrap();
        write_atomic(&p, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "two");
        let left: Vec<_> = std::fs::read_dir(p.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["f.yaml"]);
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_keeps_permissions_and_follows_symlinks() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let real = d.path().join("real.yaml");
        std::fs::write(&real, "old").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = d.path().join("link.yaml");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "new").unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
        let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    #[test]
    fn write_atomic_failure_keeps_the_old_file() {
        let d = tempfile::tempdir().unwrap();
        // a directory where the file should be: rename fails, old content stays
        let p = d.path().join("f.yaml");
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("keep"), "x").unwrap();
        assert!(write_atomic(&p, "new").is_err());
        assert!(p.join("keep").exists());
        let names: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["f.yaml"], "temp file cleaned up");
    }
}
