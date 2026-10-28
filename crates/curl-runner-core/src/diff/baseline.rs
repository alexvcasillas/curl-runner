//! Baseline files: path, load/save, create, hash, list and clean
//! (`diff/baseline-manager.ts`). Files stay byte-compatible with TS:
//! `JSON.stringify(data, null, 2)`, no trailing newline.

use crate::jsfile::{headers_field, read_json_file, split_path, stem, text_field, untyped_entry};
use crate::model::{
    Baseline, BaselineFile, DiffConfig, DiffSetting, ExecutionResult, GlobalDiffConfig, Json,
};
use crate::snapshot::json::{js_entries, js_stringify_pretty};
use crate::snapshot::manager::{hash_json, normalize_headers, now_iso, path_join};
use indexmap::IndexMap;
use std::path::PathBuf;

const BASELINE_VERSION: u64 = 1;
pub const DEFAULT_BASELINE_DIR: &str = "__baselines__";

#[derive(Debug, Clone)]
pub struct BaselineManager {
    dir: String,
    cwd: PathBuf,
}

impl BaselineManager {
    /// `dir` empty or unset means `__baselines__` (TS `dir || DEFAULT`).
    pub fn new(global: Option<&GlobalDiffConfig>) -> Self {
        let dir = global
            .and_then(|g| g.dir.as_deref())
            .filter(|d| !d.is_empty())
            .unwrap_or(DEFAULT_BASELINE_DIR);
        Self {
            dir: dir.to_string(),
            cwd: PathBuf::new(),
        }
    }

    /// Base for relative YAML paths (the process cwd in TS).
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// `<dirname(yaml)>/<dir>/<basename without extension>.<label>.baseline.json`,
    /// joined and normalized like `path.join`.
    pub fn baseline_path(&self, yaml_path: &str, label: &str) -> String {
        let (dir, file) = split_path(yaml_path);
        path_join(&[
            dir,
            &self.dir,
            &format!("{}.{label}.baseline.json", stem(file)),
        ])
    }

    /// `<dirname(yaml)>/<dir>`.
    pub fn baseline_dir(&self, yaml_path: &str) -> String {
        path_join(&[split_path(yaml_path).0, &self.dir])
    }

    fn fs_path(&self, yaml_path: &str, label: &str) -> PathBuf {
        self.cwd.join(self.baseline_path(yaml_path, label))
    }

    /// The raw baseline file: `Ok(None)` when it does not exist, `Err` when it
    /// exists but cannot be read, parsed or is not shaped `{ baselines: {...} }`.
    /// Callers that would replace the file on their own initiative must not do
    /// so for an `Err` file.
    pub fn read_file(&self, yaml_path: &str, label: &str) -> std::io::Result<Option<Json>> {
        read_json_file(&self.fs_path(yaml_path, label), "baseline", "baselines")
    }

    /// Loads the baseline file for a label (`None` when missing or unusable,
    /// like TS, whose `load` returned null for anything it could not parse).
    /// Entries are read like untyped JS objects; falsy ones are skipped.
    pub fn load(&self, yaml_path: &str, label: &str) -> Option<BaselineFile> {
        let raw = self.read_file(yaml_path, label).ok()??;
        let baselines = js_entries(raw.get("baselines")?.as_object()?)
            .into_iter()
            .filter_map(|(k, v)| Some((k.clone(), parse_baseline(v)?)))
            .collect();
        Some(BaselineFile {
            version: raw
                .get("version")
                .and_then(Json::as_u64)
                .unwrap_or(BASELINE_VERSION),
            label: raw
                .get("label")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string(),
            captured_at: raw
                .get("capturedAt")
                .and_then(Json::as_str)
                .unwrap_or_default()
                .to_string(),
            baselines,
        })
    }

    /// Writes the file pretty-printed with 2 spaces and no trailing newline.
    /// TS serialized writes per path; here every write is one `fs::write` of
    /// the whole file and the callers are sequential.
    pub fn save(&self, yaml_path: &str, label: &str, data: &BaselineFile) -> std::io::Result<()> {
        let path = self.fs_path(yaml_path, label);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let v = serde_json::to_value(data).map_err(std::io::Error::other)?;
        std::fs::write(path, js_stringify_pretty(&v, 2))
    }

    /// One baseline by request name (`None` when missing or falsy).
    pub fn get(&self, yaml_path: &str, label: &str, request_name: &str) -> Option<Baseline> {
        self.load(yaml_path, label)?
            .baselines
            .get(request_name)
            .cloned()
    }

    /// Labels of the baseline files of a YAML file, sorted (UTF-16 order).
    /// The file stem goes into the pattern unescaped, as in TS.
    pub fn list_labels(&self, yaml_path: &str) -> Vec<String> {
        let (_, file) = split_path(yaml_path);
        let pattern = format!("^{}\\.(.+)\\.baseline\\.json$", stem(file));
        let Some(re) = crate::jsval::compile_regex(&pattern, false) else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(self.cwd.join(self.baseline_dir(yaml_path))) else {
            return Vec::new();
        };
        let mut labels: Vec<String> = entries
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter_map(|name| {
                let caps = re.captures(&name).ok()??;
                Some(caps.get(1)?.as_str().to_string())
            })
            .collect();
        labels.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        labels
    }

    /// Baseline of a result: status, normalized headers, body and (with
    /// `includeTimings`) the duration; `hash` is over the object while `hash`
    /// is still `""` (so it includes `capturedAt`).
    pub fn create_baseline(&self, result: &ExecutionResult, config: &DiffConfig) -> Baseline {
        let mut b = Baseline {
            captured_at: now_iso(),
            ..Default::default()
        };
        b.status = result.status.map(Json::from);
        if let Some(h) = &result.headers {
            b.headers = Some(normalize_headers(h));
        }
        b.body = result.body.clone();
        if config.include_timings == Some(true) {
            b.timing = result.metrics.as_ref().map(|m| m.duration);
        }
        b.hash = baseline_hash(&b);
        b
    }

    /// First 8 hex digits of md5 over `JSON.stringify(content)`.
    pub fn hash(&self, content: &Json) -> String {
        hash_json(content)
    }

    /// Saves results as a baseline file (skipped and failed results are left
    /// out; the whole file is replaced, as in TS).
    pub fn save_baseline(
        &self,
        yaml_path: &str,
        label: &str,
        results: &[ExecutionResult],
        config: &DiffConfig,
    ) -> std::io::Result<()> {
        let mut baselines: IndexMap<String, Baseline> = IndexMap::new();
        for r in results {
            if r.skipped == Some(true) || !r.success {
                continue;
            }
            baselines.insert(request_key(r), self.create_baseline(r, config));
        }
        let file = BaselineFile {
            version: BASELINE_VERSION,
            label: label.to_string(),
            captured_at: now_iso(),
            baselines,
        };
        self.save(yaml_path, label, &file)
    }

    /// Deletes `*.baseline.json` files older than `older_than_days`; returns
    /// how many were removed (errors end the scan quietly, as in TS).
    pub fn clean_old_baselines(&self, yaml_path: &str, older_than_days: f64) -> usize {
        let dir = self.cwd.join(self.baseline_dir(yaml_path));
        let age =
            std::time::Duration::try_from_secs_f64(older_than_days * 86_400.0).unwrap_or_default();
        let Some(cutoff) = std::time::SystemTime::now().checked_sub(age) else {
            return 0;
        };
        let mut deleted = 0;
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return 0;
        };
        for e in entries.flatten() {
            if !e.file_name().to_string_lossy().ends_with(".baseline.json") {
                continue;
            }
            let Ok(modified) = e.metadata().and_then(|m| m.modified()) else {
                return deleted;
            };
            if modified < cutoff {
                if std::fs::remove_file(e.path()).is_err() {
                    return deleted;
                }
                deleted += 1;
            }
        }
        deleted
    }

    /// Request config merged with the global one; `None` when diffing is not
    /// enabled for the request.
    pub fn merge_config(
        global: Option<&GlobalDiffConfig>,
        request: Option<&DiffSetting>,
    ) -> Option<DiffConfig> {
        let g_enabled = global.and_then(|g| g.enabled) == Some(true);
        let g_exclude = || global.and_then(|g| g.exclude.clone()).unwrap_or_default();
        let g_match = || {
            global
                .and_then(|g| g.match_rules.clone())
                .unwrap_or_default()
        };
        let g_timings = global.and_then(|g| g.include_timings);
        let from_global = || DiffConfig {
            enabled: Some(true),
            exclude: Some(g_exclude()),
            match_rules: Some(g_match()),
            include_timings: Some(g_timings == Some(true)),
        };
        match request {
            None | Some(DiffSetting::Bool(false)) if !g_enabled => None,
            Some(DiffSetting::Bool(true)) => Some(from_global()),
            Some(DiffSetting::Config(c)) if c.enabled != Some(false) => {
                let mut exclude = g_exclude();
                exclude.extend(c.exclude.iter().flatten().cloned());
                let mut rules = g_match();
                for (k, v) in c.match_rules.iter().flatten() {
                    rules.insert(k.clone(), v.clone());
                }
                Some(DiffConfig {
                    enabled: Some(true),
                    exclude: Some(exclude),
                    match_rules: Some(rules),
                    include_timings: Some(c.include_timings.or(g_timings).unwrap_or(false)),
                })
            }
            None if g_enabled => Some(from_global()),
            _ => None,
        }
    }
}

/// Baseline key of a result: `request.name || request.url`.
pub(crate) fn request_key(r: &ExecutionResult) -> String {
    match r.request.name.as_deref() {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => r.request.url.clone(),
    }
}

/// Hash as TS computes it: over the baseline while `hash` is still `""`.
fn baseline_hash(b: &Baseline) -> String {
    let unhashed = Baseline {
        hash: String::new(),
        ..b.clone()
    };
    serde_json::to_value(&unhashed)
        .map(|v| hash_json(&v))
        .unwrap_or_default()
}

/// Reads a stored entry like TS reads an untyped object (see
/// [`untyped_entry`]); falsy entries count as missing.
pub(crate) fn parse_baseline(v: &Json) -> Option<Baseline> {
    let o = untyped_entry(v)?;
    Some(Baseline {
        hash: text_field(o, "hash"),
        captured_at: text_field(o, "capturedAt"),
        status: o.get("status").cloned(),
        headers: headers_field(o),
        body: o.get("body").cloned(),
        timing: o.get("timing").and_then(Json::as_f64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HeaderValue, Metrics, RequestConfig};
    use serde_json::json;

    fn mgr() -> BaselineManager {
        BaselineManager::new(None)
    }

    fn gdir(d: &str) -> GlobalDiffConfig {
        GlobalDiffConfig {
            dir: Some(d.into()),
            ..Default::default()
        }
    }

    #[test]
    fn baseline_path_cases() {
        assert_eq!(
            mgr().baseline_path("tests/api.yaml", "staging"),
            "tests/__baselines__/api.staging.baseline.json"
        );
        assert_eq!(
            BaselineManager::new(Some(&gdir(".baselines"))).baseline_path("api.yaml", "prod"),
            ".baselines/api.prod.baseline.json"
        );
        assert_eq!(
            mgr().baseline_path("tests/integration/users.yaml", "staging"),
            "tests/integration/__baselines__/users.staging.baseline.json"
        );
        assert_eq!(
            mgr().baseline_path("api.yaml", "v1.0.0"),
            "__baselines__/api.v1.0.0.baseline.json"
        );
        // absolute dir does not reset (path.join)
        assert_eq!(
            BaselineManager::new(Some(&gdir("/abs"))).baseline_path("t/a.yaml", "x"),
            "t/abs/a.x.baseline.json"
        );
    }

    #[test]
    fn baseline_dir_cases() {
        assert_eq!(mgr().baseline_dir("tests/api.yaml"), "tests/__baselines__");
        assert_eq!(
            BaselineManager::new(Some(&gdir("snapshots"))).baseline_dir("api.yaml"),
            "snapshots"
        );
    }

    fn result() -> ExecutionResult {
        let req: RequestConfig =
            serde_json::from_value(json!({"url": "https://api.example.com", "name": "Get Users"}))
                .unwrap();
        let mut r = ExecutionResult::new(req, true);
        r.status = Some(200);
        let mut h = indexmap::IndexMap::new();
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
        r.metrics = Some(Metrics {
            duration: 150.0,
            ..Default::default()
        });
        r
    }

    #[test]
    fn create_baseline_all_fields() {
        let b = mgr().create_baseline(&result(), &DiffConfig::default());
        assert_eq!(b.status, Some(json!(200)));
        assert_eq!(b.body, result().body);
        assert!(b.headers.is_some());
        assert_eq!(b.hash.len(), 8);
        assert!(!b.captured_at.is_empty());
        assert_eq!(b.timing, None);
    }

    #[test]
    fn create_baseline_timing_when_configured() {
        let cfg = DiffConfig {
            include_timings: Some(true),
            ..Default::default()
        };
        assert_eq!(mgr().create_baseline(&result(), &cfg).timing, Some(150.0));
    }

    #[test]
    fn headers_are_lowercased_and_sorted() {
        let b = mgr().create_baseline(&result(), &DiffConfig::default());
        let h = b.headers.unwrap();
        assert_eq!(
            h.keys().map(String::as_str).collect::<Vec<_>>(),
            ["content-type", "x-request-id"]
        );
        assert_eq!(h["content-type"], json!("application/json"));
    }

    #[test]
    fn hash_cases() {
        let m = mgr();
        let c = json!({"id": 1, "name": "test"});
        assert_eq!(m.hash(&c), m.hash(&c));
        assert_ne!(m.hash(&json!({"id": 1})), m.hash(&json!({"id": 2})));
        assert_eq!(m.hash(&json!({"data": "test"})).len(), 8);
        // md5('{"id":1}') = 5d3f..., independent of Bun's implementation
        assert_eq!(m.hash(&json!({"id": 1})).len(), 8);
    }

    #[test]
    fn baseline_hash_covers_captured_at_with_hash_empty() {
        let b = mgr().create_baseline(&result(), &DiffConfig::default());
        let mut v = serde_json::to_value(&b).unwrap();
        v["hash"] = json!("");
        assert_eq!(b.hash, mgr().hash(&v));
        assert!(crate::snapshot::js_stringify(&v).starts_with("{\"hash\":\"\",\"capturedAt\":\""));
    }

    fn g(v: serde_json::Value) -> GlobalDiffConfig {
        serde_json::from_value(v).unwrap()
    }
    fn s(v: serde_json::Value) -> DiffSetting {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn merge_none_when_not_enabled() {
        assert_eq!(
            BaselineManager::merge_config(Some(&g(json!({}))), None),
            None
        );
        assert_eq!(BaselineManager::merge_config(None, None), None);
        assert_eq!(
            BaselineManager::merge_config(Some(&g(json!({}))), Some(&s(json!({"enabled": false})))),
            None
        );
        // `false` request beats a global enabled
        assert_eq!(
            BaselineManager::merge_config(
                Some(&g(json!({"enabled": true}))),
                Some(&s(json!(false)))
            ),
            None
        );
        assert_eq!(
            BaselineManager::merge_config(
                Some(&g(json!({"enabled": true}))),
                Some(&s(json!({"enabled": false})))
            ),
            None
        );
    }

    #[test]
    fn merge_true_and_global() {
        let c = BaselineManager::merge_config(Some(&g(json!({}))), Some(&s(json!(true)))).unwrap();
        assert_eq!(
            c,
            DiffConfig {
                enabled: Some(true),
                exclude: Some(vec![]),
                match_rules: Some(Default::default()),
                include_timings: Some(false)
            }
        );
        let c = BaselineManager::merge_config(Some(&g(json!({"enabled": true}))), None).unwrap();
        assert_eq!(c.enabled, Some(true));
    }

    #[test]
    fn merge_excludes_matches_timings() {
        let c = BaselineManager::merge_config(
            Some(&g(json!({"exclude": ["*.timestamp"], "match": {"body.id": "*"}}))),
            Some(&s(json!({"enabled": true, "exclude": ["body.id"], "match": {"body.token": "regex:^[a-z]+$"}}))),
        )
        .unwrap();
        assert_eq!(c.exclude.unwrap(), ["*.timestamp", "body.id"]);
        let m = c.match_rules.unwrap();
        assert_eq!(m["body.id"], "*");
        assert_eq!(m["body.token"], "regex:^[a-z]+$");
        let c = BaselineManager::merge_config(
            Some(&g(json!({"enabled": true, "includeTimings": true}))),
            Some(&s(json!(true))),
        )
        .unwrap();
        assert_eq!(c.include_timings, Some(true));
        let c = BaselineManager::merge_config(
            Some(&g(json!({"enabled": true, "includeTimings": false}))),
            Some(&s(json!({"enabled": true, "includeTimings": true}))),
        )
        .unwrap();
        assert_eq!(c.include_timings, Some(true));
    }

    #[test]
    fn save_load_list_clean_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let m = mgr().with_cwd(tmp.path());
        let mut ok = result();
        ok.request.name = None; // keyed by url
        let mut failed = result();
        failed.success = false;
        let mut skipped = result();
        skipped.skipped = Some(true);
        m.save_baseline(
            "t.yaml",
            "b",
            &[ok, failed, skipped],
            &DiffConfig::default(),
        )
        .unwrap();
        m.save_baseline("t.yaml", "a.1", &[result()], &DiffConfig::default())
            .unwrap();
        std::fs::write(tmp.path().join("__baselines__/other.x.baseline.json"), "{}").unwrap();
        let f = m.load("t.yaml", "b").unwrap();
        assert_eq!(f.version, 1);
        assert_eq!(f.label, "b");
        assert_eq!(
            f.baselines.keys().collect::<Vec<_>>(),
            ["https://api.example.com"]
        );
        assert!(m.get("t.yaml", "a.1", "Get Users").is_some());
        assert!(m.get("t.yaml", "a.1", "nope").is_none());
        assert_eq!(m.list_labels("t.yaml"), ["a.1", "b"]);
        assert_eq!(m.list_labels("nofile.yaml"), Vec::<String>::new());
        let bytes =
            std::fs::read_to_string(tmp.path().join("__baselines__/t.b.baseline.json")).unwrap();
        assert!(
            bytes.starts_with("{\n  \"version\": 1,\n  \"label\": \"b\",\n  \"capturedAt\": \"")
        );
        assert!(!bytes.ends_with('\n'));
        // nothing is older than a day; everything is older than -1 day
        assert_eq!(m.clean_old_baselines("t.yaml", 1.0), 0);
        assert_eq!(m.clean_old_baselines("t.yaml", -1.0), 3);
        assert_eq!(m.list_labels("t.yaml"), Vec::<String>::new());
    }

    #[test]
    fn unusable_files_load_as_none_but_report_err() {
        let tmp = tempfile::tempdir().unwrap();
        let m = mgr().with_cwd(tmp.path());
        let d = tmp.path().join("__baselines__");
        std::fs::create_dir_all(&d).unwrap();
        for (i, bad) in ["{nope", "[]", "{\"baselines\": 3}"].iter().enumerate() {
            std::fs::write(d.join(format!("t.l{i}.baseline.json")), bad).unwrap();
            let l = format!("l{i}");
            assert!(m.load("t.yaml", &l).is_none());
            assert!(m.read_file("t.yaml", &l).is_err());
        }
        assert!(m.read_file("t.yaml", "missing").unwrap().is_none());
    }

    #[test]
    fn lenient_entries_and_js_key_order() {
        let tmp = tempfile::tempdir().unwrap();
        let m = mgr().with_cwd(tmp.path());
        let d = tmp.path().join("__baselines__");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("t.x.baseline.json"),
            r#"{"baselines":{"b":{"status":"200","timing":"x"},"2":{},"a":null,"1":0}}"#,
        )
        .unwrap();
        let f = m.load("t.yaml", "x").unwrap();
        assert_eq!(f.baselines.keys().collect::<Vec<_>>(), ["2", "b"]);
        assert_eq!(f.baselines["b"].status, Some(json!("200")));
        assert_eq!(f.baselines["b"].timing, None);
        assert_eq!(f.version, 1);
    }
}

#[cfg(test)]
mod ts_fixture_tests {
    //! Files written by the real TS CLI / `BaselineManager` (Bun), kept
    //! byte-for-byte under `tests/fixtures/ts-baselines`.
    use super::*;
    use crate::diff::DiffOrchestrator;
    use crate::snapshot::js_stringify_pretty;
    use serde_json::json;
    use std::path::Path;

    const FIXTURES: [(&str, &str, &str); 3] = [
        (
            "api",
            "staging",
            include_str!("../../tests/fixtures/ts-baselines/api.staging.baseline.json"),
        ),
        (
            "api",
            "production",
            include_str!("../../tests/fixtures/ts-baselines/api.production.baseline.json"),
        ),
        (
            "tricky",
            "edge",
            include_str!("../../tests/fixtures/ts-baselines/tricky.edge.baseline.json"),
        ),
    ];

    fn install(tmp: &Path) {
        std::fs::create_dir_all(tmp.join("__baselines__")).unwrap();
        for (stem, label, text) in FIXTURES {
            std::fs::write(
                tmp.join(format!("__baselines__/{stem}.{label}.baseline.json")),
                text,
            )
            .unwrap();
        }
    }

    #[test]
    fn reads_existing_ts_baselines() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path());
        let m = BaselineManager::new(None).with_cwd(tmp.path());
        for (stem, label, text) in FIXTURES {
            let yaml = format!("{stem}.yaml");
            let parsed: Json = serde_json::from_str(text).unwrap();
            let file = m
                .load(&yaml, label)
                .unwrap_or_else(|| panic!("{label} loads"));
            assert_eq!((file.version, file.label.as_str()), (1, label));
            assert!(!file.baselines.is_empty());
            for (key, b) in &file.baselines {
                // hash = md5 of JSON.stringify of the object while hash was ""
                assert_eq!(b.hash, baseline_hash(b), "{label}: {key}");
                assert!(!b.captured_at.is_empty());
                let d = crate::diff::ResponseDiffer::new(&DiffConfig {
                    include_timings: Some(true),
                    ..Default::default()
                });
                let r = d.compare(b, b, label, label, key);
                assert!(!r.has_differences, "{label}: {key}");
            }
            // the pretty printer reproduces the bytes, from Json and from the model
            assert_eq!(
                js_stringify_pretty(&parsed, 2),
                text,
                "{label} json round trip"
            );
            m.save(&yaml, label, &file).unwrap();
            let again = std::fs::read_to_string(
                tmp.path()
                    .join(format!("__baselines__/{stem}.{label}.baseline.json")),
            )
            .unwrap();
            assert_eq!(again, text, "{label} typed save reproduces TS bytes");
        }
        assert_eq!(m.list_labels("api.yaml"), ["production", "staging"]);
        assert_eq!(m.list_labels("tricky.yaml"), ["edge"]);
    }

    #[test]
    fn stored_ts_baselines_compare_like_ts() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path());
        let o = DiffOrchestrator::new(None, tmp.path());
        let cfg = DiffConfig {
            exclude: Some(vec!["*.date".into(), "*.timing".into()]),
            ..Default::default()
        };
        let s = o
            .compare_two_baselines("api.yaml", "staging", "production", &cfg)
            .unwrap();
        // Get User (user 1 vs 2), the unnamed url-keyed one (same body), List Posts
        // only in staging, Only Here only in production
        let names: Vec<_> = s.results.iter().map(|r| r.request_name.as_str()).collect();
        assert_eq!(names.len(), 4);
        assert_eq!(&names[..2], ["Get User", "List Posts"]);
        assert!(names[3] == "Only Here");
        assert!(s.results[0].has_differences);
        assert!(s.results[0].differences.iter().any(|d| d.path == "body.id"
            && d.baseline == Some(json!(1))
            && d.current == Some(json!(2))));
        assert_eq!(s.results[1].differences[0].diff_type, "removed");
        assert_eq!(s.results[3].differences[0].diff_type, "added");
        assert!(!s.results[2].has_differences, "{:?}", s.results[2]);
        assert_eq!((s.unchanged, s.changed, s.new_baselines), (1, 3, 0));
        // missing label
        assert_eq!(
            o.compare_two_baselines("api.yaml", "staging", "nope", &cfg)
                .unwrap_err(),
            "Baseline 'nope' not found"
        );
    }
}
