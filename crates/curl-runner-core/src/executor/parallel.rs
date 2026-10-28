//! Parallel runner (TS `executeParallel` / `executePooled`) on one
//! `curl::multi::Multi`, driven by `perform` + `wait`.
//!
//! - At most `maxConcurrency` requests are live (in flight or waiting for a
//!   retry); unset or 0 means all at once. Requests start in order.
//! - Retries never block: a request waiting for its retry delay leaves the
//!   multi handle and is re-added once its deadline passes.
//! - With `connectionPool.enabled`, `maxStreamsPerHost` (default 10, as TS)
//!   becomes `CURLMOPT_MAX_HOST_CONNECTIONS`; HTTP/2 multiplexing is always on.
//! - With a limit below the request count, a failure without
//!   `continueOnError` stops launching new requests (the ones already started
//!   finish and are reported). Without a limit every request runs, like TS
//!   `Promise.all`.
//! - Errors building a transfer (missing formData file, guard) fail the
//!   request at once, without retries (TS checked them before sending).
//! - As in TS, neither `when` nor the response store is used in this mode.

use super::retry::{Attempt, RetryPolicy, deadline, delay_duration};
use super::{EventSink, Executor, PreparedRequest, untyped_failure};
use crate::model::{ExecutionResult, RequestConfig};
use crate::transport::{self, Collector, TransportError};
use curl::easy::Easy2;
use curl::multi::{Easy2Handle, Multi};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default `maxStreamsPerHost` of the TS pooled executor.
const DEFAULT_MAX_STREAMS: usize = 10;
/// Upper bound for one `Multi::wait` (cancel / deadline latency).
const WAIT_SLICE: Duration = Duration::from_millis(20);

struct Live {
    index: usize,
    req: RequestConfig,
    policy: RetryPolicy,
    start: Instant,
    /// Attempts finished so far.
    done: u32,
    /// `Some(deadline)` while waiting for a retry, `None` while in flight.
    retry_at: Option<Instant>,
}

struct Runner<'a> {
    ex: &'a Executor,
    sink: &'a mut dyn EventSink,
    multi: Multi,
    live: HashMap<usize, Live>,
    handles: HashMap<usize, Easy2Handle<Collector>>,
    results: Vec<Option<ExecutionResult>>,
    /// A failure may stop the run (a limit below the request count).
    bounded: bool,
    stop: bool,
}

pub(super) fn run(
    ex: &Executor,
    requests: Vec<PreparedRequest>,
    sink: &mut dyn EventSink,
) -> Vec<ExecutionResult> {
    let n = requests.len();
    let max = match ex.global.max_concurrency {
        Some(m) if m > 0 => usize::try_from(m).unwrap_or(usize::MAX),
        _ => usize::MAX,
    };
    let mut r = Runner {
        ex,
        sink,
        multi: new_multi(ex),
        live: HashMap::new(),
        handles: HashMap::new(),
        results: (0..n).map(|_| None).collect(),
        bounded: max < n,
        stop: false,
    };
    let mut pending = requests.into_iter().enumerate();

    loop {
        if ex.cancelled() {
            r.drop_in_flight();
            break;
        }
        while !r.stop && r.live.len() < max {
            let Some((i, p)) = pending.next() else { break };
            r.launch(i, p);
            if ex.cancelled() {
                break;
            }
        }
        r.readd_due();
        if r.live.is_empty() && (r.stop || pending.len() == 0) {
            break;
        }
        r.drive();
    }
    r.results.into_iter().flatten().collect()
}

pub(super) fn new_multi(ex: &Executor) -> Multi {
    let mut multi = Multi::new();
    // Option failures only lose an optimisation; transfers still work.
    let _ = multi.pipelining(false, true);
    if let Some(pool) = ex
        .global
        .connection_pool
        .as_ref()
        .filter(|p| p.enabled == Some(true))
    {
        let streams = pool
            .max_streams_per_host
            .filter(|s| *s > 0)
            .map_or(DEFAULT_MAX_STREAMS, |s| {
                usize::try_from(s).unwrap_or(usize::MAX)
            });
        let _ = multi.set_max_host_connections(streams);
    }
    multi
}

impl Runner<'_> {
    /// Type, preflight and start request `i` (or record its early result).
    fn launch(&mut self, i: usize, p: PreparedRequest) {
        let start = Instant::now();
        let index = i + 1;
        let req = match self.ex.type_request(&p.raw, None) {
            Ok(r) => r,
            Err(e) => {
                let res = untyped_failure(&p.raw, e, start);
                self.record(i, res);
                return;
            }
        };
        if let Some(res) = self.ex.preflight(index, &req, start, self.sink) {
            self.record(i, res);
            return;
        }
        self.live.insert(
            i,
            Live {
                index,
                policy: RetryPolicy::for_request(&req),
                req,
                start,
                done: 0,
                retry_at: None,
            },
        );
        self.add(i);
    }

    /// Put request `i` on the multi handle. A build failure ends the
    /// request (no retry); an add failure counts as a failed attempt.
    fn add(&mut self, i: usize) {
        let l = &self.live[&i];
        let easy = match transport::build_easy(&l.req, &self.ex.global, &self.ex.cwd) {
            Ok(e) => e,
            Err(e) => {
                let l = self.live.remove(&i).expect("live entry");
                let (res, _) = self.ex.complete(&l.req, &l.policy, Err(e), l.start);
                self.record(i, res);
                return;
            }
        };
        let added = self.multi.add2(easy).map_err(multi_err).and_then(|mut h| {
            h.set_token(i).map_err(|e| TransportError {
                code: None,
                message: e.to_string(),
            })?;
            Ok(h)
        });
        match added {
            Ok(h) => {
                self.handles.insert(i, h);
            }
            Err(e) => self.attempt_done(i, Err(e)),
        }
    }

    /// Re-add requests whose retry deadline has passed.
    fn readd_due(&mut self) {
        let now = Instant::now();
        let due: Vec<usize> = self
            .live
            .iter()
            .filter(|(_, l)| l.retry_at.is_some_and(|t| t <= now))
            .map(|(i, _)| *i)
            .collect();
        for i in due {
            if let Some(l) = self.live.get_mut(&i) {
                l.retry_at = None;
            }
            self.add(i);
        }
    }

    /// One `perform` + completed-message pass + bounded `wait`.
    fn drive(&mut self) {
        if !self.handles.is_empty() {
            if let Err(e) = self.multi.perform() {
                let msg = multi_err(e).message;
                for i in self.handles.keys().copied().collect::<Vec<_>>() {
                    self.finish_handle(i, None, Some(msg.clone()));
                }
                return;
            }
            let mut finished = Vec::new();
            self.multi.messages(|m| {
                if let Ok(tok) = m.token()
                    && let Some(h) = self.handles.get(&tok)
                    && let Some(res) = m.result_for2(h)
                {
                    finished.push((tok, res));
                }
            });
            for (i, res) in finished {
                self.finish_handle(i, Some(res), None);
            }
        }
        let wait = self.next_deadline().map_or(WAIT_SLICE, |t| {
            t.saturating_duration_since(Instant::now()).min(WAIT_SLICE)
        });
        if self.handles.is_empty() {
            std::thread::sleep(wait);
        } else {
            let _ = self.multi.wait(&mut [], wait);
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.live.values().filter_map(|l| l.retry_at).min()
    }

    /// Remove a finished transfer and turn it into an attempt outcome.
    fn finish_handle(
        &mut self,
        i: usize,
        res: Option<Result<(), curl::Error>>,
        error: Option<String>,
    ) {
        let Some(h) = self.handles.remove(&i) else {
            return;
        };
        let easy = self.multi.remove2(h).map_err(multi_err);
        let attempt = match res {
            Some(res) => outcome(easy, res, &self.live[&i].req, self.ex),
            None => Err(TransportError {
                code: None,
                message: error.unwrap_or_default(),
            }),
        };
        self.attempt_done(i, attempt);
    }

    /// Schedule a retry or finalize request `i`.
    fn attempt_done(&mut self, i: usize, attempt: Attempt) {
        let Some(l) = self.live.get_mut(&i) else {
            return;
        };
        l.done += 1;
        if l.policy.wants_another(l.done, &attempt) {
            let delay = l.policy.delay_ms(l.done, &attempt);
            self.sink
                .on_retry(l.index, &l.req, l.done, delay.ceil() as u64);
            l.retry_at = Some(deadline(Instant::now(), delay_duration(delay)));
            return;
        }
        let l = self.live.remove(&i).expect("live entry checked above");
        let (res, _) = self.ex.complete(&l.req, &l.policy, attempt, l.start);
        self.record(i, res);
    }

    fn record(&mut self, i: usize, res: ExecutionResult) {
        self.sink.on_result(i + 1, &res);
        if self.bounded && !res.success && !self.ex.continue_on_error() && !self.stop {
            self.stop = true;
            self.sink.on_stop();
        }
        self.results[i] = Some(res);
    }

    /// Cancellation: detach and drop every in-flight transfer.
    fn drop_in_flight(&mut self) {
        for (_, h) in self.handles.drain() {
            let _ = self.multi.remove2(h);
        }
        self.live.clear();
    }
}

fn multi_err(e: curl::MultiError) -> TransportError {
    TransportError {
        code: None,
        message: e.to_string(),
    }
}

/// Turn a finished transfer into an attempt outcome.
fn outcome(
    easy: Result<Easy2<Collector>, TransportError>,
    res: Result<(), curl::Error>,
    req: &RequestConfig,
    ex: &Executor,
) -> Attempt {
    let easy = easy?;
    match res {
        Ok(()) => transport::finish(easy),
        Err(e) => Err(match easy.get_ref().output_error() {
            Some(msg) => TransportError {
                code: Some(transport::curl_code(&e)),
                message: msg.to_string(),
            },
            None => TransportError::from_curl(&e, req, &ex.global),
        }),
    }
}

/// Why [`send_one`] produced no attempt: `Some(error)` when the transfer
/// could not be built (guard, missing file; not retried), `None` when the
/// run was cancelled and the transfer dropped.
pub(super) type NoAttempt = Option<TransportError>;

/// Run one transfer on `multi`, polling the cancel flag every `WAIT_SLICE`
/// (the sequential runner's `send`; `multi` is reused across requests so
/// connections are kept alive).
pub(super) fn send_one(
    ex: &Executor,
    multi: &Multi,
    req: &RequestConfig,
) -> Result<Attempt, NoAttempt> {
    let easy = match transport::build_easy(req, &ex.global, &ex.cwd) {
        Ok(e) => e,
        Err(e) => return Err(Some(e)),
    };
    let h = match multi.add2(easy) {
        Ok(h) => h,
        Err(e) => return Ok(Err(multi_err(e))),
    };
    loop {
        if ex.cancelled() {
            let _ = multi.remove2(h);
            return Err(None);
        }
        if let Err(e) = multi.perform() {
            let _ = multi.remove2(h);
            return Ok(Err(multi_err(e)));
        }
        let mut res = None;
        multi.messages(|m| {
            if let Some(r) = m.result_for2(&h) {
                res = Some(r);
            }
        });
        if let Some(res) = res {
            let easy = multi.remove2(h).map_err(multi_err);
            return Ok(outcome(easy, res, req, ex));
        }
        let _ = multi.wait(&mut [], WAIT_SLICE);
    }
}
