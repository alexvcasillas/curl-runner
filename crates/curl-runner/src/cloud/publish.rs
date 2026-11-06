//! `curl-runner publish [DIR...] [--collection SLUG] [--team TEAM]
//! [--server URL] [--dry-run]`.
//!
//! This is what CI runs on every push, so it checks everything locally
//! before it sends anything (phase 1), retries what is worth retrying on its
//! own, and ends with one exit code per kind of failure (phase 2):
//! 0 ok, 1 invalid files, 2 auth, 3 plan limit, 4 wrong source, 5 unreachable.

use super::{Env, credentials, flag_value, server_from_args};
use crate::output::Console;
use crate::validate_cmd::lint_and_print;
use crate::version_check::{HOOKS_ENABLED, hook};
use curl_runner_cloud_client::retry;
use curl_runner_cloud_proto::{
    BundleFile, ErrorBody, ErrorCode, FileChanges, GitMeta, Input, PublishRequest, PublishResponse,
    PublishStatus,
};
use curl_runner_core::cloud::{
    SkipReason, find_secret_literals, load_cloud_config, scan_bundle, valid_slug,
};
use curl_runner_core::config::CONFIG_FILE_PATHS;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::Path;

pub(super) const EXIT_INVALID: i32 = 1;
pub(super) const EXIT_AUTH: i32 = 2;
const EXIT_PLAN_LIMIT: i32 = 3;
const EXIT_WRONG_SOURCE: i32 = 4;
pub(super) const EXIT_UNREACHABLE: i32 = 5;

const NO_SLEEP_ENV: &str = "CURL_RUNNER_TEST_NO_SLEEP";
const TEAM_ENV: &str = "CURL_RUNNER_TEAM";

/// A failed publish: what to print and the exit code.
#[derive(Debug, PartialEq)]
pub(super) struct Fail {
    pub(super) code: i32,
    pub(super) message: String,
}

pub(super) fn fail(code: i32, message: impl Into<String>) -> Fail {
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

/// Phase 1 for one directory. Prints every problem it finds and returns the
/// target only when there are none.
fn prepare(dir: &str, slug: Option<&str>, color: bool, c: &mut Console) -> Option<Target> {
    let root = Path::new(dir);
    let ok = std::cell::Cell::new(true);
    let error = |c: &mut Console, msg: String| {
        c.err(&format!("{dir}: {msg}"));
        ok.set(false);
    };
    // Only the plain names are published: the bundle leaves dotfiles out, so a
    // dotted config would go missing without a word.
    if let Some(name) = CONFIG_FILE_PATHS
        .iter()
        .find(|n| root.join(n).is_file())
        .filter(|n| n.starts_with('.'))
    {
        let plain = name.trim_start_matches('.');
        error(c, format!("rename {name} to {plain} to publish it."));
        return None;
    }
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

/// A client that retries transport errors, 5xx and 429 (see
/// `curl_runner_cloud_client::retry`), reporting failures as exit codes.
pub(super) struct Api<'a> {
    inner: retry::Api<'a>,
}

impl<'a> Api<'a> {
    pub(super) fn new(server: &'a str, token: Option<String>, env: Env) -> Api<'a> {
        Api {
            inner: retry::Api::new(
                server,
                token,
                hook(env, NO_SLEEP_ENV, HOOKS_ENABLED).is_none(),
            ),
        }
    }

    /// One logical request. Any non-retryable HTTP status comes back as
    /// `Ok`; running out of attempts is exit 5.
    pub(super) fn request(
        &self,
        c: &mut Console,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<(u16, Vec<u8>), Fail> {
        self.inner
            .request(&mut |m| c.err(m), method, path, body, gzip)
            .map_err(|m| fail(EXIT_UNREACHABLE, m))
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
pub(super) fn api_failure(status: u16, body: &[u8]) -> Fail {
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
        // A suspended team is refused like a token that may not publish:
        // nothing in the bundle can fix it.
        ErrorCode::Unauthorized | ErrorCode::Forbidden | ErrorCode::TeamSuspended => EXIT_AUTH,
        ErrorCode::PlanLimit => EXIT_PLAN_LIMIT,
        ErrorCode::WrongSource => EXIT_WRONG_SOURCE,
        ErrorCode::Unavailable | ErrorCode::RateLimited => EXIT_UNREACHABLE,
        _ => EXIT_INVALID,
    };
    fail(code, super::escape_control(&b.message))
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
            let me: curl_runner_cloud_client::Me = api.get(c, "/v1/me", || {
                fail(
                    EXIT_AUTH,
                    "This account can't be found. Run curl-runner login.",
                )
            })?;
            match me.teams.as_slice() {
                [only] => only.slug.clone(),
                [] => {
                    return Err(fail(
                        EXIT_AUTH,
                        "Your account is not in any team. Cloud is in private preview.",
                    ));
                }
                many => {
                    return Err(fail(
                        EXIT_AUTH,
                        format!("Pass --team: you're in {} teams.", many.len()),
                    ));
                }
            }
        }
    };
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

fn dry_run_one(
    api: &Api,
    c: &mut Console,
    team: &str,
    rows: &[CollectionRow],
    t: &Target,
) -> Result<(), Fail> {
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
    let team_env = env(TEAM_ENV).filter(|t| !t.is_empty());
    if let Some(t) = opts.team.as_ref().or(team_env.as_ref())
        && !valid_slug(t)
    {
        console.err(&format!("{t} is not a valid team."));
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
    let api = Api::new(&server, Some(token.clone()), env);
    let team = match resolve_team(opts.team.clone(), env, &token, &api, console) {
        Ok(t) => t,
        Err(f) => {
            console.err(&f.message);
            return f.code;
        }
    };
    let git = git_meta(env);
    let rows: Vec<CollectionRow> = if opts.dry_run {
        match api.get(console, &format!("/v1/teams/{team}/collections"), || {
            cannot_publish(&team)
        }) {
            Ok(r) => r,
            Err(f) => {
                console.err(&f.message);
                return f.code;
            }
        }
    } else {
        Vec::new()
    };
    // A server-side refusal of one collection does not stop the others; an
    // unreachable server does, since the rest would fail the same way.
    let mut worst = 0;
    for t in &targets {
        let result = if opts.dry_run {
            dry_run_one(&api, console, &team, &rows, t)
        } else {
            publish_one(&api, console, &team, t, git.clone())
        };
        if let Err(f) = result {
            if targets.len() > 1 {
                console.err(&format!("{}: {}", t.collection, f.message));
            } else {
                console.err(&f.message);
            }
            worst = worse(worst, f.code);
            if f.code == EXIT_UNREACHABLE {
                break;
            }
        }
    }
    worst
}

/// The more severe of two exit codes: 5 unreachable, then 2 auth, 4 wrong
/// source, 3 plan limit, 1 invalid, 0 ok.
fn worse(a: i32, b: i32) -> i32 {
    let rank = |c: i32| match c {
        EXIT_UNREACHABLE => 5,
        EXIT_AUTH => 4,
        EXIT_WRONG_SOURCE => 3,
        EXIT_PLAN_LIMIT => 2,
        EXIT_INVALID => 1,
        _ => 0,
    };
    if rank(b) > rank(a) { b } else { a }
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
            ("team_suspended", 2),
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
    fn worst_exit_code_is_5_2_4_3_1() {
        let order = [0, 1, 3, 4, 2, 5];
        for (i, a) in order.iter().enumerate() {
            for b in &order[..=i] {
                assert_eq!(worse(*b, *a), *a);
                assert_eq!(worse(*a, *b), *a);
            }
        }
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
}
