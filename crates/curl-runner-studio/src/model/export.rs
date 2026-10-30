//! Run reports: JSON (byte-identical to core's `save_summary`), JUnit XML,
//! a Markdown summary and the equivalent CLI command. Pure text, no GPUI.
//! Every string that reaches a report goes through the run's [`Redactor`].

use crate::model::environments::EnvCtx;
use crate::model::run::RunTarget;
use curl_runner_core::model::{ExecutionResult, ExecutionSummary};
use curl_runner_core::security::redact::Redactor;
use curl_runner_core::snapshot::json::js_stringify_pretty;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::path::{Path, PathBuf};

/// The content core's `save_summary` writes: `JSON.stringify(summary, null,
/// 2)` text, redacted.
pub fn to_json(s: &ExecutionSummary, r: &Redactor) -> String {
    let value = serde_json::to_value(s).unwrap_or(serde_json::Value::Null);
    r.redact(&js_stringify_pretty(&value, 2))
}

fn is_skipped(res: &ExecutionResult) -> bool {
    res.skipped == Some(true)
}

fn duration_ms(res: &ExecutionResult) -> f64 {
    res.metrics.as_ref().map_or(0.0, |m| m.duration)
}

fn method(res: &ExecutionResult) -> &str {
    res.request.method.as_deref().unwrap_or("GET")
}

/// The request name, or `METHOD url`.
fn title(res: &ExecutionResult) -> String {
    match res.request.name.as_deref().filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None => format!("{} {}", method(res), res.request.url),
    }
}

fn failure_text(res: &ExecutionResult) -> String {
    res.error
        .clone()
        .filter(|e| !e.is_empty())
        .unwrap_or_else(|| "request failed".to_string())
}

fn seconds(ms: f64) -> String {
    format!("{:.3}", ms / 1000.0)
}

/// `groups` partitions `summary.results` in order (the `RunEvent::Started`
/// groups); results beyond the groups land in a trailing `requests` suite.
pub fn to_junit(s: &ExecutionSummary, groups: &[(PathBuf, usize)], r: &Redactor) -> String {
    let mut suites: Vec<(String, &[ExecutionResult])> = Vec::new();
    let mut at = 0;
    for (file, n) in groups {
        let end = (at + n).min(s.results.len());
        suites.push((file.to_string_lossy().into_owned(), &s.results[at..end]));
        at = end;
    }
    if at < s.results.len() {
        suites.push(("requests".to_string(), &s.results[at..]));
    }

    // Counted here, not from the summary: core's `failed` also counts a
    // skipped result whose `success` is false.
    let failed = s
        .results
        .iter()
        .filter(|x| !x.success && !is_skipped(x))
        .count();
    let skipped = s.results.iter().filter(|x| is_skipped(x)).count();

    let mut w = Writer::new_with_indent(Vec::new(), b' ', 2);
    let _ = w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)));
    let mut root = BytesStart::new("testsuites");
    root.push_attribute(("name", "curl-runner"));
    root.push_attribute(("tests", s.results.len().to_string().as_str()));
    root.push_attribute(("failures", failed.to_string().as_str()));
    root.push_attribute(("skipped", skipped.to_string().as_str()));
    root.push_attribute(("time", seconds(s.duration).as_str()));
    let _ = w.write_event(Event::Start(root));
    for (file, results) in &suites {
        let file = r.redact(file);
        let failures = results
            .iter()
            .filter(|x| !x.success && !is_skipped(x))
            .count();
        let skipped = results.iter().filter(|x| is_skipped(x)).count();
        let time: f64 = results.iter().map(duration_ms).sum();
        let mut suite = BytesStart::new("testsuite");
        suite.push_attribute(("name", file.as_str()));
        suite.push_attribute(("tests", results.len().to_string().as_str()));
        suite.push_attribute(("failures", failures.to_string().as_str()));
        suite.push_attribute(("skipped", skipped.to_string().as_str()));
        suite.push_attribute(("time", seconds(time).as_str()));
        let _ = w.write_event(Event::Start(suite));
        for res in *results {
            let name = r.redact(&title(res));
            let mut case = BytesStart::new("testcase");
            case.push_attribute(("name", name.as_str()));
            case.push_attribute(("classname", file.as_str()));
            case.push_attribute(("time", seconds(duration_ms(res)).as_str()));
            if is_skipped(res) {
                let _ = w.write_event(Event::Start(case));
                let _ = w.write_event(Event::Empty(BytesStart::new("skipped")));
                let _ = w.write_event(Event::End(BytesEnd::new("testcase")));
            } else if !res.success {
                let msg = r.redact(&failure_text(res));
                let _ = w.write_event(Event::Start(case));
                let mut f = BytesStart::new("failure");
                f.push_attribute(("message", msg.as_str()));
                let _ = w.write_event(Event::Start(f));
                let _ = w.write_event(Event::Text(BytesText::new(&msg)));
                let _ = w.write_event(Event::End(BytesEnd::new("failure")));
                let _ = w.write_event(Event::End(BytesEnd::new("testcase")));
            } else {
                let _ = w.write_event(Event::Empty(case));
            }
        }
        let _ = w.write_event(Event::End(BytesEnd::new("testsuite")));
    }
    let _ = w.write_event(Event::End(BytesEnd::new("testsuites")));
    let mut out = String::from_utf8(w.into_inner()).unwrap_or_default();
    out.push('\n');
    out
}

/// `184 ms`, or `4.8 s` from 1000 ms.
fn human_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.1} s", ms / 1000.0)
    } else {
        format!("{} ms", ms.round() as i64)
    }
}

/// Table cells must not break the row.
fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\n', '\r'], " ")
}

pub fn to_markdown(s: &ExecutionSummary, title_text: &str, env: &str, r: &Redactor) -> String {
    let failed: Vec<_> = s
        .results
        .iter()
        .filter(|x| !x.success && !is_skipped(x))
        .collect();
    let mut out = format!("# {}\n\n", r.redact(title_text));
    out.push_str(&format!(
        "Environment: {} · Passed {} · Failed {} · Duration {}\n\n",
        r.redact(env),
        s.successful,
        failed.len(),
        human_ms(s.duration)
    ));
    out.push_str("| Method | Request | Status | Duration | Result |\n|---|---|---|---|---|\n");
    for res in &s.results {
        let status = res.status.map_or("—".to_string(), |c| c.to_string());
        let verdict = if is_skipped(res) {
            "skipped"
        } else if res.success {
            "✓"
        } else {
            "✗"
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            method(res),
            cell(&r.redact(&title(res))),
            status,
            human_ms(duration_ms(res)),
            verdict
        ));
    }
    if !failed.is_empty() {
        out.push_str("\n### Failures\n\n");
        for res in failed {
            let msg = r.redact(&failure_text(res)).replace('\n', " ");
            out.push_str(&format!("- **{}**: {}\n", r.redact(&title(res)), msg));
        }
    }
    r.redact(&out)
}

/// Single-quote a path for the shell when it needs it.
fn quote(p: &str) -> String {
    let plain = !p.is_empty()
        && p.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+=:@,%".contains(c));
    if plain {
        p.to_string()
    } else {
        format!("'{}'", p.replace('\'', "'\\''"))
    }
}

/// The `curl-runner` command that repeats the run, for the CLI's working
/// directory = the collection root (paths are relative, no `cd` is
/// included). `Collection` gives no paths: the CLI discovers YAML files in
/// the working directory. `Request` can only name its file (the CLI has no
/// per-request flag). `needed_secrets` are names only, never values, in a
/// leading comment.
pub fn cli_command(target: &RunTarget, ctx: &EnvCtx, needed_secrets: &[String]) -> String {
    let paths: Vec<&Path> = match target {
        RunTarget::File(f) | RunTarget::Request { file: f, .. } => vec![f.as_path()],
        RunTarget::Collection => Vec::new(),
        RunTarget::Files(fs) => fs.iter().map(PathBuf::as_path).collect(),
    };
    let mut parts = vec!["curl-runner".to_string()];
    parts.extend(paths.iter().map(|p| quote(&p.to_string_lossy())));
    if let Some(env) = ctx.env_name() {
        parts.push("--env".to_string());
        parts.push(quote(env));
    }
    let cmd = parts.join(" ");
    if needed_secrets.is_empty() {
        cmd
    } else {
        format!(
            "# set {} (keychain) before running\n{cmd}",
            needed_secrets.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::executor::{create_summary, save_summary};
    use curl_runner_core::model::{GlobalConfig, Json, OutputConfig, RequestConfig};
    use quick_xml::Reader;

    fn res(name: &str, success: bool, status: Option<u16>) -> ExecutionResult {
        let req = RequestConfig {
            name: Some(name.into()),
            url: "https://api.example.com/x".into(),
            method: Some("GET".into()),
            ..Default::default()
        };
        let mut r = ExecutionResult::new(req, success);
        r.status = status;
        r
    }

    fn summary() -> ExecutionSummary {
        let ok = res("List charges", true, Some(200));
        let mut bad = res("Create <charge> & more", false, Some(500));
        bad.error = Some("boom with hunter2 \"quoted\"".into());
        bad.body = Some(Json::String("body hunter2".into()));
        let mut skip = res("Refund", false, None);
        skip.skipped = Some(true);
        create_summary(vec![ok, bad, skip], 4800.0)
    }

    fn redactor() -> Redactor {
        Redactor::new(["hunter2".to_string()])
    }

    fn groups() -> Vec<(PathBuf, usize)> {
        vec![(PathBuf::from("payments/charges.yaml"), 3)]
    }

    #[test]
    fn json_matches_core_save_summary() {
        let dir = tempfile::tempdir().unwrap();
        let s = summary();
        let global = GlobalConfig {
            output: Some(OutputConfig {
                save_to_file: Some("out.json".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let r = redactor();
        let path = save_summary(&s, &global, dir.path(), Some(&r))
            .unwrap()
            .unwrap();
        let written = std::fs::read_to_string(path).unwrap();
        assert_eq!(to_json(&s, &r), written);
    }

    #[test]
    fn junit_is_well_formed_and_counts() {
        let xml = to_junit(&summary(), &groups(), &redactor());
        let mut reader = Reader::from_str(&xml);
        let (mut suites, mut cases, mut failures, mut skips) = (0, 0, 0, 0);
        let mut suite_attrs = String::new();
        loop {
            match reader.read_event().unwrap() {
                Event::Start(e) | Event::Empty(e) => match e.name().as_ref() {
                    b"testsuite" => {
                        suites += 1;
                        suite_attrs = String::from_utf8_lossy(&e).into_owned();
                    }
                    b"testcase" => cases += 1,
                    b"failure" => failures += 1,
                    b"skipped" => skips += 1,
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!((suites, cases, failures, skips), (1, 3, 1, 1));
        assert!(suite_attrs.contains(r#"tests="3" failures="1" skipped="1""#));
        assert!(xml.contains(r#"name="Create &lt;charge&gt; &amp; more""#));
    }

    #[test]
    fn exports_redact_secrets() {
        let s = summary();
        let r = redactor();
        for out in [
            to_json(&s, &r),
            to_junit(&s, &groups(), &r),
            to_markdown(&s, "Charges", "staging", &r),
        ] {
            assert!(!out.contains("hunter2"), "leaked in: {out}");
        }
    }

    #[test]
    fn markdown_table_and_failures() {
        let md = to_markdown(&summary(), "Charges", "staging", &redactor());
        assert!(md.starts_with("# Charges\n"));
        assert!(md.contains("Passed 1 · Failed 1 · Duration 4.8 s"));
        assert!(md.contains("| GET | List charges | 200 | 0 ms | ✓ |"));
        assert!(md.contains("| — | skipped") || md.contains("| — | 0 ms | skipped |"));
        assert!(md.contains("### Failures"));
    }

    fn ctx(env: &str) -> EnvCtx {
        EnvCtx {
            root: PathBuf::from("/work/acme"),
            collection: "acme".into(),
            environment: env.into(),
            marked_secret: vec![],
        }
    }

    #[test]
    fn cli_command_lists_env_and_secrets() {
        let t = RunTarget::Files(vec!["a.yaml".into(), "my dir/b.yaml".into()]);
        assert_eq!(
            cli_command(&t, &ctx("staging"), &["API_TOKEN".into()]),
            "# set API_TOKEN (keychain) before running\ncurl-runner a.yaml 'my dir/b.yaml' --env staging"
        );
        assert_eq!(
            cli_command(&RunTarget::Collection, &ctx("Default"), &[]),
            "curl-runner"
        );
    }
}
