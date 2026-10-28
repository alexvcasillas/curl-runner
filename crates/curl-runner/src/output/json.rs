//! JSON format (`--output-format json`): the objects `logger.ts` prints for
//! each result, the summary and (verbose) skipped requests.

use super::Logger;
use super::format::truthy;
use curl_runner_core::model::{ExecutionResult, ExecutionSummary, Json, RequestConfig};
use serde_json::{Map, json};

/// `request: {name, url, method}`; the URL is fully redacted, as in TS.
fn request_json(l: &Logger, req: &RequestConfig) -> Json {
    let mut o = Map::new();
    if let Some(n) = &req.name {
        o.insert("name".into(), json!(n));
    }
    o.insert("url".into(), json!(l.redact(&req.url)));
    let method = req
        .method
        .as_deref()
        .filter(|m| !m.is_empty())
        .unwrap_or("GET");
    o.insert("method".into(), json!(method));
    Json::Object(o)
}

impl Logger {
    /// One result (`logRequestComplete` json branch and `logSummary` entries).
    pub(super) fn result_json(&self, r: &ExecutionResult) -> Json {
        let mut o = Map::new();
        o.insert("request".into(), request_json(self, &r.request));
        o.insert("success".into(), json!(r.success));
        if r.dry_run == Some(true) {
            o.insert("dryRun".into(), json!(true));
        } else if let Some(s) = r.status {
            o.insert("status".into(), json!(s));
        }
        if self.should_show_headers()
            && let Some(h) = &r.headers
        {
            o.insert(
                "headers".into(),
                serde_json::to_value(h).unwrap_or_default(),
            );
        }
        if self.should_show_body() && truthy(r.body.as_ref()) {
            o.insert("body".into(), r.body.clone().unwrap_or_default());
        }
        if let Some(e) = r.error.as_deref().filter(|e| !e.is_empty()) {
            o.insert("error".into(), json!(e));
        }
        if self.should_show_metrics()
            && let Some(m) = &r.metrics
        {
            o.insert(
                "metrics".into(),
                serde_json::to_value(m).unwrap_or_default(),
            );
        }
        Json::Object(o)
    }

    pub(super) fn summary_json(&self, s: &ExecutionSummary) -> Json {
        json!({
            "summary": {
                "total": s.total,
                "successful": s.successful,
                "failed": s.failed,
                "skipped": s.skipped,
                "duration": s.duration,
            },
            "results": s.results.iter().map(|r| self.result_json(r)).collect::<Vec<_>>(),
        })
    }

    pub(super) fn skipped_json(&self, req: &RequestConfig, reason: Option<&str>) -> Json {
        json!({
            "request": request_json(self, req),
            "skipped": true,
            "reason": reason.filter(|r| !r.is_empty()).unwrap_or("condition not met"),
        })
    }
}
