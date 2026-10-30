//! Local run history: one JSON file per run under
//! `{base}/history/{root hash}/`, pruned after 30 days, plus the comparison
//! with the previous run of the same target and environment.
//!
//! Only redacted data is stored: bodies are redacted again here and capped,
//! and a run's captured-value stores are never persisted.

use crate::model::result_view::checks;
use crate::model::run::{RunOutcome, RunTarget};
use chrono::{DateTime, Duration, Local};
use curl_runner_core::model::Json;
use curl_runner_core::security::redact::Redactor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Bodies longer than this are cut (on a char boundary).
const BODY_CAP: usize = 64 * 1024;
const KEEP_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    CollectionRun,
    Request,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub at: DateTime<Local>,
    pub collection: String,
    /// A file path, `file#n` or the collection: whatever was run.
    pub target: String,
    pub kind: EntryKind,
    pub environment: String,
    pub passed: u64,
    pub total: u64,
    pub duration_ms: f64,
    pub retries: u32,
    pub results: Vec<StoredResult>,
    /// The curl-runner version that ran it; empty in entries written before
    /// it was stored.
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredResult {
    /// Relative to the collection root.
    pub file: PathBuf,
    pub name: String,
    pub method: String,
    pub status: Option<u16>,
    pub success: bool,
    pub duration_ms: f64,
    pub attempts: u32,
    pub checks_passed: usize,
    pub checks_total: usize,
    /// Labels of the failed checks.
    pub failed_checks: Vec<String>,
    /// Redacted, at most 64 KB.
    pub body: Option<String>,
}

/// One collection's history directory.
pub struct History {
    dir: PathBuf,
}

impl History {
    /// `{base}/history/{hash of the root}/`, created on the first `record`.
    pub fn open(base: &Path, collection_root: &Path) -> History {
        let root = collection_root
            .canonicalize()
            .unwrap_or_else(|_| collection_root.to_path_buf());
        let hash = fnv1a(root.to_string_lossy().as_bytes());
        History {
            dir: base.join("history").join(format!("{hash:016x}")),
        }
    }

    /// Write atomically: a temp file, then a rename.
    pub fn record(&self, e: &HistoryEntry) -> io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let name = format!("{}-{}.json", e.at.format("%Y%m%dT%H%M%S"), e.id);
        let tmp = self.dir.join(format!(".{name}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec(e).map_err(io::Error::other)?)?;
        std::fs::rename(&tmp, self.dir.join(name)).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }

    /// Newest first; unreadable files are skipped.
    pub fn list(&self) -> Vec<HistoryEntry> {
        let mut out: Vec<HistoryEntry> = self
            .files()
            .filter_map(|(p, _)| std::fs::read(p).ok())
            .filter_map(|b| serde_json::from_slice(&b).ok())
            .collect();
        out.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| b.id.cmp(&a.id)));
        out
    }

    /// Delete everything older than 30 days.
    pub fn prune(&self, now: DateTime<Local>) {
        let cutoff = now - Duration::days(KEEP_DAYS);
        for (path, _) in self.files() {
            let old = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<HistoryEntry>(&b).ok())
                .is_some_and(|e| e.at < cutoff);
            if old {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    /// Stored runs and their size in bytes.
    pub fn usage(&self) -> (usize, u64) {
        self.files()
            .fold((0, 0), |(n, bytes), (_, len)| (n + 1, bytes + len))
    }

    /// Entry files with their sizes.
    fn files(&self) -> impl Iterator<Item = (PathBuf, u64)> {
        std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| {
                let n = f.file_name();
                let n = n.to_string_lossy();
                n.ends_with(".json") && !n.starts_with('.')
            })
            .filter_map(|f| Some((f.path(), f.metadata().ok()?.len())))
    }
}

/// FNV-1a 64: stable across releases, unlike `DefaultHasher`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A finished run as an entry. `collection` is left empty for the caller
/// (the outcome does not know it). `stores` are never read.
pub fn entry_from_outcome(
    o: &RunOutcome,
    target: &str,
    kind: EntryKind,
    env: &str,
    redactor: &Redactor,
) -> HistoryEntry {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let at = o.started_at;
    let mut seed = format!(
        "{}|{target}|{}",
        at.timestamp_nanos_opt().unwrap_or_default(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    seed.push_str(env);
    let id = format!("{:08x}", fnv1a(seed.as_bytes()) as u32);

    let results: Vec<StoredResult> = o
        .summary
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let all = checks(r);
            StoredResult {
                file: r
                    .request
                    .source_file
                    .as_deref()
                    .map(PathBuf::from)
                    .unwrap_or_default(),
                name: r
                    .request
                    .name
                    .clone()
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| r.request.url.clone()),
                method: r.request.method.clone().unwrap_or_else(|| "GET".into()),
                status: r.status,
                success: r.success,
                duration_ms: r.metrics.as_ref().map_or(0.0, |m| m.duration),
                attempts: o.attempts.get(i).copied().unwrap_or(0),
                checks_passed: all.iter().filter(|c| c.passed).count(),
                checks_total: all.len(),
                failed_checks: all
                    .iter()
                    .filter(|c| !c.passed)
                    .map(|c| c.label.clone())
                    .collect(),
                body: r.body.as_ref().map(|b| stored_body(b, redactor)),
            }
        })
        .collect();
    let retries = (0..results.len())
        .map(|i| o.attempts.get(i).copied().unwrap_or(0).saturating_sub(1))
        .sum();
    HistoryEntry {
        id,
        at,
        collection: String::new(),
        target: target.to_string(),
        kind,
        environment: env.to_string(),
        passed: o.summary.successful,
        total: o.summary.total,
        duration_ms: o.summary.duration,
        retries,
        results,
        version: curl_runner_core::VERSION.to_string(),
    }
}

/// Redact first (a secret must not straddle the cut), then cap.
fn stored_body(body: &Json, redactor: &Redactor) -> String {
    let text = match body {
        Json::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    };
    let mut text = redactor.redact(&text);
    if text.len() > BODY_CAP {
        let mut end = BODY_CAP;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// What changed since the previous run.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub newly_failing: Vec<StoredResult>,
    pub fixed: Vec<StoredResult>,
    /// With the previous duration in ms.
    pub slower: Vec<(StoredResult, f64)>,
    pub unchanged: usize,
}

/// Results match by `(file, name)`; results without a counterpart are not
/// counted. Slower: at least 25% and more than 50 ms over the previous run.
pub fn compare(current: &HistoryEntry, previous: &HistoryEntry) -> Comparison {
    let mut prev: HashMap<(&Path, &str), &StoredResult> = HashMap::new();
    for r in &previous.results {
        prev.entry((r.file.as_path(), r.name.as_str())).or_insert(r);
    }
    let mut out = Comparison {
        newly_failing: Vec::new(),
        fixed: Vec::new(),
        slower: Vec::new(),
        unchanged: 0,
    };
    for cur in &current.results {
        let Some(p) = prev.get(&(cur.file.as_path(), cur.name.as_str())) else {
            continue;
        };
        if !cur.success && p.success {
            out.newly_failing.push(cur.clone());
        } else if cur.success && !p.success {
            out.fixed.push(cur.clone());
        } else if cur.duration_ms >= p.duration_ms * 1.25 && cur.duration_ms - p.duration_ms > 50.0
        {
            out.slower.push((cur.clone(), p.duration_ms));
        } else {
            out.unchanged += 1;
        }
    }
    out
}

/// The latest earlier entry with the same target and environment.
pub fn previous_for<'a>(entries: &'a [HistoryEntry], e: &HistoryEntry) -> Option<&'a HistoryEntry> {
    entries
        .iter()
        .filter(|o| o.target == e.target && o.environment == e.environment && o.at < e.at)
        .max_by_key(|o| o.at)
}

/// Durations of the last `n` runs of `e`'s target and environment up to
/// and including `e`, oldest first (so `e` is the last).
pub fn duration_series(entries: &[HistoryEntry], e: &HistoryEntry, n: usize) -> Vec<f64> {
    let mut runs: Vec<&HistoryEntry> = entries
        .iter()
        .filter(|o| o.target == e.target && o.environment == e.environment && o.at <= e.at)
        .collect();
    runs.sort_by_key(|e| e.at);
    let skip = runs.len().saturating_sub(n);
    runs[skip..].iter().map(|e| e.duration_ms).collect()
}

/// How a run target is stored: `collection`, the files of a re-run each
/// ended by a newline (a path may hold `, `), a file path, or `file#index`
/// for one request of a file. Entries written before the newline form
/// joined the files by `, `; [`target_of`] still reads those.
pub fn target_key(t: &RunTarget) -> (String, EntryKind) {
    match t {
        RunTarget::Collection => ("collection".into(), EntryKind::CollectionRun),
        RunTarget::Files(fs) => (
            fs.iter()
                .map(|f| format!("{}\n", f.to_string_lossy()))
                .collect(),
            EntryKind::CollectionRun,
        ),
        RunTarget::File(f) => (f.to_string_lossy().into_owned(), EntryKind::Request),
        RunTarget::Request { file, index } => (
            format!("{}#{index}", file.to_string_lossy()),
            EntryKind::Request,
        ),
    }
}

/// The target an entry ran, from its [`target_key`].
pub fn target_of(e: &HistoryEntry) -> RunTarget {
    match e.kind {
        EntryKind::CollectionRun if e.target == "collection" => RunTarget::Collection,
        EntryKind::CollectionRun if e.target.contains('\n') => {
            RunTarget::Files(e.target.lines().map(PathBuf::from).collect())
        }
        EntryKind::CollectionRun => {
            RunTarget::Files(e.target.split(", ").map(PathBuf::from).collect())
        }
        EntryKind::Request => match e.target.rsplit_once('#') {
            Some((file, n)) if n.parse::<usize>().is_ok() => RunTarget::Request {
                file: file.into(),
                index: n.parse().unwrap_or_default(),
            },
            _ => RunTarget::File(e.target.clone().into()),
        },
    }
}

/// The History screen's kind tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    #[default]
    All,
    CollectionRuns,
    Requests,
    /// Runs with at least one request that did not pass.
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub kind: KindFilter,
    /// `None`: every environment.
    pub environment: Option<String>,
}

/// Indices of the entries `f` keeps, in their order.
pub fn filter_entries(e: &[HistoryEntry], f: &Filter) -> Vec<usize> {
    e.iter()
        .enumerate()
        .filter(|(_, e)| match f.kind {
            KindFilter::All => true,
            KindFilter::CollectionRuns => e.kind == EntryKind::CollectionRun,
            KindFilter::Requests => e.kind == EntryKind::Request,
            KindFilter::Failed => e.passed < e.total,
        })
        .filter(|(_, e)| {
            f.environment
                .as_ref()
                .is_none_or(|env| *env == e.environment)
        })
        .map(|(i, _)| i)
        .collect()
}

/// The list's day heading: `Today`, `Yesterday`, else `Oct 27`.
pub fn day_bucket(at: DateTime<Local>, now: DateTime<Local>) -> String {
    let (day, today) = (at.date_naive(), now.date_naive());
    if day == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(day) {
        "Yesterday".to_string()
    } else {
        at.format("%b %-d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use curl_runner_core::executor::create_summary;
    use curl_runner_core::model::{ExecutionResult, Metrics, RequestConfig};

    fn at(day: u32, h: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, day, h, 0, 0).unwrap()
    }

    fn stored(name: &str, ok: bool, ms: f64) -> StoredResult {
        StoredResult {
            file: "a.yaml".into(),
            name: name.into(),
            method: "GET".into(),
            status: Some(if ok { 200 } else { 500 }),
            success: ok,
            duration_ms: ms,
            attempts: 1,
            checks_passed: 0,
            checks_total: 0,
            failed_checks: vec![],
            body: None,
        }
    }

    fn entry(id: &str, when: DateTime<Local>, results: Vec<StoredResult>) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            at: when,
            collection: "Payments API".into(),
            target: "a.yaml".into(),
            kind: EntryKind::CollectionRun,
            environment: "staging".into(),
            passed: 1,
            total: 1,
            duration_ms: 10.0,
            retries: 0,
            results,
            version: "2.1.1".into(),
        }
    }

    fn history() -> (tempfile::TempDir, History) {
        let d = tempfile::tempdir().unwrap();
        let h = History::open(d.path(), Path::new("/projects/acme"));
        (d, h)
    }

    #[test]
    fn record_list_roundtrip_newest_first() {
        let (_d, h) = history();
        let old = entry("aaaa0001", at(1, 9), vec![stored("x", true, 5.0)]);
        let new = entry("aaaa0002", at(2, 9), vec![stored("x", false, 7.0)]);
        h.record(&old).unwrap();
        h.record(&new).unwrap();
        assert_eq!(h.list(), [new, old]);
        let (n, bytes) = h.usage();
        assert_eq!(n, 2);
        assert!(bytes > 0);
    }

    #[test]
    fn open_is_stable_per_root() {
        let d = tempfile::tempdir().unwrap();
        let a = History::open(d.path(), Path::new("/projects/acme"));
        let b = History::open(d.path(), Path::new("/projects/acme"));
        let c = History::open(d.path(), Path::new("/projects/other"));
        assert_eq!(a.dir, b.dir);
        assert_ne!(a.dir, c.dir);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn prune_drops_entries_older_than_30_days() {
        let (_d, h) = history();
        let now = Local.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap();
        let stale = entry("aaaa0001", now - Duration::days(31), vec![]);
        let fresh = entry("aaaa0002", now - Duration::days(29), vec![]);
        h.record(&stale).unwrap();
        h.record(&fresh).unwrap();
        h.prune(now);
        assert_eq!(h.list(), [fresh]);
        assert_eq!(h.usage().0, 1);
    }

    #[test]
    fn corrupt_file_skipped() {
        let (_d, h) = history();
        let e = entry("aaaa0001", at(1, 9), vec![]);
        h.record(&e).unwrap();
        std::fs::write(h.dir.join("20260101T000000-bad.json"), "{ not json").unwrap();
        assert_eq!(h.list(), [e]);
    }

    fn outcome_with_body(body: Json) -> RunOutcome {
        let mut r = ExecutionResult::new(
            RequestConfig {
                name: Some("Charge".into()),
                url: "https://api.example.com/charge".into(),
                method: Some("POST".into()),
                source_file: Some("pay/charge.yaml".into()),
                ..Default::default()
            },
            true,
        );
        r.status = Some(200);
        r.body = Some(body);
        r.metrics = Some(Metrics {
            duration: 42.0,
            ..Default::default()
        });
        RunOutcome {
            summary: create_summary(vec![r], 42.0),
            stopped: false,
            stores: vec![],
            attempts: vec![3],
            started_at: at(2, 9),
            redactor: Default::default(),
            keychain_names: vec![],
        }
    }

    #[test]
    fn body_capped_and_redacted() {
        let secret = "sk_live_abc";
        let text = format!("{secret} {}", "é".repeat(52_000));
        assert!(text.len() > 100 * 1024);
        let o = outcome_with_body(Json::String(text));
        let redactor = Redactor::new([secret.to_string()]);
        let e = entry_from_outcome(
            &o,
            "pay/charge.yaml",
            EntryKind::Request,
            "staging",
            &redactor,
        );
        let body = e.results[0].body.as_deref().unwrap();
        assert!(body.len() <= BODY_CAP);
        assert!(!body.contains(secret));
        assert!(body.starts_with("[REDACTED]"));
    }

    #[test]
    fn entry_fields_from_outcome() {
        let o = outcome_with_body(serde_json::json!({"ok": true}));
        let e = entry_from_outcome(
            &o,
            "pay/charge.yaml",
            EntryKind::Request,
            "staging",
            &Redactor::default(),
        );
        assert_eq!(e.id.len(), 8);
        assert_eq!((e.passed, e.total, e.retries), (1, 1, 2));
        assert_eq!(e.duration_ms, 42.0);
        let r = &e.results[0];
        assert_eq!(r.file, PathBuf::from("pay/charge.yaml"));
        assert_eq!((r.name.as_str(), r.method.as_str()), ("Charge", "POST"));
        assert_eq!((r.status, r.attempts, r.duration_ms), (Some(200), 3, 42.0));
        assert_eq!(r.body.as_deref(), Some(r#"{"ok":true}"#));
        let again = entry_from_outcome(
            &o,
            "pay/charge.yaml",
            EntryKind::Request,
            "staging",
            &Redactor::default(),
        );
        assert_ne!(e.id, again.id);
    }

    #[test]
    fn compare_classifies() {
        let prev = entry(
            "p",
            at(1, 9),
            vec![
                stored("breaks", true, 10.0),
                stored("heals", false, 10.0),
                stored("slow", true, 109.0),
                stored("mild", true, 100.0),
                stored("tiny", true, 20.0),
            ],
        );
        let cur = entry(
            "c",
            at(2, 9),
            vec![
                stored("breaks", false, 10.0),
                stored("heals", true, 10.0),
                stored("slow", true, 212.0),
                stored("mild", true, 120.0),
                stored("tiny", true, 40.0),
                stored("brand new", true, 1.0),
            ],
        );
        let c = compare(&cur, &prev);
        assert_eq!(c.newly_failing.len(), 1);
        assert_eq!(c.newly_failing[0].name, "breaks");
        assert_eq!(c.fixed.len(), 1);
        assert_eq!(c.fixed[0].name, "heals");
        assert_eq!(c.slower.len(), 1);
        assert_eq!(
            (c.slower[0].0.name.as_str(), c.slower[0].1),
            ("slow", 109.0)
        );
        assert_eq!(c.unchanged, 2);
    }

    #[test]
    fn previous_and_series() {
        let mut other = entry("o", at(2, 12), vec![]);
        other.environment = "prod".into();
        let mut list = vec![
            entry("c", at(3, 9), vec![]),
            other,
            entry("b", at(2, 9), vec![]),
            entry("a", at(1, 9), vec![]),
        ];
        list[0].duration_ms = 30.0;
        list[2].duration_ms = 20.0;
        list[3].duration_ms = 10.0;
        let cur = list[0].clone();
        assert_eq!(previous_for(&list, &cur).unwrap().id, "b");
        assert!(previous_for(&list, &list[3]).is_none());
        assert_eq!(duration_series(&list, &cur, 12), [10.0, 20.0, 30.0]);
        assert_eq!(duration_series(&list, &cur, 2), [20.0, 30.0]);
        // Runs after the picked one are not in its series.
        assert_eq!(duration_series(&list, &list[2], 12), [10.0, 20.0]);
    }

    #[test]
    fn filter_failed_only() {
        let ok = entry("a", at(1, 9), vec![]);
        let mut bad = entry("b", at(2, 9), vec![]);
        bad.passed = 0;
        let mut req = entry("c", at(3, 9), vec![]);
        req.kind = EntryKind::Request;
        req.environment = "prod".into();
        let list = [ok, bad, req];
        let only = |kind, environment: Option<&str>| {
            filter_entries(
                &list,
                &Filter {
                    kind,
                    environment: environment.map(Into::into),
                },
            )
        };
        assert_eq!(only(KindFilter::Failed, None), [1]);
        assert_eq!(only(KindFilter::All, None), [0, 1, 2]);
        assert_eq!(only(KindFilter::CollectionRuns, None), [0, 1]);
        assert_eq!(only(KindFilter::Requests, None), [2]);
        assert_eq!(only(KindFilter::All, Some("staging")), [0, 1]);
        assert!(only(KindFilter::Failed, Some("prod")).is_empty());
    }

    #[test]
    fn day_buckets() {
        let now = Local.with_ymd_and_hms(2026, 10, 30, 0, 30, 0).unwrap();
        assert_eq!(day_bucket(at(30, 0), now), "Today");
        assert_eq!(day_bucket(at(29, 23), now), "Yesterday");
        assert_eq!(day_bucket(at(29, 0), now), "Yesterday");
        assert_eq!(day_bucket(at(28, 23), now), "Oct 28");
        assert_eq!(day_bucket(at(7, 9), now), "Oct 7");
        let jan = Local.with_ymd_and_hms(2027, 1, 1, 8, 0, 0).unwrap();
        let eve = Local.with_ymd_and_hms(2026, 12, 31, 22, 0, 0).unwrap();
        assert_eq!(day_bucket(eve, jan), "Yesterday");
    }

    #[test]
    fn old_entries_without_a_version_still_load() {
        let (_d, h) = history();
        let mut v = serde_json::to_value(entry("aaaa0001", at(1, 9), vec![])).unwrap();
        v.as_object_mut().unwrap().remove("version");
        std::fs::create_dir_all(&h.dir).unwrap();
        std::fs::write(h.dir.join("20261001T090000-aaaa0001.json"), v.to_string()).unwrap();
        assert_eq!(h.list()[0].version, "");
    }

    #[test]
    fn target_keys_round_trip() {
        for t in [
            RunTarget::Collection,
            RunTarget::Files(vec!["a.yaml".into(), "b/c.yaml".into()]),
            RunTarget::Files(vec!["x, y.yaml".into()]),
            RunTarget::File("charges/get-charge.yaml".into()),
            RunTarget::Request {
                file: "charges/list.yaml".into(),
                index: 2,
            },
        ] {
            let (key, kind) = target_key(&t);
            let mut e = entry("a", at(1, 9), vec![]);
            (e.target, e.kind) = (key, kind);
            assert_eq!(target_of(&e), t);
        }
        // Entries from before the newline form.
        let mut old = entry("a", at(1, 9), vec![]);
        old.target = "a.yaml, b.yaml".into();
        assert_eq!(
            target_of(&old),
            RunTarget::Files(vec!["a.yaml".into(), "b.yaml".into()])
        );
        assert_eq!(
            target_key(&RunTarget::Request {
                file: "a.yaml".into(),
                index: 1
            }),
            ("a.yaml#1".to_string(), EntryKind::Request)
        );
    }
}
