//! Pretty format (`Logger.logRequestComplete` / `logSummary` / `logSkipped`).

use super::format::{format_duration, format_json, format_size, short_filename, truthy};
use super::snapshot::SnapshotFormatter;
use super::tree::{TreeNode, render};
use super::{Console, Logger};
use crate::colors::Color;
use curl_runner_core::model::{ExecutionResult, ExecutionSummary, HeaderValue, RequestConfig};
use regex::Regex;
use std::sync::LazyLock;

static STATUS_ERR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Expected status (.+?), got (.+)$").unwrap());
static FIELD_ERR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Expected (.+?) to be (.+?), got (.+)$").unwrap());

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

fn method(req: &RequestConfig) -> &str {
    non_empty(req.method.as_deref()).unwrap_or("GET")
}

fn plural(n: u64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

impl Logger {
    fn render(&self, c: &mut Console, nodes: &[TreeNode]) {
        for line in render(nodes, self.color) {
            c.out(&line);
        }
    }

    pub(super) fn pretty_result(&self, c: &mut Console, r: &ExecutionResult) {
        let ok = r.success;
        let status_color = if ok { Color::Green } else { Color::Red };
        let icon = if ok { "✓" } else { "x" };
        let name = self.secrets(non_empty(r.request.name.as_deref()).unwrap_or("Request"));
        let url = self.redact(&r.request.url);
        let dry = r.dry_run == Some(true);
        let status = match r.status {
            _ if dry => "DRY-RUN".to_string(),
            Some(s) if s != 0 => s.to_string(),
            _ => "ERROR".to_string(),
        };
        let status_node_color = if dry { Color::Cyan } else { status_color };

        let mut nodes = Vec::new();
        if self.level() == "minimal" {
            let tag = r
                .request
                .source_file
                .as_deref()
                .filter(|s| !s.is_empty())
                .map_or_else(|| "inline".to_string(), short_filename);
            c.out(&format!(
                "{} {} [{tag}]",
                self.color(icon, status_color),
                self.color(&name, Color::Bright)
            ));
            nodes.push(TreeNode::leaf(method(&r.request), url, Some(Color::Blue)));
            nodes.push(TreeNode::leaf(
                format!("{icon} Status"),
                status,
                Some(status_node_color),
            ));
            if let Some(m) = &r.metrics {
                nodes.push(TreeNode::leaf(
                    "Duration",
                    format!("{} | {}", format_duration(m.duration), format_size(m.size)),
                    Some(Color::Cyan),
                ));
            }
            self.render(c, &nodes);
            self.result_footer(c, r);
            return;
        }

        c.out(&format!(
            "{} {}",
            self.color(icon, status_color),
            self.color(&name, Color::Bright)
        ));
        nodes.push(TreeNode::leaf("URL", url, Some(Color::Blue)));
        nodes.push(TreeNode::leaf(
            "Method",
            method(&r.request),
            Some(Color::Yellow),
        ));
        nodes.push(TreeNode::leaf("Status", status, Some(status_node_color)));
        if let Some(m) = &r.metrics {
            nodes.push(TreeNode::leaf(
                "Duration",
                format_duration(m.duration),
                Some(Color::Cyan),
            ));
        }
        if self.should_show_headers()
            && let Some(h) = r.headers.as_ref().filter(|h| !h.is_empty())
        {
            let children = h
                .iter()
                .map(|(k, v)| {
                    let v = match v {
                        HeaderValue::One(s) => s.clone(),
                        HeaderValue::Many(l) => l.join(","),
                    };
                    TreeNode::leaf(self.color(k, Color::Dim), self.secrets(&v), None)
                })
                .collect();
            nodes.push(TreeNode::section("Headers", children));
        }
        if self.should_show_body() && truthy(r.body.as_ref()) {
            let body = self.secrets(&format_json(r.body.as_ref().unwrap(), &self.cfg.format));
            let lines: Vec<&str> = body.split('\n').collect();
            let max = if self.should_show_request_details() {
                usize::MAX
            } else {
                10
            };
            let mut shown: Vec<String> = lines.iter().take(max).map(|s| s.to_string()).collect();
            if lines.len() > max {
                shown.push(self.color(
                    &format!("... ({} more lines)", lines.len() - max),
                    Color::Dim,
                ));
            }
            nodes.push(TreeNode::leaf("Response Body", shown.join("\n"), None));
        }
        if self.should_show_metrics()
            && self.level() == "detailed"
            && let Some(m) = &r.metrics
        {
            let cy = Some(Color::Cyan);
            let mut ch = vec![TreeNode::leaf(
                "Request Duration",
                format_duration(m.duration),
                cy,
            )];
            if m.size.is_some() {
                ch.push(TreeNode::leaf("Response Size", format_size(m.size), cy));
            }
            let timed = [
                ("DNS Lookup", m.dns_lookup),
                ("TCP Connection", m.tcp_connection),
                ("TLS Handshake", m.tls_handshake),
                ("Time to First Byte", m.first_byte),
            ];
            for (label, v) in timed {
                if let Some(v) = v.filter(|v| *v != 0.0 && !v.is_nan()) {
                    ch.push(TreeNode::leaf(label, format_duration(v), cy));
                }
            }
            nodes.push(TreeNode::section("Metrics", ch));
        }
        self.render(c, &nodes);
        self.result_footer(c, r);
    }

    /// Error block, snapshot result and the trailing blank line.
    fn result_footer(&self, c: &mut Console, r: &ExecutionResult) {
        if let Some(e) = non_empty(r.error.as_deref()) {
            c.out("");
            self.validation_errors(c, &self.secrets(e));
        }
        if let Some(s) = &r.snapshot_result {
            c.out("");
            let redact = |t: &str| self.secrets(t);
            let name = non_empty(r.request.name.as_deref()).unwrap_or("Request");
            c.out(&SnapshotFormatter::new(self.color, &redact).format_result(name, s));
        }
        c.out("");
    }

    fn validation_errors(&self, c: &mut Console, error: &str) {
        let errors: Vec<&str> = error.split("; ").collect();
        let red = |s: &str| self.color(s, Color::Red);
        let yellow = |s: &str| self.color(s, Color::Yellow);
        if errors.len() == 1 {
            let e = curl_runner_core::jsval::trim(errors[0]);
            if let Some(m) = STATUS_ERR.captures(e) {
                c.out(&format!(
                    "  {} {} Expected {} {}, got {}",
                    red("✗"),
                    red("Error:"),
                    yellow("status"),
                    yellow(&m[1].replacen(" or ", "|", 1)),
                    red(&m[2])
                ));
            } else {
                c.out(&format!("  {} {} {e}", red("✗"), red("Error:")));
            }
            return;
        }
        c.out(&format!("  {} {}", red("✗"), red("Validation Errors:")));
        for e in errors {
            let e = curl_runner_core::jsval::trim(e);
            if e.is_empty() {
                continue;
            }
            let line = if let Some(m) = STATUS_ERR.captures(e) {
                format!(
                    "    {} {}: expected {}, got {}",
                    red("•"),
                    yellow("status"),
                    yellow(&m[1].replacen(" or ", "|", 1)),
                    red(&m[2])
                )
            } else if let Some(m) = e
                .starts_with("Expected ")
                .then(|| FIELD_ERR.captures(e))
                .flatten()
            {
                format!(
                    "    {} {}: expected {}, got {}",
                    red("•"),
                    yellow(&m[1]),
                    self.color(&m[2], Color::Green),
                    red(&m[3])
                )
            } else {
                format!("    {} {e}", red("•"))
            };
            c.out(&line);
        }
    }

    pub(super) fn pretty_summary(&self, c: &mut Console, s: &ExecutionSummary, global: bool) {
        if global {
            c.out("");
        }
        let status_color = if s.failed == 0 {
            Color::Green
        } else {
            Color::Red
        };
        let skipped = if s.skipped > 0 {
            format!(", {} skipped", s.skipped)
        } else {
            String::new()
        };
        let text = if s.failed == 0 {
            format!(
                "{} request{} completed successfully{skipped}",
                s.total,
                plural(s.total)
            )
        } else {
            format!(
                "{}/{} request{} completed, {} failed{skipped}",
                s.successful,
                s.total,
                plural(s.total),
                s.failed
            )
        };
        let prefix = if global {
            "◆ Global Summary"
        } else {
            "Summary"
        };
        if self.level() == "minimal" {
            c.out(&format!("{prefix}: {}", self.color(&text, status_color)));
            return;
        }
        c.out("");
        c.out(&format!(
            "{prefix}: {} ({})",
            self.color(&text, status_color),
            self.color(&format_duration(s.duration), Color::Cyan)
        ));
        if s.failed > 0 && self.should_show_request_details() {
            for r in s.results.iter().filter(|r| !r.success) {
                let name = match non_empty(r.request.name.as_deref()) {
                    Some(n) => self.secrets(n),
                    None => self.redact(&r.request.url),
                };
                let err = self.secrets(r.error.as_deref().unwrap_or(""));
                c.out(&format!("  {} {name}: {err}", self.color("•", Color::Red)));
            }
        }
    }

    pub(super) fn pretty_skipped(
        &self,
        c: &mut Console,
        req: &RequestConfig,
        index: usize,
        reason: Option<&str>,
    ) {
        let name = match non_empty(req.name.as_deref()) {
            Some(n) => self.secrets(n),
            None => format!("Request {index}"),
        };
        c.out(&format!(
            "{} {} {}",
            self.color("⊘", Color::Yellow),
            self.color(&name, Color::Bright),
            self.color("[SKIP]", Color::Yellow)
        ));
        if let Some(reason) = non_empty(reason) {
            let node = TreeNode::leaf("Reason", self.secrets(reason), Some(Color::Yellow));
            self.render(c, &[node]);
        }
        c.out("");
    }
}
