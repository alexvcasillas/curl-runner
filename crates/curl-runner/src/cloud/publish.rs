//! `curl-runner publish [DIR...] [--collection SLUG] [--team TEAM]
//! [--server URL] [--dry-run]`.
//!
//! This is what CI runs on every push, so it checks everything locally
//! before it sends anything (phase 1), retries what is worth retrying on its
//! own, and ends with one exit code per kind of failure (phase 2):
//! 0 ok, 1 invalid files, 2 auth, 3 plan limit, 4 wrong source, 5 unreachable.

use super::client::{Client, Reply};
use super::{Env, credentials, flag_value, server_from_args};
use crate::output::Console;
use crate::validate_cmd::lint_and_print;
use crate::version_check::{HOOKS_ENABLED, hook};
use curl_runner_cloud_proto::{
    BundleFile, ErrorBody, ErrorCode, FileChanges, GitMeta, Input, PublishRequest, PublishResponse,
    PublishStatus,
};
use curl_runner_core::cloud::{SkipReason, find_secret_literals, load_cloud_config, scan_bundle};
use curl_runner_core::config::CONFIG_FILE_PATHS;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

const EXIT_INVALID: i32 = 1;
const EXIT_AUTH: i32 = 2;
const EXIT_PLAN_LIMIT: i32 = 3;
const EXIT_WRONG_SOURCE: i32 = 4;
const EXIT_UNREACHABLE: i32 = 5;

const ATTEMPTS: usize = 5;
const BASE_DELAYS: [u64; ATTEMPTS - 1] = [1, 2, 4, 8];
const MAX_RETRY_AFTER: u64 = 60;
const NO_SLEEP_ENV: &str = "CURL_RUNNER_TEST_NO_SLEEP";
const TEAM_ENV: &str = "CURL_RUNNER_TEAM";

/// A failed publish: what to print and the exit code.
#[derive(Debug, PartialEq)]
struct Fail {
    code: i32,
    message: String,
}

fn fail(code: i32, message: impl Into<String>) -> Fail {
    Fail {
        code,
        message: message.into(),
    }
}

struct Options {
    dirs: Vec<String>,
    collection: Option<String>,
    team: Option<String>,
    dry_run: bool,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut dirs = Vec::new();
    let mut skip = false;
    for (i, a) in args.iter().enumerate() {
        if std::mem::take(&mut skip) {
            continue;
        }
        match a.as_str() {
            "--dry-run" => {}
            "--collection" | "--team" | "--server" => skip = i + 1 < args.len(),
            s if s.starts_with("--collection=")
                || s.starts_with("--team=")
                || s.starts_with("--server=") => {}
            s if s.starts_with("--") => return Err(format!("Unknown option {s}.")),
            s => dirs.push(s.to_string()),
        }
    }
    if dirs.is_empty() {
        dirs.push(".".into());
    }
    Ok(Options {
        dirs,
        collection: flag_value(args, "--collection")?,
        team: flag_value(args, "--team")?,
        dry_run: args.iter().any(|a| a == "--dry-run"),
    })
}

/// One collection, validated and ready to send.
struct Target {
    collection: String,
    name: Option<String>,
    description: Option<String>,
    inputs: Vec<Input>,
    files: Vec<BundleFile>,
    hash: String,
}

fn valid_slug(s: &str) -> bool {
    let b = s.as_bytes();
    let ok = |c: u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    (1..=63).contains(&b.len()) && ok(b[0]) && b.iter().all(|&c| ok(c) || c == b'-')
}

/// Phase 1 for one directory. Prints every problem it finds and returns the
/// target only when there are none.
fn prepare(dir: &str, slug: Option<&str>, color: bool, c: &mut Console) -> Option<Target> {
    let root = Path::new(dir);
    let ok = std::cell::Cell::new(true);
    let error = |c: &mut Console, msg: String| {
        c.err(&format!("{dir}: {msg}"));
        ok.set(false);
    };
    let config = match load_cloud_config(root) {
        Ok(cfg) => cfg,
        Err(e) => {
            error(c, e.to_string());
            return None;
        }
    };
    let (collection, name, description, inputs) = match (config, slug) {
        (Some(cfg), over) => (
            over.map(str::to_string).unwrap_or(cfg.collection),
            cfg.name,
            cfg.description,
            cfg.inputs,
        ),
        (None, Some(s)) => (s.to_string(), None, None, Vec::new()),
        (None, None) => {
            error(
                c,
                "no cloud.collection in curl-runner.yaml. Add one, or pass --collection.".into(),
            );
            return None;
        }
    };
    if !valid_slug(&collection) {
        error(
            c,
            format!(
                "invalid collection `{collection}`: use lowercase letters, digits and dashes (max 63)"
            ),
        );
    }
    let bundle = match scan_bundle(root) {
        Ok(b) => b,
        Err(e) => {
            error(c, e.to_string());
            return None;
        }
    };
    for (path, why) in &bundle.skipped {
        let p = path.display();
        match why {
            SkipReason::TooBig => error(c, format!("{p} is larger than 1 MiB.")),
            SkipReason::Unreadable => error(c, format!("cannot read {p}.")),
            SkipReason::OutsideRoot => c.err(&format!(
                "{dir}: skipped {p}: it points outside the directory."
            )),
        }
    }
    if bundle.files.is_empty() {
        error(c, "no YAML files found.".into());
    }
    for f in &bundle.files {
        // The config file is not a request file; `load_cloud_config` has
        // already parsed it.
        if CONFIG_FILE_PATHS.contains(&f.path.as_str()) {
            continue;
        }
        let label = if dir == "." {
            f.path.clone()
        } else {
            format!("{}/{}", dir.trim_end_matches('/'), f.path)
        };
        match lint_and_print(&label, &f.text, color, c) {
            Ok(true) => {}
            Ok(false) => ok.set(false),
            Err(crash) => error(c, format!("{label}: {}", crash.0)),
        }
    }
    for leak in find_secret_literals(&bundle, &inputs) {
        error(
            c,
            format!(
                "{}: secret input {} has a literal value. Remove it and set it in Cloud.",
                leak.file, leak.name
            ),
        );
    }
    ok.get().then_some(Target {
        collection,
        name,
        description,
        inputs,
        files: bundle.files,
        hash: bundle.hash,
    })
}

/// Seconds to wait before retry number `retry` (0-based). `unit` in [0, 1)
/// places the wait inside the +-25 % jitter band. A `Retry-After` is
/// honored as sent, up to a minute.
fn delay(retry: usize, retry_after: Option<u64>, unit: f64) -> Duration {
    if let Some(s) = retry_after {
        return Duration::from_secs(s.min(MAX_RETRY_AFTER));
    }
    let base = BASE_DELAYS[retry.min(BASE_DELAYS.len() - 1)] as f64;
    Duration::from_secs_f64(base * (0.75 + 0.5 * unit))
}

fn jitter_unit() -> f64 {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    f64::from(n % 1000) / 1000.0
}

/// A client that retries transport errors, 5xx and 429.
struct Api<'a> {
    client: Client,
    server: &'a str,
    sleep: bool,
}

impl Api<'_> {
    /// One logical request. Any non-retryable HTTP status comes back as
    /// `Ok`; running out of attempts is exit 5.
    fn request(
        &self,
        c: &mut Console,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<(u16, Vec<u8>), Fail> {
        for attempt in 0..ATTEMPTS {
            let last = attempt + 1 == ATTEMPTS;
            let (why, retry_after) = match self.client.send_full(method, path, body, gzip) {
                Ok(Reply {
                    status,
                    body,
                    retry_after,
                }) => {
                    if status != 429 && status < 500 {
                        return Ok((status, body));
                    }
                    if last {
                        return Err(match serde_json::from_slice::<ErrorBody>(&body) {
                            Ok(b) => fail(EXIT_UNREACHABLE, b.message),
                            Err(_) => fail(
                                EXIT_UNREACHABLE,
                                format!(
                                    "Cannot reach {}: unexpected reply (HTTP {status}).",
                                    self.server
                                ),
                            ),
                        });
                    }
                    (format!("HTTP {status}"), retry_after)
                }
                Err(e) => {
                    if last {
                        return Err(fail(
                            EXIT_UNREACHABLE,
                            format!("Cannot reach {}: {e}.", self.server),
                        ));
                    }
                    (e.to_string(), None)
                }
            };
            if self.sleep {
                let wait = delay(attempt, retry_after, jitter_unit());
                c.err(&format!(
                    "{why}. Retrying in {:.0}s (attempt {} of {ATTEMPTS}).",
                    wait.as_secs_f64(),
                    attempt + 2
                ));
                std::thread::sleep(wait);
            }
        }
        unreachable!("the last attempt returns")
    }

    fn get<T: DeserializeOwned>(
        &self,
        c: &mut Console,
        path: &str,
        on_404: impl Fn() -> Fail,
    ) -> Result<T, Fail> {
        let (status, body) = self.request(c, "GET", path, None, false)?;
        if status == 404 {
            return Err(on_404());
        }
        decode(status, &body)
    }
}

/// A 2xx body as `T`, anything else as the failure it stands for.
fn decode<T: DeserializeOwned>(status: u16, body: &[u8]) -> Result<T, Fail> {
    if !(200..300).contains(&status) {
        return Err(api_failure(status, body));
    }
    serde_json::from_slice(body).map_err(|_| {
        fail(
            EXIT_UNREACHABLE,
            format!("The server sent an unexpected reply (HTTP {status})."),
        )
    })
}

/// The exit code for an error answer, with the server's message as-is.
fn api_failure(status: u16, body: &[u8]) -> Fail {
    if (300..400).contains(&status) {
        return fail(
            EXIT_UNREACHABLE,
            "The server answered with a redirect. Check the --server URL.",
        );
    }
    let Ok(b) = serde_json::from_slice::<ErrorBody>(body) else {
        return match status {
            401 | 403 => fail(EXIT_AUTH, format!("Not authorized (HTTP {status}).")),
            _ => fail(
                EXIT_UNREACHABLE,
                format!("The server sent an unexpected reply (HTTP {status})."),
            ),
        };
    };
    let code = match b.error {
        ErrorCode::Unauthorized | ErrorCode::Forbidden => EXIT_AUTH,
        ErrorCode::PlanLimit => EXIT_PLAN_LIMIT,
        ErrorCode::WrongSource => EXIT_WRONG_SOURCE,
        ErrorCode::Unavailable | ErrorCode::RateLimited => EXIT_UNREACHABLE,
        _ => EXIT_INVALID,
    };
    fail(code, b.message)
}

#[derive(Deserialize)]
struct CollectionRow {
    slug: String,
    current_version: Option<u32>,
}

fn cannot_publish(team: &str) -> Fail {
    fail(
        EXIT_AUTH,
        format!("This token or account can't publish to team {team}. Check --team and the token."),
    )
}

fn resolve_team(
    flag: Option<String>,
    env: Env,
    token: &str,
    api: &Api,
    c: &mut Console,
) -> Result<String, Fail> {
    let team = flag.or_else(|| env(TEAM_ENV).filter(|t| !t.is_empty()));
    let team = match team {
        Some(t) => t,
        None if token.starts_with("crt_") => {
            return Err(fail(
                EXIT_AUTH,
                "A publish token needs --team or CURL_RUNNER_TEAM.",
            ));
        }
        None => {
            let me: super::client::Me = api.get(c, "/v1/me", || {
                fail(
                    EXIT_AUTH,
                    "This account can't be found. Run curl-runner login.",
                )
            })?;
            match me.teams.as_slice() {
                [only] => only.slug.clone(),
                [] => return Err(fail(EXIT_AUTH, "Your account is not in any team.")),
                many => {
                    return Err(fail(
                        EXIT_AUTH,
                        format!("Pass --team: you're in {} teams.", many.len()),
                    ));
                }
            }
        }
    };
    if team.is_empty()
        || !team
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(fail(EXIT_AUTH, format!("{team} is not a valid team.")));
    }
    Ok(team)
}

/// What publishing `local` over `remote` adds, changes and removes.
fn diff(local: &[BundleFile], remote: &[BundleFile]) -> FileChanges {
    let old: HashMap<&str, &str> = remote
        .iter()
        .map(|f| (f.path.as_str(), f.text.as_str()))
        .collect();
    let mut out = FileChanges::default();
    for f in local {
        match old.get(f.path.as_str()) {
            None => out.added.push(f.path.clone()),
            Some(t) if *t != f.text => out.changed.push(f.path.clone()),
            Some(_) => {}
        }
    }
    let new: std::collections::HashSet<&str> = local.iter().map(|f| f.path.as_str()).collect();
    out.removed = remote
        .iter()
        .filter(|f| !new.contains(f.path.as_str()))
        .map(|f| f.path.clone())
        .collect();
    out
}

fn counts(ch: &FileChanges) -> String {
    format!(
        "{} added, {} changed, {} removed",
        ch.added.len(),
        ch.changed.len(),
        ch.removed.len()
    )
}

fn git_meta(env: Env) -> Option<GitMeta> {
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let repo_url = match (get("GITHUB_SERVER_URL"), get("GITHUB_REPOSITORY")) {
        (Some(s), Some(r)) => Some(format!("{}/{r}", s.trim_end_matches('/'))),
        _ => get("CI_PROJECT_URL"),
    };
    let meta = GitMeta {
        commit: get("GITHUB_SHA").or_else(|| get("CI_COMMIT_SHA")),
        git_ref: get("GITHUB_REF").or_else(|| get("CI_COMMIT_REF_NAME")),
        repo_url,
    };
    (meta.commit.is_some() || meta.git_ref.is_some() || meta.repo_url.is_some()).then_some(meta)
}

fn publish_one(
    api: &Api,
    c: &mut Console,
    team: &str,
    t: &Target,
    git: Option<GitMeta>,
) -> Result<(), Fail> {
    let req = PublishRequest {
        files: t.files.clone(),
        hash: t.hash.clone(),
        inputs: t.inputs.clone(),
        name: t.name.clone(),
        description: t.description.clone(),
        git,
    };
    let body = serde_json::to_vec(&req).map_err(|e| fail(EXIT_INVALID, e.to_string()))?;
    let path = format!("/v1/teams/{team}/collections/{}/versions", t.collection);
    let (status, resp) = api.request(c, "PUT", &path, Some(&body), true)?;
    if status == 404 {
        return Err(cannot_publish(team));
    }
    let r: PublishResponse = decode(status, &resp)?;
    c.out(&match r.status {
        PublishStatus::Created => format!(
            "{}: published v{} ({})",
            t.collection,
            r.version,
            counts(&r.changed)
        ),
        PublishStatus::Unchanged => format!("{}: unchanged (v{})", t.collection, r.version),
    });
    Ok(())
}

fn dry_run_one(api: &Api, c: &mut Console, team: &str, t: &Target) -> Result<(), Fail> {
    let rows: Vec<CollectionRow> = api.get(c, &format!("/v1/teams/{team}/collections"), || {
        cannot_publish(team)
    })?;
    let current = rows
        .iter()
        .find(|r| r.slug == t.collection)
        .and_then(|r| r.current_version);
    let (remote, version) = match current {
        None => (Vec::new(), None),
        Some(n) => {
            let path = format!(
                "/v1/teams/{team}/collections/{}/versions/{n}/files",
                t.collection
            );
            let (status, body) = api.request(c, "GET", &path, None, false)?;
            if status == 404 {
                (Vec::new(), None)
            } else {
                (decode::<Vec<BundleFile>>(status, &body)?, Some(n))
            }
        }
    };
    let ch = diff(&t.files, &remote);
    c.out(&match version {
        Some(n) if ch == FileChanges::default() => format!("{}: unchanged (v{n})", t.collection),
        _ => format!("{}: would publish ({})", t.collection, counts(&ch)),
    });
    Ok(())
}

pub fn publish(args: &[String], env: Env, console: &mut Console) -> i32 {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(m) => {
            console.err(&m);
            return EXIT_INVALID;
        }
    };
    if opts.collection.is_some() && opts.dirs.len() > 1 {
        console.err("--collection works with one directory.");
        return EXIT_INVALID;
    }
    let server = match server_from_args(args, env) {
        Ok(s) => s,
        Err(m) => {
            console.err(&m);
            return EXIT_INVALID;
        }
    };

    // Phase 1: check everything, send nothing.
    let color = crate::colors::enabled_from_env(env);
    let mut targets = Vec::new();
    let mut ok = true;
    for dir in &opts.dirs {
        match prepare(dir, opts.collection.as_deref(), color, console) {
            Some(t) => targets.push(t),
            None => ok = false,
        }
    }
    let mut seen = std::collections::HashSet::new();
    for t in &targets {
        if !seen.insert(t.collection.as_str()) {
            console.err(&format!(
                "{} is the collection of more than one directory.",
                t.collection
            ));
            ok = false;
        }
    }
    if !ok {
        return EXIT_INVALID;
    }

    // Phase 2: send.
    let token = match credentials::token_for(env, &server) {
        Ok(Some(t)) => t,
        Ok(None) => {
            console.err("You are not signed in. Run curl-runner login, or set CURL_RUNNER_TOKEN.");
            return EXIT_AUTH;
        }
        Err(m) => {
            console.err(&m);
            return EXIT_AUTH;
        }
    };
    let api = Api {
        client: Client::new(&server, Some(token.clone())),
        server: &server,
        sleep: hook(env, NO_SLEEP_ENV, HOOKS_ENABLED).is_none(),
    };
    let run = |console: &mut Console| -> Result<(), Fail> {
        let team = resolve_team(opts.team.clone(), env, &token, &api, console)?;
        let git = git_meta(env);
        for t in &targets {
            if opts.dry_run {
                dry_run_one(&api, console, &team, t)?;
            } else {
                publish_one(&api, console, &team, t, git.clone())?;
            }
        }
        Ok(())
    };
    match run(console) {
        Ok(()) => 0,
        Err(f) => {
            console.err(&f.message);
            f.code
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, text: &str) -> BundleFile {
        BundleFile {
            path: path.into(),
            text: text.into(),
        }
    }

    #[test]
    fn backoff_is_1_2_4_8_with_25_percent_jitter() {
        for (i, base) in [1.0, 2.0, 4.0, 8.0].into_iter().enumerate() {
            assert_eq!(delay(i, None, 0.5).as_secs_f64(), base);
            assert!((delay(i, None, 0.0).as_secs_f64() - base * 0.75).abs() < 1e-9);
            assert!((delay(i, None, 0.999).as_secs_f64() - base * 1.2495).abs() < 1e-6);
        }
    }

    #[test]
    fn retry_after_is_honored_and_capped() {
        assert_eq!(delay(0, Some(7), 0.9), Duration::from_secs(7));
        assert_eq!(delay(0, Some(3600), 0.9), Duration::from_secs(60));
    }

    #[test]
    fn diff_counts_added_changed_removed() {
        let local = [f("a", "1"), f("b", "2"), f("c", "3")];
        let remote = [f("a", "1"), f("b", "old"), f("z", "9")];
        let d = diff(&local, &remote);
        assert_eq!(
            (d.added, d.changed, d.removed),
            (
                vec!["c".to_string()],
                vec!["b".to_string()],
                vec!["z".to_string()]
            )
        );
    }

    #[test]
    fn git_meta_prefers_github_then_gitlab_then_none() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(a, _)| *a == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(git_meta(&env(&[])), None);
        let g = git_meta(&env(&[
            ("GITHUB_SHA", "abc"),
            ("GITHUB_SERVER_URL", "https://github.com/"),
            ("GITHUB_REPOSITORY", "acme/x"),
            ("CI_COMMIT_SHA", "zzz"),
        ]))
        .unwrap();
        assert_eq!(g.commit.as_deref(), Some("abc"));
        assert_eq!(g.git_ref, None);
        assert_eq!(g.repo_url.as_deref(), Some("https://github.com/acme/x"));
        let l = git_meta(&env(&[
            ("CI_COMMIT_SHA", "def"),
            ("CI_COMMIT_REF_NAME", "main"),
            ("CI_PROJECT_URL", "https://gitlab.example.com/acme/x"),
        ]))
        .unwrap();
        assert_eq!(l.commit.as_deref(), Some("def"));
        assert_eq!(l.git_ref.as_deref(), Some("main"));
        assert_eq!(
            l.repo_url.as_deref(),
            Some("https://gitlab.example.com/acme/x")
        );
        // Only one half of the GitHub pair gives no repo_url.
        assert_eq!(git_meta(&env(&[("GITHUB_REPOSITORY", "acme/x")])), None);
    }

    #[test]
    fn error_codes_map_to_exit_codes() {
        let body = |code: &str| format!(r#"{{"error":"{code}","message":"Msg."}}"#);
        for (code, exit) in [
            ("unauthorized", 2),
            ("forbidden", 2),
            ("plan_limit", 3),
            ("wrong_source", 4),
            ("unavailable", 5),
            ("invalid_bundle", 1),
        ] {
            let f = api_failure(400, body(code).as_bytes());
            assert_eq!((f.code, f.message.as_str()), (exit, "Msg."), "{code}");
        }
        assert_eq!(api_failure(502, b"<html>").code, 5);
        assert_eq!(api_failure(403, b"<html>").code, 2);
        assert_eq!(api_failure(302, b"").code, 5);
    }

    #[test]
    fn options_parse_flags_and_dirs() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let o = parse_options(&a(&[
            "x",
            "--team",
            "acme",
            "y",
            "--dry-run",
            "--collection=c",
        ]))
        .unwrap();
        assert_eq!(o.dirs, ["x", "y"]);
        assert_eq!(o.team.as_deref(), Some("acme"));
        assert_eq!(o.collection.as_deref(), Some("c"));
        assert!(o.dry_run);
        assert_eq!(parse_options(&a(&[])).unwrap().dirs, ["."]);
        assert!(parse_options(&a(&["--bogus"])).is_err());
        assert!(parse_options(&a(&["--team"])).is_err());
    }

    #[test]
    fn slugs() {
        assert!(valid_slug("payments-api"));
        assert!(!valid_slug("Payments"));
        assert!(!valid_slug("-a"));
        assert!(!valid_slug(""));
    }
}
