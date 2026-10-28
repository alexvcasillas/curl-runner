//! Raw format (`--output-format raw`): only successful response bodies.

use super::format::{format_json, truthy};
use super::{Console, Logger};
use curl_runner_core::model::ExecutionResult;

impl Logger {
    pub(super) fn raw_result(&self, c: &mut Console, r: &ExecutionResult) {
        if r.success
            && self.cfg.show_body
            && truthy(r.body.as_ref())
            && let Some(body) = &r.body
        {
            c.out(&self.secrets(&format_json(body, "raw")));
        }
    }
}
