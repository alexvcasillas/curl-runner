//! Profile-mode console output (`Logger.logProfileStart/Result/Summary`).

use super::format::{format_latency, stringify, to_fixed};
use super::{Console, Logger};
use crate::colors::Color;
use crate::profile::ProfileOutcome;
use crate::stats;
use curl_runner_core::jsval::num_to_string;
use serde_json::json;

impl Logger {
    pub fn log_profile_start(
        &self,
        c: &mut Console,
        name: &str,
        iterations: u64,
        warmup: u64,
        concurrency: u64,
    ) {
        if !self.should_show_output() {
            return;
        }
        c.out("");
        c.out(&format!(
            "{} {}",
            self.color("⚡ PROFILING", Color::Magenta),
            self.color(&self.secrets(name), Color::Bright)
        ));
        c.out(&self.color(
            &format!("   {iterations} iterations, {warmup} warmup, concurrency: {concurrency}"),
            Color::Dim,
        ));
    }

    /// Unlike the start/summary lines this prints even with `quiet`, as in TS.
    pub fn log_profile_result(&self, c: &mut Console, r: &ProfileOutcome, histogram: bool) {
        let (s, req) = (&r.stats, &r.request);
        let name = match req.name.as_deref() {
            Some(n) if !n.is_empty() => n,
            _ => &req.url,
        };
        let name = self.secrets(name);
        if self.cfg.format == "json" {
            let v = json!({
                "request": {
                    "name": name,
                    "url": self.secrets(&req.url),
                    "method": req.method.as_deref().filter(|m| !m.is_empty()).unwrap_or("GET"),
                },
                "stats": {
                    "iterations": s.iterations,
                    "warmup": s.warmup,
                    "failures": s.failures,
                    "failureRate": s.failure_rate,
                    "min": s.min,
                    "max": s.max,
                    "mean": s.mean,
                    "median": s.median,
                    "p50": s.p50,
                    "p95": s.p95,
                    "p99": s.p99,
                    "stdDev": s.std_dev,
                },
            });
            c.out(&stringify(&v, None));
            return;
        }
        if self.cfg.format == "raw" {
            c.out(&format!(
                "{}\t{}\t{}\t{}",
                num_to_string(s.p50),
                num_to_string(s.p95),
                num_to_string(s.p99),
                num_to_string(s.mean)
            ));
            return;
        }
        c.out("");
        let icon = if s.failures == 0 {
            self.color("✓", Color::Green)
        } else {
            self.color("⚠", Color::Yellow)
        };
        c.out(&format!("{icon} {}", self.color(&name, Color::Bright)));
        c.out(&self.color("   ┌─────────────────────────────────────┐", Color::Dim));
        let row = |l: &str, lc: Color, lv: f64, r: &str, rc: Color, rv: f64, pad: &str| {
            format!(
                "   │ {}    {:>10} │ {}{pad}{:>10} │",
                self.color(l, lc),
                format_latency(lv),
                self.color(r, rc),
                format_latency(rv)
            )
        };
        c.out(&row(
            "p50",
            Color::Cyan,
            s.p50,
            "min",
            Color::Dim,
            s.min,
            "   ",
        ));
        c.out(&row(
            "p95",
            Color::Yellow,
            s.p95,
            "max",
            Color::Dim,
            s.max,
            "   ",
        ));
        c.out(&row(
            "p99",
            Color::Red,
            s.p99,
            "mean",
            Color::Dim,
            s.mean,
            "  ",
        ));
        c.out(&self.color("   └─────────────────────────────────────┘", Color::Dim));
        c.out(&self.color(
            &format!(
                "   σ {}ms | {} samples | {} failures ({}%)",
                to_fixed(s.std_dev, 2),
                s.iterations,
                s.failures,
                num_to_string(s.failure_rate)
            ),
            Color::Dim,
        ));
        if histogram && !s.timings.is_empty() {
            c.out("");
            c.out(&self.color("   Distribution:", Color::Dim));
            for line in stats::histogram(&s.timings, 8, 30) {
                c.out(&format!("   {}", self.color(&line, Color::Dim)));
            }
        }
    }

    pub fn log_profile_summary(&self, c: &mut Console, results: &[ProfileOutcome]) {
        if !self.should_show_output() {
            return;
        }
        let iterations: u64 = results.iter().map(|r| r.stats.iterations).sum();
        let failures: u64 = results.iter().map(|r| r.stats.failures).sum();
        c.out("");
        c.out(&self.color(&"─".repeat(50), Color::Dim));
        c.out(&format!(
            "{} {} request{}, {iterations} total iterations",
            self.color("⚡ Profile Summary:", Color::Magenta),
            results.len(),
            if results.len() == 1 { "" } else { "s" }
        ));
        if failures > 0 {
            c.out(&self.color(&format!("   {failures} total failures"), Color::Yellow));
        }
    }
}
