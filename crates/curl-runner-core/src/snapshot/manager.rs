//! Snapshot files: path, load/save, create, hash and compare/update
//! (`snapshot/snapshot-manager.ts`). Files stay byte-compatible with TS:
//! `JSON.stringify(data, null, 2)`, no trailing newline.

use super::differ::SnapshotDiffer;
use super::json::{js_stringify, js_stringify_pretty};
use crate::jsfile::{headers_field, read_json_file, split_path, stem, text_field, untyped_entry};
use crate::model::{
    ExecutionResult, GlobalSnapshotConfig, HeaderValue, Json, RequestConfig, Snapshot,
    SnapshotCompareResult, SnapshotConfig, SnapshotDiff, SnapshotSetting,
};
use indexmap::IndexMap;
use md5::{Digest, Md5};
use std::path::PathBuf;

const SNAPSHOT_VERSION: u64 = 1;
pub const DEFAULT_SNAPSHOT_DIR: &str = "__snapshots__";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateMode {
    #[default]
    None,
    All,
    Failing,
}

impl UpdateMode {
    /// `none|all|failing`; anything else behaves like `none` (as in TS).
    pub fn parse(s: Option<&str>) -> Self {
        match s {
            Some("all") => Self::All,
            Some("failing") => Self::Failing,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotManager {
    dir: String,
    update_mode: UpdateMode,
    ci: bool,
    cwd: PathBuf,
}

impl SnapshotManager {
    /// An empty `dir` means the default (`__snapshots__`, TS `dir || DEFAULT`).
    pub fn new(dir: &str, update_mode: UpdateMode, ci: bool) -> Self {
        Self {
            dir: if dir.is_empty() {
                DEFAULT_SNAPSHOT_DIR.into()
            } else {
                dir.into()
            },
            update_mode,
            ci,
            cwd: PathBuf::new(),
        }
    }

    pub fn from_global(g: Option<&GlobalSnapshotConfig>) -> Self {
        Self::new(
            g.and_then(|g| g.dir.as_deref()).unwrap_or(""),
            UpdateMode::parse(g.and_then(|g| g.update_mode.as_deref())),
            g.and_then(|g| g.ci) == Some(true),
        )
    }

    /// Base for relative YAML paths (the process cwd in TS).
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// `<dirname(yaml)>/<dir>/<basename without extension>.snap.json`, joined
    /// and normalized like `path.join`.
    pub fn snapshot_path(&self, yaml_path: &str) -> String {
        let (dir, file) = split_path(yaml_path);
        let stem = stem(file);
        path_join(&[dir, &self.dir, &format!("{stem}.snap.json")])
    }

    fn fs_path(&self, yaml_path: &str) -> PathBuf {
        self.cwd.join(self.snapshot_path(yaml_path))
    }

    /// The raw snapshot file: `Ok(None)` when it does not exist, `Err` when
    /// it exists but cannot be read, parsed (e.g. a lone-surrogate escape
    /// `serde_json` rejects) or is not shaped `{ snapshots: {...} }`. Callers
    /// must not write over an `Err` file: that would drop every entry.
    fn read_file(&self, yaml_path: &str) -> std::io::Result<Option<Json>> {
        read_json_file(&self.fs_path(yaml_path), "snapshot", "snapshots")
    }

    /// Loads the snapshot file for a YAML file (`None` when missing or
    /// unusable). Entries are read leniently, like the untyped JS objects TS
    /// uses; falsy entries are skipped.
    pub fn load(&self, yaml_path: &str) -> Option<crate::model::SnapshotFile> {
        let raw = self.read_file(yaml_path).ok()??;
        let snapshots = raw
            .get("snapshots")?
            .as_object()?
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), parse_snapshot(v)?)))
            .collect();
        Some(crate::model::SnapshotFile {
            version: raw
                .get("version")
                .and_then(Json::as_u64)
                .unwrap_or(SNAPSHOT_VERSION),
            snapshots,
        })
    }

    /// Writes the file pretty-printed with 2 spaces and no trailing newline.
    fn save_raw(&self, yaml_path: &str, data: &Json) -> std::io::Result<()> {
        let path = self.fs_path(yaml_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, js_stringify_pretty(data, 2))
    }

    pub fn save(&self, yaml_path: &str, data: &crate::model::SnapshotFile) -> std::io::Result<()> {
        let v = serde_json::to_value(data).map_err(std::io::Error::other)?;
        self.save_raw(yaml_path, &v)
    }

    /// One snapshot by name (`None` when missing or the file is unusable).
    pub fn get(&self, yaml_path: &str, name: &str) -> Option<Snapshot> {
        entry_of(&self.read_file(yaml_path).ok()??, name)
    }

    /// Snapshot of a result: only the `include`d parts (default `body`).
    pub fn create_snapshot(&self, result: &ExecutionResult, config: &SnapshotConfig) -> Snapshot {
        let default = vec!["body".to_string()];
        let include = config.include.as_ref().unwrap_or(&default);
        let has = |p: &str| include.iter().any(|i| i == p);
        let mut s = Snapshot {
            updated_at: now_iso(),
            ..Default::default()
        };
        if has("status") {
            s.status = result.status.map(Json::from);
        }
        if has("headers")
            && let Some(h) = &result.headers
        {
            s.headers = Some(normalize_headers(h));
        }
        if has("body") {
            s.body = result.body.clone();
        }
        s.hash = snapshot_hash(&s);
        s
    }

    /// First 8 hex digits of md5 over `JSON.stringify(content)`.
    pub fn hash(&self, content: &Json) -> String {
        hash_json(content)
    }

    /// Compares a result with the stored snapshot, creating or updating the
    /// file per the CI and update modes. `Err` only when the file cannot be
    /// written.
    pub fn compare(
        &self,
        req: &RequestConfig,
        result: &ExecutionResult,
        config: &SnapshotConfig,
    ) -> std::io::Result<SnapshotCompareResult> {
        let yaml_path = req.source_file.as_deref().unwrap_or("");
        let request_name = non_empty(req.name.as_deref()).unwrap_or("Request");
        self.compare_and_update(yaml_path, request_name, result, config)
    }

    pub fn compare_and_update(
        &self,
        yaml_path: &str,
        request_name: &str,
        result: &ExecutionResult,
        config: &SnapshotConfig,
    ) -> std::io::Result<SnapshotCompareResult> {
        let name = non_empty(config.name.as_deref()).unwrap_or(request_name);
        let file = self.read_file(yaml_path)?;
        let existing = file.as_ref().and_then(|f| entry_of(f, name));
        let new = self.create_snapshot(result, config);

        let Some(existing) = existing else {
            if self.ci {
                return Ok(SnapshotCompareResult {
                    matches: false,
                    is_new: true,
                    updated: false,
                    differences: vec![SnapshotDiff {
                        path: String::new(),
                        expected: Some(Json::String("snapshot".into())),
                        received: Some(Json::String("none".into())),
                        diff_type: "removed".into(),
                    }],
                });
            }
            self.update_snapshot(yaml_path, name, &new)?;
            return Ok(SnapshotCompareResult {
                matches: true,
                is_new: true,
                updated: true,
                differences: Vec::new(),
            });
        };

        let diff = SnapshotDiffer::new(config).compare(&existing, &new);
        if diff.matches {
            return Ok(SnapshotCompareResult {
                matches: true,
                is_new: false,
                updated: false,
                differences: Vec::new(),
            });
        }
        if matches!(self.update_mode, UpdateMode::All | UpdateMode::Failing) {
            self.update_snapshot(yaml_path, name, &new)?;
            return Ok(SnapshotCompareResult {
                matches: true,
                is_new: false,
                updated: true,
                differences: diff.differences,
            });
        }
        Ok(SnapshotCompareResult {
            matches: false,
            is_new: false,
            updated: false,
            differences: diff.differences,
        })
    }

    fn update_snapshot(&self, yaml_path: &str, name: &str, s: &Snapshot) -> std::io::Result<()> {
        let mut file = self
            .read_file(yaml_path)?
            .unwrap_or_else(|| serde_json::json!({ "version": SNAPSHOT_VERSION, "snapshots": {} }));
        let value = serde_json::to_value(s).map_err(std::io::Error::other)?;
        if let Some(Json::Object(snaps)) = file.get_mut("snapshots") {
            snaps.insert(name.to_string(), value);
        }
        self.save_raw(yaml_path, &file)
    }

    /// Request config merged with the global one; `None` when snapshots are
    /// not enabled for the request.
    pub fn merge_config(
        global: Option<&GlobalSnapshotConfig>,
        request: Option<&SnapshotSetting>,
    ) -> Option<SnapshotConfig> {
        let body = || vec!["body".to_string()];
        let g_enabled = global.and_then(|g| g.enabled) == Some(true);
        let g_include = global.and_then(|g| g.include.clone());
        let g_exclude = global.and_then(|g| g.exclude.clone());
        let g_match = global.and_then(|g| g.match_rules.clone());

        let from_global = || SnapshotConfig {
            enabled: Some(true),
            name: None,
            include: Some(g_include.clone().unwrap_or_else(body)),
            exclude: Some(g_exclude.clone().unwrap_or_default()),
            match_rules: Some(g_match.clone().unwrap_or_default()),
        };

        match request {
            None | Some(SnapshotSetting::Bool(false)) if !g_enabled => None,
            Some(SnapshotSetting::Bool(true)) => Some(from_global()),
            Some(SnapshotSetting::Config(c)) if c.enabled != Some(false) => {
                let mut exclude = g_exclude.clone().unwrap_or_default();
                exclude.extend(c.exclude.iter().flatten().cloned());
                let mut rules = g_match.clone().unwrap_or_default();
                for (k, v) in c.match_rules.iter().flatten() {
                    rules.insert(k.clone(), v.clone());
                }
                Some(SnapshotConfig {
                    enabled: Some(true),
                    name: c.name.clone(),
                    include: Some(c.include.clone().or(g_include).unwrap_or_else(body)),
                    exclude: Some(exclude),
                    match_rules: Some(rules),
                })
            }
            None if g_enabled => Some(from_global()),
            _ => None,
        }
    }
}

/// The entry `name` of a loaded file, or `None` when absent or falsy.
fn entry_of(file: &Json, name: &str) -> Option<Snapshot> {
    parse_snapshot(file.get("snapshots")?.get(name)?)
}

/// Reads a stored entry like TS reads an untyped object: missing `hash` or
/// `updatedAt` are empty, `status`/header values keep whatever type they have
/// (so `"200"` differs from `200`), a non-object `headers` is ignored. Falsy
/// entries (`null`, `false`, `0`, `""`) count as missing (`|| null`).
fn parse_snapshot(v: &Json) -> Option<Snapshot> {
    let o = untyped_entry(v)?;
    Some(Snapshot {
        hash: text_field(o, "hash"),
        updated_at: text_field(o, "updatedAt"),
        status: o.get("status").cloned(),
        headers: headers_field(o),
        body: o.get("body").cloned(),
    })
}

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

/// JS `new Date().toISOString()`.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub(crate) fn hash_json(v: &Json) -> String {
    let digest = Md5::digest(js_stringify(v).as_bytes());
    digest.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Hash as TS computes it: over the snapshot object while `hash` is still
/// `""` (so it includes `updatedAt`).
pub fn snapshot_hash(s: &Snapshot) -> String {
    let unhashed = Snapshot {
        hash: String::new(),
        ..s.clone()
    };
    serde_json::to_value(&unhashed)
        .map(|v| hash_json(&v))
        .unwrap_or_default()
}

/// Lowercased keys in sorted order; repeated values joined with `", "`.
pub(crate) fn normalize_headers(h: &IndexMap<String, HeaderValue>) -> IndexMap<String, Json> {
    let mut keys: Vec<&String> = h.keys().collect();
    keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    let mut out = IndexMap::new();
    for k in keys {
        let v = match &h[k] {
            HeaderValue::One(s) => s.clone(),
            HeaderValue::Many(v) => v.join(", "),
        };
        out.insert(k.to_lowercase(), Json::String(v));
    }
    out
}

/// `path.posix.join` (join with `/`, then normalize).
pub(crate) fn path_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    if joined.is_empty() {
        return ".".into();
    }
    let abs = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|l| *l != "..") {
                    out.pop();
                } else if !abs {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let body = out.join("/");
    match (abs, body.is_empty()) {
        (true, _) => format!("/{body}"),
        (false, true) => ".".into(),
        (false, false) => body,
    }
}

/// Body with `body.`-prefixed exclusions removed (exact, `*.x` and `[*]`
/// patterns).
pub fn filter_snapshot_body(body: &Json, exclude: &[String]) -> Json {
    if !matches!(body, Json::Object(_) | Json::Array(_)) {
        return body.clone();
    }
    let excludes: Vec<&str> = exclude
        .iter()
        .filter_map(|p| p.strip_prefix("body."))
        .collect();
    if excludes.is_empty() {
        return body.clone();
    }
    filter_value(body, &excludes, "")
}

fn filter_value(v: &Json, excludes: &[&str], current: &str) -> Json {
    match v {
        Json::Array(a) => Json::Array(
            a.iter()
                .enumerate()
                .map(|(i, item)| {
                    let p = if current.is_empty() {
                        format!("[{i}]")
                    } else {
                        format!("{current}[{i}]")
                    };
                    filter_value(item, excludes, &p)
                })
                .collect(),
        ),
        Json::Object(m) => {
            let mut out = serde_json::Map::new();
            for (k, val) in super::json::js_entries(m) {
                let full = if current.is_empty() {
                    k.clone()
                } else {
                    format!("{current}.{k}")
                };
                if !excludes.iter().any(|p| path_excluded(p, &full)) {
                    out.insert(k.clone(), filter_value(val, excludes, &full));
                }
            }
            Json::Object(out)
        }
        other => other.clone(),
    }
}

fn path_excluded(pattern: &str, full: &str) -> bool {
    if pattern == full {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return full.ends_with(&format!(".{suffix}")) || full == suffix;
    }
    if pattern.contains("[*]") {
        let re = format!("^{}$", pattern.replace("[*]", "\\[\\d+\\]"));
        return crate::jsval::regex_test(&re, full, false) == Some(true);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn fixture(name: &str) -> String {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ts-snapshots")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    fn req(name: Option<&str>, src: &str) -> RequestConfig {
        let mut v = json!({ "url": "https://api.example.com", "sourceFile": src });
        if let Some(n) = name {
            v["name"] = json!(n);
        }
        serde_json::from_value(v).unwrap()
    }

    fn mock_result() -> ExecutionResult {
        let mut r = ExecutionResult::new(req(None, "api.yaml"), true);
        r.status = Some(200);
        let mut h = IndexMap::new();
        h.insert(
            "X-Request-Id".to_string(),
            HeaderValue::One("abc123".into()),
        );
        h.insert(
            "Content-Type".to_string(),
            HeaderValue::One("application/json".into()),
        );
        r.headers = Some(h);
        r.body = Some(json!({"id": 1, "name": "Test", "timestamp": "2024-01-01"}));
        r
    }

    fn cfg(v: Json) -> SnapshotConfig {
        serde_json::from_value(v).unwrap()
    }

    fn mgr() -> SnapshotManager {
        SnapshotManager::new("", UpdateMode::None, false)
    }

    // ---- getSnapshotPath

    #[test]
    fn generates_correct_snapshot_path() {
        assert_eq!(
            mgr().snapshot_path("tests/api.yaml"),
            "tests/__snapshots__/api.snap.json"
        );
    }

    #[test]
    fn uses_custom_directory() {
        let m = SnapshotManager::new(".snapshots", UpdateMode::None, false);
        assert_eq!(m.snapshot_path("api.yaml"), ".snapshots/api.snap.json");
    }

    #[test]
    fn handles_nested_paths() {
        assert_eq!(
            mgr().snapshot_path("tests/integration/users.yaml"),
            "tests/integration/__snapshots__/users.snap.json"
        );
    }

    #[test]
    fn snapshot_path_matches_path_join_quirks() {
        let abs_dir = SnapshotManager::new("/abs/snaps", UpdateMode::None, false);
        // path.join never resets on an absolute segment
        assert_eq!(abs_dir.snapshot_path("t/a.yaml"), "t/abs/snaps/a.snap.json");
        let up = SnapshotManager::new("../snaps", UpdateMode::None, false);
        assert_eq!(up.snapshot_path("t/a.yaml"), "snaps/a.snap.json");
        assert_eq!(up.snapshot_path("a.yaml"), "../snaps/a.snap.json");
        assert_eq!(
            mgr().snapshot_path("/x/y/a.b.yml"),
            "/x/y/__snapshots__/a.b.snap.json"
        );
        assert_eq!(mgr().snapshot_path("./a.yaml"), "__snapshots__/a.snap.json");
        assert_eq!(
            mgr().snapshot_path("noext"),
            "__snapshots__/noext.snap.json"
        );
    }

    // ---- createSnapshot

    #[test]
    fn creates_snapshot_with_body_only_by_default() {
        let s = mgr().create_snapshot(&mock_result(), &cfg(json!({"include": ["body"]})));
        assert_eq!(s.body, mock_result().body);
        assert_eq!(s.status, None);
        assert_eq!(s.headers, None);
        assert_eq!(s.hash.len(), 8);
        assert!(!s.updated_at.is_empty());
    }

    #[test]
    fn creates_snapshot_with_status() {
        let s = mgr().create_snapshot(&mock_result(), &cfg(json!({"include": ["status"]})));
        assert_eq!(s.status, Some(json!(200)));
        assert_eq!(s.body, None);
    }

    #[test]
    fn creates_snapshot_with_normalized_headers() {
        let s = mgr().create_snapshot(&mock_result(), &cfg(json!({"include": ["headers"]})));
        let h = s.headers.unwrap();
        assert_eq!(
            serde_json::to_string(&h).unwrap(),
            r#"{"content-type":"application/json","x-request-id":"abc123"}"#
        );
    }

    #[test]
    fn creates_snapshot_with_all_components() {
        let s = mgr().create_snapshot(
            &mock_result(),
            &cfg(json!({"include": ["status", "headers", "body"]})),
        );
        assert_eq!(s.status, Some(json!(200)));
        assert!(s.headers.is_some());
        assert!(s.body.is_some());
    }

    #[test]
    fn default_include_is_body() {
        let s = mgr().create_snapshot(&mock_result(), &cfg(json!({})));
        assert!(s.body.is_some() && s.status.is_none());
    }

    #[test]
    fn multi_value_headers_are_joined() {
        let mut r = mock_result();
        r.headers = Some(
            [(
                "Set-Cookie".to_string(),
                HeaderValue::Many(vec!["a=1".into(), "b=2".into()]),
            )]
            .into_iter()
            .collect(),
        );
        let s = mgr().create_snapshot(&r, &cfg(json!({"include": ["headers"]})));
        assert_eq!(s.headers.unwrap()["set-cookie"], json!("a=1, b=2"));
    }

    #[test]
    fn updated_at_is_js_iso_format() {
        let s = mgr().create_snapshot(&mock_result(), &cfg(json!({})));
        let b = s.updated_at.as_bytes();
        assert_eq!(s.updated_at.len(), 24, "{}", s.updated_at);
        assert_eq!(
            (b[4], b[7], b[10], b[13], b[16], b[19], b[23]),
            (b'-', b'-', b'T', b':', b':', b'.', b'Z')
        );
    }

    // ---- hash

    #[test]
    fn generates_consistent_hashes() {
        let c = json!({"id": 1, "name": "test"});
        assert_eq!(mgr().hash(&c), mgr().hash(&c));
    }

    #[test]
    fn generates_different_hashes_for_different_content() {
        assert_ne!(mgr().hash(&json!({"id": 1})), mgr().hash(&json!({"id": 2})));
    }

    #[test]
    fn hash_is_first_8_hex_of_md5_of_js_json() {
        assert_eq!(mgr().hash(&json!({"id": 1})), "d2ce28b9"); // md5 of {"id":1}
        assert_eq!(mgr().hash(&json!(1.0)), mgr().hash(&json!(1)));
    }

    // ---- mergeConfig

    fn global(v: Json) -> GlobalSnapshotConfig {
        serde_json::from_value(v).unwrap()
    }
    fn setting(v: Json) -> SnapshotSetting {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn merge_returns_none_if_not_enabled() {
        assert_eq!(
            SnapshotManager::merge_config(Some(&global(json!({}))), None),
            None
        );
        assert_eq!(SnapshotManager::merge_config(None, None), None);
    }

    #[test]
    fn merge_handles_boolean_true() {
        let c =
            SnapshotManager::merge_config(Some(&global(json!({}))), Some(&setting(json!(true))));
        assert_eq!(
            c,
            Some(cfg(
                json!({"enabled": true, "include": ["body"], "exclude": [], "match": {}})
            ))
        );
    }

    #[test]
    fn merge_combines_global_and_request_excludes() {
        let c = SnapshotManager::merge_config(
            Some(&global(json!({"exclude": ["*.timestamp"]}))),
            Some(&setting(json!({"enabled": true, "exclude": ["body.id"]}))),
        );
        assert_eq!(
            c.unwrap().exclude,
            Some(vec!["*.timestamp".into(), "body.id".into()])
        );
    }

    #[test]
    fn merge_overrides_include_from_request() {
        let c = SnapshotManager::merge_config(
            Some(&global(json!({"include": ["body"]}))),
            Some(&setting(
                json!({"enabled": true, "include": ["status", "body"]}),
            )),
        );
        assert_eq!(
            c.unwrap().include,
            Some(vec!["status".into(), "body".into()])
        );
    }

    #[test]
    fn merge_uses_global_enabled() {
        let c = SnapshotManager::merge_config(Some(&global(json!({"enabled": true}))), None);
        assert_eq!(c.unwrap().enabled, Some(true));
    }

    #[test]
    fn merge_request_false_or_disabled_is_none_even_with_global_enabled() {
        let g = global(json!({"enabled": true}));
        assert_eq!(
            SnapshotManager::merge_config(Some(&g), Some(&setting(json!(false)))),
            None
        );
        assert_eq!(
            SnapshotManager::merge_config(Some(&g), Some(&setting(json!({"enabled": false})))),
            None
        );
    }

    #[test]
    fn merge_detailed_keeps_name_and_merges_match_rules() {
        let c = SnapshotManager::merge_config(
            Some(&global(json!({"match": {"body.a": "*", "body.b": "*"}}))),
            Some(&setting(
                json!({"name": "n", "match": {"body.b": "regex:x", "body.c": "*"}}),
            )),
        )
        .unwrap();
        assert_eq!(c.name.as_deref(), Some("n"));
        let rules = c.match_rules.unwrap();
        assert_eq!(
            rules.into_iter().collect::<Vec<_>>(),
            vec![
                ("body.a".to_string(), "*".to_string()),
                ("body.b".to_string(), "regex:x".to_string()),
                ("body.c".to_string(), "*".to_string())
            ]
        );
    }

    // ---- filterSnapshotBody

    #[test]
    fn filter_returns_primitives_unchanged() {
        assert_eq!(filter_snapshot_body(&json!("string"), &[]), json!("string"));
        assert_eq!(filter_snapshot_body(&json!(123), &[]), json!(123));
        assert_eq!(filter_snapshot_body(&Json::Null, &[]), Json::Null);
    }

    #[test]
    fn filter_returns_body_unchanged_without_excludes() {
        let b = json!({"id": 1, "name": "test"});
        assert_eq!(filter_snapshot_body(&b, &[]), b);
    }

    fn ex(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn filter_exact_paths() {
        let b = json!({"id": 1, "timestamp": "2024-01-01", "name": "test"});
        assert_eq!(
            filter_snapshot_body(&b, &ex(&["body.timestamp"])),
            json!({"id": 1, "name": "test"})
        );
    }

    #[test]
    fn filter_wildcard_paths() {
        let b = json!({"user": {"id": 1, "createdAt": "a"}, "post": {"id": 2, "createdAt": "b"}});
        assert_eq!(
            filter_snapshot_body(&b, &ex(&["body.*.createdAt"])),
            json!({"user": {"id": 1}, "post": {"id": 2}})
        );
    }

    #[test]
    fn filter_array_wildcards() {
        let b = json!({"items": [{"id": 1, "updatedAt": "a"}, {"id": 2, "updatedAt": "b"}]});
        assert_eq!(
            filter_snapshot_body(&b, &ex(&["body.items[*].updatedAt"])),
            json!({"items": [{"id": 1}, {"id": 2}]})
        );
    }

    #[test]
    fn filter_nested_objects() {
        let b = json!({"data": {"user": {"id": 1, "meta": {"lastLogin": "x"}}}});
        assert_eq!(
            filter_snapshot_body(&b, &ex(&["body.data.user.meta.lastLogin"])),
            json!({"data": {"user": {"id": 1, "meta": {}}}})
        );
    }

    // ---- compareAndUpdate

    fn dir_mgr(tmp: &Path, mode: UpdateMode, ci: bool) -> SnapshotManager {
        SnapshotManager::new("", mode, ci).with_cwd(tmp)
    }

    fn snap_file(tmp: &Path) -> std::path::PathBuf {
        tmp.join("__snapshots__/api.snap.json")
    }

    fn run(m: &SnapshotManager, r: &ExecutionResult, c: Json) -> SnapshotCompareResult {
        m.compare_and_update("api.yaml", "Req", r, &cfg(c)).unwrap()
    }

    #[test]
    fn creates_a_new_snapshot_and_file() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        let r = run(&m, &mock_result(), json!({"include": ["status", "body"]}));
        assert_eq!(
            r,
            SnapshotCompareResult {
                matches: true,
                is_new: true,
                updated: true,
                differences: vec![]
            }
        );
        let text = std::fs::read_to_string(snap_file(tmp.path())).unwrap();
        assert!(text.starts_with(
            "{\n  \"version\": 1,\n  \"snapshots\": {\n    \"Req\": {\n      \"hash\": \""
        ));
        assert!(!text.ends_with('\n'), "TS writes no trailing newline");
        let got = m.get("api.yaml", "Req").unwrap();
        assert_eq!(got.status, Some(json!(200)));
        assert_eq!(got.hash, snapshot_hash(&got));
    }

    #[test]
    fn second_run_matches_and_leaves_the_file_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        run(&m, &mock_result(), json!({}));
        let before = std::fs::read(snap_file(tmp.path())).unwrap();
        let r = run(&m, &mock_result(), json!({}));
        assert_eq!(
            r,
            SnapshotCompareResult {
                matches: true,
                is_new: false,
                updated: false,
                differences: vec![]
            }
        );
        assert_eq!(std::fs::read(snap_file(tmp.path())).unwrap(), before);
    }

    #[test]
    fn ci_mode_fails_on_missing_snapshot_and_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, true);
        let r = run(&m, &mock_result(), json!({}));
        assert!(!r.matches && r.is_new && !r.updated);
        assert_eq!(
            r.differences,
            vec![SnapshotDiff {
                path: String::new(),
                expected: Some(json!("snapshot")),
                received: Some(json!("none")),
                diff_type: "removed".into()
            }]
        );
        assert!(!snap_file(tmp.path()).exists());
    }

    #[test]
    fn ci_mode_still_compares_existing_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        run(
            &dir_mgr(tmp.path(), UpdateMode::None, false),
            &mock_result(),
            json!({}),
        );
        let ci = dir_mgr(tmp.path(), UpdateMode::None, true);
        assert!(run(&ci, &mock_result(), json!({})).matches);
        let mut changed = mock_result();
        changed.body = Some(json!({"id": 2}));
        assert!(!run(&ci, &changed, json!({})).matches);
    }

    #[test]
    fn mismatch_without_update_mode_fails_and_keeps_file() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        run(&m, &mock_result(), json!({}));
        let before = std::fs::read(snap_file(tmp.path())).unwrap();
        let mut changed = mock_result();
        changed.body = Some(json!({"id": 2, "name": "Test", "timestamp": "2024-01-01"}));
        let r = run(&m, &changed, json!({}));
        assert!(!r.matches && !r.updated && !r.is_new);
        assert_eq!(r.differences[0].path, "body.id");
        assert_eq!(std::fs::read(snap_file(tmp.path())).unwrap(), before);
    }

    #[test]
    fn update_modes_all_and_failing_rewrite_and_report_updated() {
        for mode in [UpdateMode::All, UpdateMode::Failing] {
            let tmp = tempfile::tempdir().unwrap();
            run(
                &dir_mgr(tmp.path(), UpdateMode::None, false),
                &mock_result(),
                json!({}),
            );
            let m = dir_mgr(tmp.path(), mode, false);
            let mut changed = mock_result();
            changed.body = Some(json!({"id": 2}));
            let r = run(&m, &changed, json!({}));
            assert!(r.matches && r.updated && !r.is_new);
            assert!(!r.differences.is_empty());
            assert_eq!(
                m.get("api.yaml", "Req").unwrap().body,
                Some(json!({"id": 2}))
            );
            // and the next run matches
            assert!(run(&m, &changed, json!({})).matches);
        }
    }

    #[test]
    fn update_mode_all_does_not_rewrite_a_matching_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        run(
            &dir_mgr(tmp.path(), UpdateMode::None, false),
            &mock_result(),
            json!({}),
        );
        let before = std::fs::read(snap_file(tmp.path())).unwrap();
        let r = run(
            &dir_mgr(tmp.path(), UpdateMode::All, false),
            &mock_result(),
            json!({}),
        );
        assert!(r.matches && !r.updated);
        assert_eq!(std::fs::read(snap_file(tmp.path())).unwrap(), before);
    }

    #[test]
    fn config_name_overrides_request_name_and_snapshots_accumulate() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        run(&m, &mock_result(), json!({"name": "custom"}));
        run(&m, &mock_result(), json!({}));
        let f = m.load("api.yaml").unwrap();
        assert_eq!(
            f.snapshots.keys().collect::<Vec<_>>(),
            vec!["custom", "Req"]
        );
    }

    #[test]
    fn compare_names_by_request_name_or_request_default() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        let c = cfg(json!({}));
        m.compare(&req(Some("My Req"), "a/api.yaml"), &mock_result(), &c)
            .unwrap();
        m.compare(&req(None, "a/api.yaml"), &mock_result(), &c)
            .unwrap();
        let f = m.load("a/api.yaml").unwrap();
        assert_eq!(
            f.snapshots.keys().collect::<Vec<_>>(),
            vec!["My Req", "Request"]
        );
        assert!(tmp.path().join("a/__snapshots__/api.snap.json").exists());
    }

    #[test]
    fn unusable_snapshot_file_is_never_overwritten() {
        // `\ud800` (lone surrogate) is valid in JS JSON.parse, rejected here.
        for text in [
            "{not json",
            r#"{"version":1,"snapshots":{"Req":{"hash":"h","updatedAt":"u","body":"\ud800"}}}"#,
            r#"{"version":1}"#,
        ] {
            let tmp = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
            std::fs::write(snap_file(tmp.path()), text).unwrap();
            for (mode, ci) in [
                (UpdateMode::None, false),
                (UpdateMode::All, false),
                (UpdateMode::None, true),
            ] {
                let m = dir_mgr(tmp.path(), mode, ci);
                let err = m
                    .compare_and_update("api.yaml", "Req", &mock_result(), &cfg(json!({})))
                    .unwrap_err()
                    .to_string();
                assert!(
                    err.contains("cannot be used") && err.contains("not overwritten"),
                    "{err}"
                );
                assert_eq!(
                    std::fs::read_to_string(snap_file(tmp.path())).unwrap(),
                    text
                );
            }
        }
    }

    #[test]
    fn lenient_entries_missing_hash_and_updated_at_still_compare() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
        let text = r#"{"version":1,"snapshots":{"Req":{"body":{"id":1,"name":"Test","timestamp":"2024-01-01"}},"Bad":{"body":{"id":2}}}}"#;
        std::fs::write(snap_file(tmp.path()), text).unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        let ok = run(&m, &mock_result(), json!({}));
        assert!(ok.matches && !ok.is_new && !ok.updated, "{ok:?}");
        let bad = m
            .compare_and_update("api.yaml", "Bad", &mock_result(), &cfg(json!({})))
            .unwrap();
        assert!(!bad.matches && !bad.updated);
        assert_eq!(bad.differences[0].path, "body.id");
        assert_eq!(
            std::fs::read_to_string(snap_file(tmp.path())).unwrap(),
            text
        );
    }

    #[test]
    fn string_status_in_a_stored_entry_differs_from_the_number() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
        std::fs::write(
            snap_file(tmp.path()),
            r#"{"version":1,"snapshots":{"Req":{"hash":"h","updatedAt":"u","status":"200"}}}"#,
        )
        .unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        let r = run(&m, &mock_result(), json!({"include": ["status"]}));
        assert!(!r.matches && !r.updated);
        assert_eq!(
            r.differences,
            vec![SnapshotDiff {
                path: "status".into(),
                expected: Some(json!("200")),
                received: Some(json!(200)),
                diff_type: "changed".into()
            }]
        );
    }

    #[test]
    fn falsy_entries_count_as_missing_and_odd_headers_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
        std::fs::write(
            snap_file(tmp.path()),
            r#"{"version":1,"snapshots":{"Req":null,"H":{"headers":"x","body":{"id":1,"name":"Test","timestamp":"2024-01-01"}}}}"#,
        )
        .unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, true);
        let r = run(&m, &mock_result(), json!({}));
        assert!(!r.matches && r.is_new, "null entry = missing (CI fails)");
        let h = m
            .compare_and_update("api.yaml", "H", &mock_result(), &cfg(json!({})))
            .unwrap();
        assert!(h.matches);
    }

    #[test]
    fn extra_fields_in_an_existing_file_survive_an_update() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
        std::fs::write(
            snap_file(tmp.path()),
            r#"{"version":1,"future":{"x":1},"snapshots":{"Other":{"hash":"h","updatedAt":"u","body":1}}}"#,
        )
        .unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        run(&m, &mock_result(), json!({}));
        let v: Json =
            serde_json::from_str(&std::fs::read_to_string(snap_file(tmp.path())).unwrap()).unwrap();
        assert_eq!(v["future"], json!({"x": 1}));
        assert!(v["snapshots"]["Other"].is_object() && v["snapshots"]["Req"].is_object());
    }

    // ---- files written by the TS CLI (and by JSON.stringify in Bun)

    const FIXTURES: [&str; 3] = [
        "snapshot-basic.snap.json",
        "snapshot-advanced.snap.json",
        "tricky.snap.json",
    ];

    #[test]
    fn reads_existing_ts_snapshot_files() {
        for name in FIXTURES {
            let text = fixture(name);
            let parsed: Json = serde_json::from_str(&text).unwrap();
            let tmp = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
            let stem = name.trim_end_matches(".snap.json");
            std::fs::write(tmp.path().join(format!("__snapshots__/{name}")), &text).unwrap();
            let m = dir_mgr(tmp.path(), UpdateMode::None, true);
            let yaml = format!("{stem}.yaml");
            let file = m.load(&yaml).unwrap_or_else(|| panic!("{name} loads"));
            assert_eq!(file.version, 1);
            assert!(!file.snapshots.is_empty());
            for (key, snap) in &file.snapshots {
                // hash = md5 of JSON.stringify of the object while hash was ""
                assert_eq!(snap.hash, snapshot_hash(snap), "{name}: {key}");
                // comparing a snapshot with itself (CI mode) never differs
                let diff = SnapshotDiffer::new(&cfg(json!({}))).compare(snap, snap);
                assert!(diff.matches, "{name}: {key}");
                assert_eq!(&m.get(&yaml, key).unwrap(), snap);
            }
            // a result equal to the stored body matches in CI mode, no rewrite
            let (key, snap) = file.snapshots.iter().next().unwrap();
            let mut result = ExecutionResult::new(req(Some(key), &yaml), true);
            result.status = snap
                .status
                .as_ref()
                .and_then(Json::as_u64)
                .map(|n| n as u16);
            result.body = snap.body.clone();
            let include = if snap.status.is_some() {
                json!(["status", "body"])
            } else {
                json!(["body"])
            };
            let r = m
                .compare(
                    &req(Some(key), &yaml),
                    &result,
                    &cfg(json!({"include": include})),
                )
                .unwrap();
            assert!(
                r.matches && !r.updated && r.differences.is_empty(),
                "{name}: {r:?}"
            );
            assert_eq!(
                std::fs::read_to_string(tmp.path().join(format!("__snapshots__/{name}"))).unwrap(),
                text,
                "{name} untouched"
            );
            // the pretty printer reproduces the bytes, from Json and from the model
            assert_eq!(
                js_stringify_pretty(&parsed, 2),
                text,
                "{name} json round trip"
            );
            m.save(&yaml, &file).unwrap();
            assert_eq!(
                std::fs::read_to_string(tmp.path().join(format!("__snapshots__/{name}"))).unwrap(),
                text,
                "{name} model round trip"
            );
        }
    }

    #[test]
    fn update_of_one_entry_keeps_other_entries_byte_identical() {
        let text = fixture("tricky.snap.json");
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("__snapshots__")).unwrap();
        std::fs::write(tmp.path().join("__snapshots__/tricky.snap.json"), &text).unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::All, false);
        let mut r = ExecutionResult::new(req(None, "tricky.yaml"), true);
        r.body = Some(json!("changed"));
        m.compare_and_update("tricky.yaml", "string body", &r, &cfg(json!({})))
            .unwrap();
        let after: Json = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join("__snapshots__/tricky.snap.json")).unwrap(),
        )
        .unwrap();
        let before: Json = serde_json::from_str(&text).unwrap();
        for k in [
            "numbers",
            "unicode",
            "index keys",
            "empty",
            "null body",
            "array body",
        ] {
            assert_eq!(after["snapshots"][k], before["snapshots"][k], "{k}");
        }
        assert_eq!(after["snapshots"]["string body"]["body"], json!("changed"));
    }

    #[test]
    fn null_body_snapshot_round_trips_and_differs_from_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let m = dir_mgr(tmp.path(), UpdateMode::None, false);
        let mut r = mock_result();
        r.body = Some(Json::Null);
        run(&m, &r, json!({}));
        assert_eq!(m.get("api.yaml", "Req").unwrap().body, Some(Json::Null));
        assert!(run(&m, &r, json!({})).matches);
        r.body = None;
        let d = run(&m, &r, json!({}));
        assert_eq!(d.differences[0].diff_type, "type_mismatch");
    }
}
