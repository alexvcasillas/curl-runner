//! The first-run walkthrough: the `getting-started` collection it creates,
//! its steps, and how a run of it reads in plain words.

use super::result_view::checks;
use curl_runner_core::model::ExecutionResult;
use std::io;
use std::path::{Path, PathBuf};

/// The request the walkthrough runs.
pub const HEALTH_URL: &str = "https://api.curl-runner.com/health";

/// `getting-started/health.yaml`, as the spec writes it.
pub const HEALTH_YAML: &str = "request:
  name: curl-runner is up
  url: https://api.curl-runner.com/health
  method: GET
  expect:
    status: 200
    body:
      status: ok
";

/// The file the walkthrough opens and runs, inside [`getting_started_dir`].
pub const HEALTH_FILE: &str = "health.yaml";

/// `home/curl-runner/getting-started`.
pub fn getting_started_dir(home: &Path) -> PathBuf {
    home.join("curl-runner").join("getting-started")
}

/// Creates `dir` if missing and writes [`HEALTH_YAML`] to `health.yaml`
/// only if there is no such file. Returns the file's path.
pub fn ensure_getting_started(dir: &Path) -> io::Result<PathBuf> {
    ensure_getting_started_with(dir, HEALTH_YAML)
}

/// [`ensure_getting_started`] with `yaml` as the file a new collection
/// gets (the walkthrough pointed at another URL).
pub fn ensure_getting_started_with(dir: &Path, yaml: &str) -> io::Result<PathBuf> {
    use std::io::Write as _;
    std::fs::create_dir_all(dir)?;
    let file = dir.join(HEALTH_FILE);
    // `create_new`: an existing file, edited or not, is never replaced.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
    {
        Ok(mut f) => f.write_all(yaml.as_bytes())?,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    Ok(file)
}

/// [`HEALTH_YAML`] sending its request to `url` instead.
pub fn health_yaml(url: &str) -> String {
    HEALTH_YAML.replace(HEALTH_URL, url)
}

/// Whether the walkthrough was finished or skipped, kept between launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TourStatus {
    #[default]
    NotStarted,
    Done,
    Skipped,
}

impl TourStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TourStatus::NotStarted => "notStarted",
            TourStatus::Done => "done",
            TourStatus::Skipped => "skipped",
        }
    }

    pub fn parse(s: &str) -> Option<TourStatus> {
        [
            TourStatus::NotStarted,
            TourStatus::Done,
            TourStatus::Skipped,
        ]
        .into_iter()
        .find(|t| t.as_str() == s)
    }
}

/// The walkthrough's three steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourStep {
    Create,
    Run,
    Result,
}

impl TourStep {
    /// `1` for Create, as `STEP 1 OF 3` counts.
    pub fn number(self) -> usize {
        match self {
            TourStep::Create => 1,
            TourStep::Run => 2,
            TourStep::Result => 3,
        }
    }
}

/// How the walkthrough's run went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TourOutcome {
    Passed,
    /// No response: `host` couldn't be reached.
    Unreachable {
        host: String,
    },
    /// A response, and a check that did not hold, in plain words.
    CheckFailed(String),
    /// The file did not load or ran without a result.
    Other(String),
}

impl TourOutcome {
    /// What the result step says about it.
    pub fn message(&self) -> String {
        match self {
            TourOutcome::Passed => String::new(),
            TourOutcome::Unreachable { host } => {
                format!("Couldn't reach {host}. Check your connection and try again.")
            }
            TourOutcome::CheckFailed(m) | TourOutcome::Other(m) => m.clone(),
        }
    }
}

/// Reads the run of `health.yaml`: its first result, or why it has none.
pub fn tour_outcome(result: Option<&ExecutionResult>, load_error: Option<&str>) -> TourOutcome {
    if let Some(e) = load_error {
        return TourOutcome::Other(format!("The file didn't load: {e}"));
    }
    let Some(r) = result else {
        return TourOutcome::Other("The run stopped before the request was sent.".into());
    };
    if r.status.is_none() {
        return TourOutcome::Unreachable {
            host: host_of(&r.request.url).to_string(),
        };
    }
    let failed = checks(r).into_iter().find(|c| !c.passed);
    match failed {
        Some(c) => TourOutcome::CheckFailed(format!(
            "The {} check failed: expected {}, got {}.",
            c.label,
            c.expected,
            c.actual.as_deref().unwrap_or("nothing")
        )),
        None if r.success => TourOutcome::Passed,
        None => TourOutcome::Other(
            r.error
                .clone()
                .unwrap_or_else(|| "The request failed.".to_string()),
        ),
    }
}

/// `https://api.curl-runner.com/health` → `api.curl-runner.com`.
fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::lint::{LintOutcome, is_valid, lint_text};
    use curl_runner_core::model::RequestConfig;

    #[test]
    fn health_yaml_passes_the_cli_validator() {
        // What `curl-runner validate` runs on each file.
        match lint_text(HEALTH_YAML).unwrap() {
            LintOutcome::Linted { issues, .. } => {
                assert!(issues.is_empty(), "{issues:?}");
                assert!(is_valid(&issues));
            }
            LintOutcome::ParseError(e) => panic!("{e}"),
        }
        assert!(HEALTH_YAML.contains(HEALTH_URL));
    }

    #[test]
    fn ensure_creates_dir_and_file() {
        let home = tempfile::tempdir().unwrap();
        let dir = getting_started_dir(home.path());
        assert_eq!(dir, home.path().join("curl-runner/getting-started"));
        let file = ensure_getting_started(&dir).unwrap();
        assert_eq!(file, dir.join("health.yaml"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), HEALTH_YAML);
    }

    #[test]
    fn ensure_never_overwrites_existing_health_yaml() {
        let home = tempfile::tempdir().unwrap();
        let dir = getting_started_dir(home.path());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("health.yaml"), "edited").unwrap();
        let file = ensure_getting_started(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(file).unwrap(), "edited");
        let file = ensure_getting_started_with(&dir, "other").unwrap();
        assert_eq!(std::fs::read_to_string(file).unwrap(), "edited");
    }

    #[test]
    fn ensure_keeps_other_files_in_existing_dir() {
        let home = tempfile::tempdir().unwrap();
        let dir = getting_started_dir(home.path());
        std::fs::create_dir_all(dir.join("more")).unwrap();
        std::fs::write(dir.join("mine.yaml"), "mine").unwrap();
        std::fs::write(dir.join("more/deep.yaml"), "deep").unwrap();
        ensure_getting_started(&dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("mine.yaml")).unwrap(),
            "mine"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("more/deep.yaml")).unwrap(),
            "deep"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("health.yaml")).unwrap(),
            HEALTH_YAML
        );
    }

    #[test]
    fn health_yaml_points_anywhere() {
        let yaml = health_yaml("http://127.0.0.1:9/health");
        assert!(yaml.contains("url: http://127.0.0.1:9/health\n"));
        assert!(!yaml.contains(HEALTH_URL));
    }

    #[test]
    fn status_round_trips() {
        for s in [
            TourStatus::NotStarted,
            TourStatus::Done,
            TourStatus::Skipped,
        ] {
            assert_eq!(TourStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(TourStatus::parse("x"), None);
    }

    fn result(status: Option<u16>, body: serde_json::Value) -> ExecutionResult {
        let request: RequestConfig = serde_json::from_value(
            curl_runner_core::parser::parse_raw_stream(HEALTH_YAML).unwrap()["request"].clone(),
        )
        .unwrap();
        let mut r = ExecutionResult::new(request, true);
        r.status = status;
        r.body = Some(body);
        r
    }

    #[test]
    fn outcomes_read_in_plain_words() {
        let r = result(Some(200), serde_json::json!({"status": "ok"}));
        assert_eq!(tour_outcome(Some(&r), None), TourOutcome::Passed);

        let mut r = result(None, serde_json::Value::Null);
        r.success = false;
        r.error = Some("Failed to connect".into());
        let o = tour_outcome(Some(&r), None);
        assert_eq!(
            o.message(),
            "Couldn't reach api.curl-runner.com. Check your connection and try again."
        );

        let mut r = result(Some(503), serde_json::json!({"status": "down"}));
        r.success = false;
        assert_eq!(
            tour_outcome(Some(&r), None).message(),
            "The status check failed: expected 200, got 503."
        );

        let mut r = result(Some(200), serde_json::json!({"status": "down"}));
        r.success = false;
        let m = tour_outcome(Some(&r), None).message();
        assert!(m.starts_with("The body.status check failed"), "{m}");

        assert!(matches!(
            tour_outcome(None, Some("bad yaml")),
            TourOutcome::Other(m) if m.contains("bad yaml")
        ));
    }
}
