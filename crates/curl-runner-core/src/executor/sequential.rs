//! Sequential runner (TS `executeSequential`): response store, `when`,
//! blocking retries, stop on first failure unless `continueOnError`.
//! Transfers run on a single-handle `Multi` loop so a cancel interrupts an
//! in-flight request within ~20 ms.

use super::parallel;
use super::retry::{RetryPolicy, deadline, delay_duration};
use super::{EventSink, Executor, PreparedRequest, untyped_failure};
use crate::interpolate::Store;
use crate::model::{ExecutionResult, RequestConfig};
use crate::{condition, store, transport};
use curl::multi::Multi;
use std::time::{Duration, Instant};

/// Longest uninterrupted sleep while waiting for a retry (cancel latency).
const SLEEP_SLICE: Duration = Duration::from_millis(20);

pub(super) fn run(
    ex: &Executor,
    requests: Vec<PreparedRequest>,
    sink: &mut dyn EventSink,
) -> Vec<ExecutionResult> {
    let mut results = Vec::with_capacity(requests.len());
    let multi = parallel::new_multi(ex);
    let mut values = Store::new();

    for (i, prepared) in requests.into_iter().enumerate() {
        if ex.cancelled() {
            break;
        }
        let index = i + 1;
        let start = Instant::now();
        let req = match ex.type_request(&prepared.raw, Some(&values)) {
            Ok(r) => r,
            Err(e) => {
                let r = untyped_failure(&prepared.raw, e, start);
                sink.on_result(index, &r);
                results.push(r);
                if !ex.continue_on_error() {
                    sink.on_stop();
                    break;
                }
                continue;
            }
        };

        if let Some(when) = &req.when {
            let outcome = condition::evaluate(when, &values);
            if !outcome.run {
                let mut r = ExecutionResult::new(req, true);
                r.skipped = Some(true);
                r.skip_reason = outcome.reason;
                sink.on_result(index, &r);
                results.push(r);
                continue;
            }
        }

        let Some((result, resp)) = execute(ex, &multi, index, &req, start, sink) else {
            break; // cancelled mid-request: not reported
        };

        // TS logs the result, then the stored values.
        let ok = result.success;
        sink.on_result(index, &result);
        if let (Some(resp), Some(cfg)) = (&resp, &req.store) {
            let mut new = Store::new();
            store::extract(cfg, resp, &mut new);
            if !new.is_empty() {
                sink.on_stored(&new);
            }
            values.extend(new);
        }
        results.push(result);
        if !ok && !ex.continue_on_error() {
            sink.on_stop();
            break;
        }
    }
    results
}

/// One request through preflight and the retry loop. `None` when cancelled.
fn execute(
    ex: &Executor,
    multi: &Multi,
    index: usize,
    req: &RequestConfig,
    start: Instant,
    sink: &mut dyn EventSink,
) -> Option<(ExecutionResult, Option<transport::Response>)> {
    if let Some(r) = ex.preflight(index, req, start, sink) {
        return Some((r, None));
    }
    let policy = RetryPolicy::for_request(req);
    let attempt = || parallel::send_one(ex, multi, req);
    let mut last = match attempt() {
        Ok(a) => a,
        // Build errors fail at once, without retries (TS pre-checks).
        Err(Some(e)) => return Some(ex.complete(req, &policy, Err(e), start)),
        Err(None) => return None,
    };
    let mut done = 1u32;
    while policy.wants_another(done, &last) {
        let delay = policy.delay_ms(done, &last);
        sink.on_retry(index, req, done, delay.ceil() as u64);
        if !sleep_unless_cancelled(ex, delay) {
            return None;
        }
        last = match attempt() {
            Ok(a) => a,
            Err(Some(e)) => return Some(ex.complete(req, &policy, Err(e), start)),
            Err(None) => return None,
        };
        done += 1;
    }
    Some(ex.complete(req, &policy, last, start))
}

/// Sleep `ms` in slices; `false` if the run was cancelled meanwhile.
fn sleep_unless_cancelled(ex: &Executor, ms: f64) -> bool {
    let until = deadline(Instant::now(), delay_duration(ms));
    loop {
        if ex.cancelled() {
            return false;
        }
        let now = Instant::now();
        if now >= until {
            return true;
        }
        std::thread::sleep((until - now).min(SLEEP_SLICE));
    }
}
