//! `curl-runner pull [LINK] [DIR] [--force]`: download a shared collection
//! and keep it updated.
//!
//! The server decides every file name, so nothing is written before the
//! whole bundle passes the shared path checks, and every target is built from
//! those validated components under the target folder, never through a
//! symlink. Files are staged first and renamed into place, and the state file
//! is written last, so a failed pull does not leave a half-updated folder.
//!
//! Exit codes: 0 ok, 1 user or data problem, 2 auth, 5 unreachable.

use super::client::{self, Client};
use super::credentials::Credentials;
use super::publish::{Api, EXIT_AUTH, EXIT_INVALID, Fail, api_failure, fail};
use super::{Env, credentials, escape_control, login};
use crate::output::Console;
use crate::version_check::{HOOKS_ENABLED, hook};
use curl_runner_cloud_proto::{
    BundleError, BundleFile, Input, Snapshot, file_hash, validate_bundle, validate_inputs,
    validate_path,
};
use curl_runner_core::cloud::valid_slug;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

const HOSTED_LANDING_HOST: &str = "cloud.curl-runner.com";
const HOSTED_API: &str = "https://api.curl-runner.com";
const STATE_DIR: &str = ".curl-runner";
const STATE_FILE: &str = "pull.json";
const STAGING_DIR: &str = "staging";
const ENV_EXAMPLE: &str = ".env.example";
const LINK_BROKEN: &str =
    "This link doesn't work. It may have been reset, expired or turned off by an admin.";

/// What `pull.json` records: where the folder came from and what was written.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct State {
    server: String,
    token: String,
    version: u32,
    /// Path to `file_hash` of what was written.
    files: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Latest {
    version: u32,
}

#[derive(Debug, PartialEq)]
enum Target {
    /// Pull this link; the folder is optional.
    Link { link: String, dir: Option<String> },
    /// Update a folder that was pulled before.
    Dir(String),
}

#[derive(Debug, PartialEq)]
struct Options {
    target: Target,
    force: bool,
}

fn is_link(s: &str) -> bool {
    ["https://", "http://", "curl-runner://"]
        .iter()
        .any(|p| s.starts_with(p))
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut force = false;
    let mut positional = Vec::new();
    for a in args {
        match a.as_str() {
            "--force" => force = true,
            s if s.starts_with("--") => return Err(format!("Unknown option {s}.")),
            s => positional.push(s.to_string()),
        }
    }
    let mut it = positional.into_iter();
    let target = match (it.next(), it.next(), it.next()) {
        (None, _, _) => Target::Dir(".".into()),
        (Some(a), None, _) if !is_link(&a) => Target::Dir(a),
        (Some(a), dir, None) if is_link(&a) => Target::Link { link: a, dir },
        (Some(_), Some(_), None) => {
            return Err("The first argument must be a share link.".into());
        }
        _ => return Err("Too many arguments. Use curl-runner pull [LINK] [DIR].".into()),
    };
    if force && matches!(target, Target::Link { .. }) {
        return Err("--force only applies when updating a folder.".into());
    }
    Ok(Options { target, force })
}

/// A share token goes into a URL path, so it is plain URL-safe characters.
fn valid_token(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 256
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn bad_link() -> Fail {
    fail(
        EXIT_INVALID,
        "That isn't a curl-runner share link. Use https://<host>/s/<token>.",
    )
}

/// A parsed link: the API base to call, or a landing page that must be asked
/// where its API is.
#[derive(Debug, PartialEq)]
enum Link {
    Api { server: String, token: String },
    Landing { origin: String, token: String },
}

fn parse_link(link: &str) -> Result<Link, Fail> {
    let u = url::Url::parse(link).map_err(|_| bad_link())?;
    if u.scheme() == "curl-runner" {
        if u.host_str() != Some("open") {
            return Err(bad_link());
        }
        let get = |k: &str| {
            u.query_pairs()
                .find(|(name, _)| name == k)
                .map(|(_, v)| v.into_owned())
        };
        let (Some(server), Some(token)) = (get("server"), get("share")) else {
            return Err(bad_link());
        };
        let server = server.trim_end_matches('/').to_string();
        client::check_server(&server).map_err(|m| fail(EXIT_INVALID, m))?;
        if !valid_token(&token) {
            return Err(bad_link());
        }
        return Ok(Link::Api { server, token });
    }
    client::check_server(link).map_err(|m| fail(EXIT_INVALID, m))?;
    let segments: Vec<&str> = u
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();
    let ["s", token] = segments.as_slice() else {
        return Err(bad_link());
    };
    if !valid_token(token) {
        return Err(bad_link());
    }
    let host = u.host_str().ok_or_else(bad_link)?;
    let origin = match u.port() {
        Some(p) if host.contains(':') => format!("{}://[{host}]:{p}", u.scheme()),
        Some(p) => format!("{}://{host}:{p}", u.scheme()),
        None if host.contains(':') => format!("{}://[{host}]", u.scheme()),
        None => format!("{}://{host}", u.scheme()),
    };
    Ok(Link::Landing {
        origin,
        token: token.to_string(),
    })
}

#[derive(Deserialize)]
struct WellKnown {
    api: String,
}

/// The API base for a landing page: the test hook, else the host's
/// `/.well-known/curl-runner.json` (no redirects), else, for the hosted
/// landing host only, the hosted API.
fn api_for_landing(origin: &str, env: Env) -> Result<String, Fail> {
    if let Some(s) = hook(env, client::TEST_SERVER_ENV, HOOKS_ENABLED) {
        let s = s.trim_end_matches('/').to_string();
        client::check_server(&s).map_err(|m| fail(EXIT_INVALID, m))?;
        return Ok(s);
    }
    let host = url::Url::parse(origin)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default();
    let found = Client::new(origin, None)
        .send("GET", "/.well-known/curl-runner.json", None, false)
        .ok()
        .filter(|(status, _)| *status == 200)
        .and_then(|(_, body)| serde_json::from_slice::<WellKnown>(&body).ok())
        .map(|w| w.api.trim_end_matches('/').to_string())
        .filter(|api| !api.chars().any(char::is_control) && client::check_server(api).is_ok());
    match found {
        Some(api) => Ok(api),
        None if host == HOSTED_LANDING_HOST => Ok(HOSTED_API.to_string()),
        None => Err(fail(
            EXIT_INVALID,
            format!(
                "Couldn't find the curl-runner server for {}.",
                escape_control(&host)
            ),
        )),
    }
}

fn resolve_link(link: &str, env: Env) -> Result<(String, String), Fail> {
    match parse_link(link)? {
        Link::Api { server, token } => Ok((server, token)),
        Link::Landing { origin, token } => Ok((api_for_landing(&origin, env)?, token)),
    }
}

/// The saved session for exactly this server.
fn saved_token(env: Env, server: &str) -> Option<String> {
    let path = credentials::path(env)?;
    Credentials::load(&path)
        .ok()?
        .servers
        .get(server)
        .map(|e| e.session.clone())
}

/// The bearer for `server`. A `CURL_RUNNER_TOKEN` belongs to the configured
/// server only; a link may name any server, so it never gets that token.
fn bearer_for(env: Env, server: &str) -> Result<Option<String>, Fail> {
    if client::resolve_server(None, env) == server {
        credentials::token_for(env, server).map_err(|m| fail(EXIT_AUTH, m))
    } else {
        Ok(saved_token(env, server))
    }
}

/// GET with retries; a 401 offers an interactive sign-in once.
fn get_authed(
    c: &mut Console,
    env: Env,
    server: &str,
    bearer: &mut Option<String>,
    path: &str,
) -> Result<(u16, Vec<u8>), Fail> {
    let mut signed_in = false;
    loop {
        let api = Api::new(server, bearer.clone(), env);
        let (status, body) = api.request(c, "GET", path, None, false)?;
        if status != 401 {
            return Ok((status, body));
        }
        if signed_in {
            return Err(fail(
                EXIT_AUTH,
                "That account can't open this collection. Check that you signed in with the right email.",
            ));
        }
        if !std::io::stdin().is_terminal() {
            return Err(fail(
                EXIT_AUTH,
                format!(
                    "This collection needs you to sign in. Run curl-runner login --server {}, then pull again.",
                    escape_control(server)
                ),
            ));
        }
        let host = url::Url::parse(server)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| server.to_string());
        c.out(&format!(
            "This collection needs you to sign in to {}.",
            escape_control(&host)
        ));
        let args = ["--server".to_string(), server.to_string()];
        if login::login(&args, env, c) != 0 {
            return Err(fail(
                EXIT_AUTH,
                "Sign-in didn't finish. Nothing was pulled.",
            ));
        }
        *bearer = saved_token(env, server);
        signed_in = true;
    }
}

fn decode_ok<T: serde::de::DeserializeOwned>(status: u16, body: &[u8]) -> Result<T, Fail> {
    if status == 404 {
        return Err(fail(EXIT_INVALID, LINK_BROKEN));
    }
    if !(200..300).contains(&status) {
        // Plan and source errors are publish concerns: for a pull they are
        // plain failures.
        let mut f = api_failure(status, body);
        if f.code == 3 || f.code == 4 {
            f.code = EXIT_INVALID;
        }
        return Err(f);
    }
    serde_json::from_slice(body).map_err(|_| {
        fail(
            super::publish::EXIT_UNREACHABLE,
            format!("The server sent an unexpected reply (HTTP {status})."),
        )
    })
}

fn fetch_snapshot(
    c: &mut Console,
    env: Env,
    server: &str,
    token: &str,
    bearer: &mut Option<String>,
) -> Result<Snapshot, Fail> {
    let (status, body) = get_authed(
        c,
        env,
        server,
        bearer,
        &format!("/v1/shared/{token}/snapshot"),
    )?;
    decode_ok(status, &body)
}

fn unsafe_path(p: &str) -> Fail {
    fail(
        EXIT_INVALID,
        format!(
            "The server sent an unsafe file path: {}. Nothing was written.",
            escape_control(p)
        ),
    )
}

/// Refuse the whole snapshot when any path or input is not safe to write.
fn check_snapshot(s: &Snapshot) -> Result<(), Fail> {
    if !valid_slug(&s.info.collection) {
        return Err(fail(
            EXIT_INVALID,
            "The server sent an invalid collection name. Nothing was written.",
        ));
    }
    match validate_bundle(&s.files) {
        Ok(()) => {}
        Err(BundleError::Path { path, .. } | BundleError::DuplicatePath(path)) => {
            return Err(unsafe_path(&path));
        }
        Err(BundleError::CaseCollision(_, path) | BundleError::FileFolderConflict(_, path)) => {
            return Err(unsafe_path(&path));
        }
        Err(e) => {
            return Err(fail(
                EXIT_INVALID,
                format!(
                    "The server sent an invalid collection ({}). Nothing was written.",
                    escape_control(&e.to_string())
                ),
            ));
        }
    }
    validate_inputs(&s.inputs).map_err(|e| {
        fail(
            EXIT_INVALID,
            format!(
                "The server sent invalid inputs ({}). Nothing was written.",
                escape_control(&e.to_string())
            ),
        )
    })
}

/// `root` joined with the validated components of `rel`. A component that is
/// a symlink, or a folder where a file belongs (or the reverse), is refused.
/// With `create`, missing folders on the way are made. `root` must already be
/// canonical.
fn safe_path(root: &Path, rel: &str, create: bool) -> Result<PathBuf, ()> {
    validate_path(rel).map_err(|_| ())?;
    let parts: Vec<&str> = rel.split('/').collect();
    let mut p = root.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        p.push(part);
        let last = i + 1 == parts.len();
        match std::fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => return Err(()),
            Ok(m) if last && !m.is_file() => return Err(()),
            Ok(m) if !last && !m.is_dir() => return Err(()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !last && create {
                    std::fs::create_dir(&p).map_err(|_| ())?;
                }
            }
            Err(_) => return Err(()),
        }
    }
    Ok(p)
}

/// Hash of the file on disk, or `None` when it is missing or unreadable.
fn disk_hash(root: &Path, rel: &str) -> Option<String> {
    let p = safe_path(root, rel, false).ok()?;
    let bytes = std::fs::read(p).ok()?;
    Some(match String::from_utf8(bytes) {
        Ok(text) => file_hash(&text),
        // Not text, so certainly not what was written.
        Err(_) => String::new(),
    })
}

/// Files the user changed since the last pull, or would lose to this one:
/// recorded files whose hash differs or that are missing, and files the new
/// snapshot adds over something already on disk. A file that already holds
/// the new content is clean, so a retry after a partial update succeeds.
/// Paths are matched case-insensitively: `A.yaml` and `a.yaml` are one file
/// on some systems.
fn local_edits(root: &Path, old: &State, new: &[BundleFile]) -> Vec<String> {
    let new_by_lower: HashMap<String, &BundleFile> =
        new.iter().map(|f| (f.path.to_lowercase(), f)).collect();
    let old_lower: std::collections::HashSet<String> =
        old.files.keys().map(|p| p.to_lowercase()).collect();
    let mut out: Vec<String> = old
        .files
        .iter()
        .filter(|(path, hash)| {
            if validate_path(path).is_err() {
                return false;
            }
            if disk_hash(root, path).as_deref() == Some(hash.as_str()) {
                return false;
            }
            let Some(twin) = new_by_lower.get(&path.to_lowercase()) else {
                return true;
            };
            let want = file_hash(&twin.text);
            disk_hash(root, path).as_deref() != Some(want.as_str())
                && disk_hash(root, &twin.path).as_deref() != Some(want.as_str())
        })
        .map(|(path, _)| path.clone())
        .collect();
    for f in new {
        if old.files.contains_key(&f.path) {
            continue;
        }
        let Some(h) = disk_hash(root, &f.path) else {
            continue;
        };
        if h == file_hash(&f.text) {
            continue;
        }
        if old_lower.contains(&f.path.to_lowercase()) {
            // A case-only rename. Where the two names are one file, it is
            // covered by the recorded path above. Where they are two
            // files, the new name is the user's.
            let twin = old
                .files
                .keys()
                .find(|p| p.to_lowercase() == f.path.to_lowercase());
            if twin.is_none_or(|p| disk_hash(root, p).as_deref() == Some(h.as_str())) {
                continue;
            }
        }
        out.push(f.path.clone());
    }
    out.sort();
    out
}

fn env_value(v: &str) -> String {
    if v.contains(['\n', '\r']) {
        format!("{v:?}")
    } else {
        v.to_string()
    }
}

/// One `# description` and one `NAME=` line per input; only non-secret
/// inputs get their default.
fn env_example(inputs: &[Input]) -> String {
    let mut out = String::new();
    for i in inputs {
        if let Some(d) = i.description.as_deref().filter(|d| !d.trim().is_empty()) {
            out.push_str("# ");
            out.push_str(&d.split(['\n', '\r']).collect::<Vec<_>>().join(" "));
            out.push('\n');
        }
        out.push_str(&i.name);
        out.push('=');
        if !i.secret
            && let Some(d) = &i.default
        {
            out.push_str(&env_value(d));
        }
        out.push('\n');
    }
    out
}

fn io_fail(what: &str, e: std::io::Error) -> Fail {
    fail(
        EXIT_INVALID,
        format!("Couldn't {what}: {e}. Nothing was changed."),
    )
}

/// The temp path for `name` in `dir`. A stale regular file is removed;
/// anything else there (a planted symlink, a folder) is refused, and the
/// path is never followed.
fn temp_path(dir: &Path, name: &str) -> Result<PathBuf, Fail> {
    let tmp = dir.join(format!("{name}.tmp"));
    match std::fs::symlink_metadata(&tmp) {
        Ok(m) if m.file_type().is_file() => {
            std::fs::remove_file(&tmp).map_err(|e| io_fail("replace a stale temp file", e))?;
        }
        Ok(_) => return Err(unsafe_path(&format!("{STATE_DIR}/{name}.tmp"))),
        Err(_) => {}
    }
    Ok(tmp)
}

fn write_atomic(dir: &Path, name: &str, data: &[u8]) -> Result<(), Fail> {
    use std::io::Write;
    let tmp = temp_path(dir, name)?;
    let err = |e| io_fail("write the state file", e);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(err)?;
    f.write_all(data).map_err(err)?;
    drop(f);
    std::fs::rename(&tmp, dir.join(name)).map_err(err)
}

/// Stage the new files, move them into place, drop what left the snapshot,
/// then record the state. `old` is the previous state of an update.
fn apply(
    root: &Path,
    snap: &Snapshot,
    old: Option<&State>,
    server: &str,
    token: &str,
) -> Result<Vec<String>, Fail> {
    let state_dir = root.join(STATE_DIR);
    match std::fs::symlink_metadata(&state_dir) {
        Ok(m) if !m.is_dir() => return Err(unsafe_path(STATE_DIR)),
        Ok(_) => {}
        Err(_) => std::fs::create_dir(&state_dir).map_err(|e| io_fail("create .curl-runner", e))?,
    }
    temp_path(&state_dir, STATE_FILE)?;
    // Stage everything first: a failure here leaves the folder untouched.
    let staging = state_dir.join(STAGING_DIR);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir(&staging).map_err(|e| io_fail("create a staging folder", e))?;
    let result = place(root, &staging, snap, old, server, token);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn place(
    root: &Path,
    staging: &Path,
    snap: &Snapshot,
    old: Option<&State>,
    server: &str,
    token: &str,
) -> Result<Vec<String>, Fail> {
    let mut staged = Vec::new();
    for (i, f) in snap.files.iter().enumerate() {
        // Every target is resolved (folders made) before anything moves, so
        // a file where a folder belongs, or the reverse, fails here.
        let target = safe_path(root, &f.path, true).map_err(|_| unsafe_path(&f.path))?;
        if target.exists() && disk_hash(root, &f.path) == Some(file_hash(&f.text)) {
            continue;
        }
        let tmp = staging.join(format!("{i}.tmp"));
        std::fs::write(&tmp, &f.text).map_err(|e| io_fail("stage files", e))?;
        staged.push((f, tmp));
    }
    let example = staging.join("env-example.tmp");
    std::fs::write(&example, env_example(&snap.inputs)).map_err(|e| io_fail("stage files", e))?;

    // Case-only renames first, so the file ends up with the new spelling.
    let kept: HashMap<String, &str> = snap
        .files
        .iter()
        .map(|f| (f.path.to_lowercase(), f.path.as_str()))
        .collect();
    if let Some(old) = old {
        for p in old.files.keys() {
            let Some(q) = kept.get(&p.to_lowercase()).filter(|q| **q != p.as_str()) else {
                continue;
            };
            let (Ok(from), Ok(to)) = (safe_path(root, p, false), safe_path(root, q, true)) else {
                continue;
            };
            if from.exists() {
                std::fs::rename(&from, &to).map_err(|e| io_fail(&format!("rename {p}"), e))?;
            }
        }
    }
    for (f, tmp) in &staged {
        let target = safe_path(root, &f.path, true).map_err(|_| unsafe_path(&f.path))?;
        std::fs::rename(tmp, target).map_err(|e| io_fail(&format!("write {}", f.path), e))?;
    }
    std::fs::rename(&example, root.join(ENV_EXAMPLE))
        .map_err(|e| io_fail("write .env.example", e))?;

    let mut failed = Vec::new();
    if let Some(old) = old {
        for path in old
            .files
            .keys()
            .filter(|p| !kept.contains_key(&p.to_lowercase()))
        {
            // pull.json is editable by hand: only validated paths are removed.
            let Ok(target) = safe_path(root, path, false) else {
                continue;
            };
            match std::fs::remove_file(&target) {
                Ok(()) => prune_empty_parents(root, &target),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => failed.push(path.clone()),
            }
        }
    }

    let state = State {
        server: server.to_string(),
        token: token.to_string(),
        version: snap.info.version,
        files: snap
            .files
            .iter()
            .map(|f| (f.path.clone(), file_hash(&f.text)))
            // A file that could not be removed stays recorded.
            .chain(failed.iter().filter_map(|p| {
                old.and_then(|o| o.files.get(p))
                    .map(|h| (p.clone(), h.clone()))
            }))
            .collect(),
    };
    let json = serde_json::to_vec_pretty(&state).map_err(|e| fail(EXIT_INVALID, e.to_string()))?;
    write_atomic(&root.join(STATE_DIR), STATE_FILE, &json)?;
    Ok(failed)
}

/// Remove folders emptied by a removal, never `root` itself.
fn prune_empty_parents(root: &Path, file: &Path) {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d == root || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn is_empty_dir(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut d| d.next().is_none())
}

fn check_empty(dir: &str) -> Result<(), Fail> {
    let p = Path::new(dir);
    if p.exists() && !is_empty_dir(p) {
        return Err(fail(
            EXIT_INVALID,
            format!("{dir} isn't empty. Pass an empty folder."),
        ));
    }
    Ok(())
}

fn first_pull(c: &mut Console, env: Env, link: &str, dir: Option<String>) -> Result<(), Fail> {
    let (server, token) = resolve_link(link, env)?;
    if let Some(d) = &dir {
        check_empty(d)?;
    }
    let mut bearer = bearer_for(env, &server)?;
    let snap = fetch_snapshot(c, env, &server, &token, &mut bearer)?;
    check_snapshot(&snap)?;
    let dir = dir.unwrap_or_else(|| snap.info.collection.clone());
    check_empty(&dir)?;

    let root = Path::new(&dir);
    // The topmost folder this pull makes, to take away again on failure.
    let mut created = None;
    for a in root.ancestors() {
        if a.as_os_str().is_empty() || a.exists() {
            break;
        }
        created = Some(a.to_path_buf());
    }
    std::fs::create_dir_all(root).map_err(|e| io_fail(&format!("create {dir}"), e))?;
    let result = root
        .canonicalize()
        .map_err(|e| io_fail(&format!("open {dir}"), e))
        .and_then(|canon| apply(&canon, &snap, None, &server, &token).map(drop));
    if let Err(e) = result {
        // The folder was empty or new: put it back the way it was.
        if let Some(top) = created {
            let _ = std::fs::remove_dir_all(top);
        } else if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let p = entry.path();
                let _ = if p.is_dir() {
                    std::fs::remove_dir_all(&p)
                } else {
                    std::fs::remove_file(&p)
                };
            }
        }
        return Err(e);
    }
    c.out(&format!(
        "Pulled {} v{} into {dir}. Fill in {dir}/.env (see .env.example), then run: cd {dir} && curl-runner --all",
        escape_control(&snap.info.name),
        snap.info.version
    ));
    Ok(())
}

fn read_state(dir: &str) -> Result<State, Fail> {
    let path = Path::new(dir).join(STATE_DIR).join(STATE_FILE);
    let not_pulled = || {
        fail(
            EXIT_INVALID,
            format!("{dir} wasn't pulled with curl-runner pull. Pass a share link to pull it."),
        )
    };
    let text = std::fs::read_to_string(&path).map_err(|_| not_pulled())?;
    let state: State = serde_json::from_str(&text).map_err(|_| {
        fail(
            EXIT_INVALID,
            format!("{} is damaged. Pull into a new folder.", path.display()),
        )
    })?;
    if client::check_server(&state.server).is_err() || !valid_token(&state.token) {
        return Err(fail(
            EXIT_INVALID,
            format!("{} is damaged. Pull into a new folder.", path.display()),
        ));
    }
    Ok(state)
}

fn confirm_replace(list: &str) -> bool {
    cliclack::confirm(format!("Replace your changes to {list}?"))
        .interact()
        .unwrap_or(false)
}

fn update(c: &mut Console, env: Env, dir: &str, force: bool) -> Result<(), Fail> {
    let old = read_state(dir)?;
    let mut bearer = bearer_for(env, &old.server)?;
    let (status, body) = get_authed(
        c,
        env,
        &old.server,
        &mut bearer,
        &format!("/v1/shared/{}/versions/latest", old.token),
    )?;
    let latest: Latest = decode_ok(status, &body)?;
    if latest.version == old.version {
        c.out(&format!("Up to date (v{}).", old.version));
        return Ok(());
    }
    let snap = fetch_snapshot(c, env, &old.server, &old.token, &mut bearer)?;
    check_snapshot(&snap)?;
    let root = Path::new(dir)
        .canonicalize()
        .map_err(|e| io_fail(&format!("open {dir}"), e))?;
    let edits = local_edits(&root, &old, &snap.files);
    if !edits.is_empty() && !force {
        let list = escape_control(&edits.join(", "));
        if !std::io::stdin().is_terminal() {
            return Err(fail(
                EXIT_INVALID,
                format!("You changed {list}. Pass --force to replace your changes."),
            ));
        }
        if !confirm_replace(&list) {
            return Err(fail(
                EXIT_INVALID,
                "Kept your changes. Nothing was updated.",
            ));
        }
    }
    let failed = apply(&root, &snap, Some(&old), &old.server, &old.token)?;
    c.out(&format!(
        "Updated {} to v{}.",
        escape_control(&snap.info.name),
        snap.info.version
    ));
    if !failed.is_empty() {
        let list = escape_control(&failed.join(", "));
        return Err(fail(
            EXIT_INVALID,
            format!("Couldn't remove {list}. Remove it yourself, or pull again."),
        ));
    }
    Ok(())
}

pub fn pull(args: &[String], env: Env, console: &mut Console) -> i32 {
    let opts = match parse_options(args) {
        Ok(o) => o,
        Err(m) => {
            console.err(&m);
            return EXIT_INVALID;
        }
    };
    let result = match opts.target {
        Target::Link { link, dir } => first_pull(console, env, &link, dir),
        Target::Dir(dir) => update(console, env, &dir, opts.force),
    };
    match result {
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

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn input(name: &str, description: Option<&str>, secret: bool, default: Option<&str>) -> Input {
        Input {
            name: name.into(),
            description: description.map(str::to_string),
            secret,
            required: false,
            locked: false,
            default: default.map(str::to_string),
        }
    }

    #[test]
    fn options_pick_link_or_dir() {
        let o = parse_options(&a(&[])).unwrap();
        assert_eq!((o.target, o.force), (Target::Dir(".".into()), false));
        assert_eq!(
            parse_options(&a(&["out"])).unwrap().target,
            Target::Dir("out".into())
        );
        let o = parse_options(&a(&["https://h/s/t", "out"])).unwrap();
        assert!(!o.force);
        assert_eq!(
            o.target,
            Target::Link {
                link: "https://h/s/t".into(),
                dir: Some("out".into())
            }
        );
        assert!(parse_options(&a(&["--force", "out"])).unwrap().force);
        assert!(parse_options(&a(&["--force", "https://h/s/t"])).is_err());
        assert!(parse_options(&a(&["--bogus"])).is_err());
        assert!(parse_options(&a(&["a", "b"])).is_err());
    }

    #[test]
    fn links_parse() {
        assert_eq!(
            parse_link("https://cloud.example.com/s/abc_DEF-1/").unwrap(),
            Link::Landing {
                origin: "https://cloud.example.com".into(),
                token: "abc_DEF-1".into()
            }
        );
        assert_eq!(
            parse_link("http://127.0.0.1:9/s/tok").unwrap(),
            Link::Landing {
                origin: "http://127.0.0.1:9".into(),
                token: "tok".into()
            }
        );
        assert_eq!(
            parse_link("curl-runner://open?server=http%3A%2F%2F127.0.0.1%3A9%2F&share=tok")
                .unwrap(),
            Link::Api {
                server: "http://127.0.0.1:9".into(),
                token: "tok".into()
            }
        );
        for bad in [
            "https://h.example.com/x/tok",
            "https://h.example.com/s/",
            "https://h.example.com/s/a/b",
            "https://h.example.com/s/a%2F..",
            "http://h.example.com/s/tok",
            "curl-runner://open?server=http://evil.example.com&share=t",
            "curl-runner://open?server=https://h.example.com",
            "curl-runner://close?server=https://h.example.com&share=t",
            "nonsense",
        ] {
            assert!(parse_link(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn env_example_lines() {
        let got = env_example(&[
            input(
                "BASE_URL",
                Some("Where the API lives"),
                false,
                Some("https://api.example.com"),
            ),
            input("API_KEY", Some("Your key"), true, Some("leak")),
            input("PLAIN", None, false, None),
            input("MULTI", Some("a\nb"), false, Some("x\ny")),
        ]);
        assert_eq!(
            got,
            "# Where the API lives\nBASE_URL=https://api.example.com\n# Your key\nAPI_KEY=\nPLAIN=\n# a b\nMULTI=\"x\\ny\"\n"
        );
    }

    #[test]
    fn safe_path_refuses_escapes_and_symlinks() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        for bad in [
            "../x.yaml",
            "/etc/x.yaml",
            "a/../../x.yaml",
            ".hidden.yaml",
            "C:/x.yaml",
        ] {
            assert!(safe_path(&root, bad, true).is_err(), "{bad}");
        }
        let p = safe_path(&root, "a/b.yaml", true).unwrap();
        assert_eq!(p, root.join("a").join("b.yaml"));
        assert!(root.join("a").is_dir());
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(outside.path(), root.join("link")).unwrap();
            assert!(safe_path(&root, "link/x.yaml", true).is_err());
            std::fs::write(outside.path().join("t.yaml"), "x").unwrap();
            std::os::unix::fs::symlink(outside.path().join("t.yaml"), root.join("f.yaml")).unwrap();
            assert!(safe_path(&root, "f.yaml", false).is_err());
        }
    }

    #[test]
    fn removal_ignores_paths_that_are_not_valid() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("root");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        std::fs::write(d.path().join("precious.yaml"), "keep").unwrap();
        std::fs::write(root.join("gone.yaml"), "old").unwrap();
        let old = State {
            server: "https://api.example.com".into(),
            token: "t".into(),
            version: 1,
            files: [
                ("../precious.yaml".to_string(), file_hash("keep")),
                ("gone.yaml".to_string(), file_hash("old")),
            ]
            .into(),
        };
        let snap = Snapshot {
            info: serde_json::from_value(serde_json::json!({
                "team":"acme","collection":"c","name":"C","description":null,"version":2,
                "updated_at":"2026-01-01T00:00:00Z","request_count":0,"access":"public"
            }))
            .unwrap(),
            files: vec![],
            inputs: vec![],
        };
        apply(&root, &snap, Some(&old), "https://api.example.com", "t").unwrap();
        assert!(d.path().join("precious.yaml").exists());
        assert!(!root.join("gone.yaml").exists());
        assert!(root.join(".curl-runner/pull.json").exists());
        assert!(!root.join(".curl-runner/staging").exists());
    }

    #[test]
    fn unsafe_snapshots_are_refused_whole() {
        let snap = |paths: &[&str]| Snapshot {
            info: serde_json::from_value(serde_json::json!({
                "team":"acme","collection":"c","name":"C","description":null,"version":1,
                "updated_at":"2026-01-01T00:00:00Z","request_count":0,"access":"public"
            }))
            .unwrap(),
            files: paths
                .iter()
                .map(|p| BundleFile {
                    path: p.to_string(),
                    text: "x".into(),
                })
                .collect(),
            inputs: vec![],
        };
        assert!(check_snapshot(&snap(&["a.yaml", "b/c.yml"])).is_ok());
        for bad in [
            &["a.yaml", "../x.yaml"][..],
            &["/abs.yaml"],
            &["con.yaml"],
            &["A.yaml", "a.yaml"],
            &["a.yaml", "a.yaml/b.yaml"],
        ] {
            let f = check_snapshot(&snap(bad)).unwrap_err();
            assert_eq!(f.code, 1);
            assert!(
                f.message
                    .starts_with("The server sent an unsafe file path: ")
            );
            assert!(f.message.ends_with("Nothing was written."));
        }
    }

    fn snap_of(files: &[(&str, &str)]) -> Snapshot {
        Snapshot {
            info: serde_json::from_value(serde_json::json!({
                "team":"acme","collection":"c","name":"C","description":null,"version":2,
                "updated_at":"2026-01-01T00:00:00Z","request_count":0,"access":"public"
            }))
            .unwrap(),
            files: files
                .iter()
                .map(|(p, t)| BundleFile {
                    path: p.to_string(),
                    text: t.to_string(),
                })
                .collect(),
            inputs: vec![],
        }
    }

    fn state_of(files: &[(&str, &str)]) -> State {
        State {
            server: "https://api.example.com".into(),
            token: "t".into(),
            version: 1,
            files: files
                .iter()
                .map(|(p, t)| (p.to_string(), file_hash(t)))
                .collect(),
        }
    }

    fn canon_root() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        (d, root)
    }

    #[test]
    fn a_file_already_at_the_new_content_is_not_a_local_edit() {
        let (_d, root) = canon_root();
        std::fs::write(root.join("a.yaml"), "new").unwrap();
        let old = state_of(&[("a.yaml", "old"), ("b.yaml", "b")]);
        let new = snap_of(&[("a.yaml", "new")]);
        // b.yaml is missing, so it is an edit; a.yaml is already updated.
        assert_eq!(local_edits(&root, &old, &new.files), ["b.yaml"]);
    }

    #[test]
    fn case_only_rename_keeps_the_file_under_the_new_name() {
        let (_d, root) = canon_root();
        std::fs::write(root.join("A.yaml"), "old").unwrap();
        let old = state_of(&[("A.yaml", "old")]);
        let new = snap_of(&[("a.yaml", "new")]);
        assert!(local_edits(&root, &old, &new.files).is_empty());
        apply(&root, &new, Some(&old), "https://api.example.com", "t").unwrap();
        let names: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"a.yaml".to_string()), "{names:?}");
        assert!(!names.contains(&"A.yaml".to_string()), "{names:?}");
        assert_eq!(std::fs::read_to_string(root.join("a.yaml")).unwrap(), "new");
        // Same content under the new name works too.
        let (_d2, root2) = canon_root();
        std::fs::write(root2.join("A.yaml"), "same").unwrap();
        let old = state_of(&[("A.yaml", "same")]);
        apply(
            &root2,
            &snap_of(&[("a.yaml", "same")]),
            Some(&old),
            "https://api.example.com",
            "t",
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(root2.join("a.yaml")).unwrap(),
            "same"
        );
        let st: State = serde_json::from_str(
            &std::fs::read_to_string(root2.join(".curl-runner/pull.json")).unwrap(),
        )
        .unwrap();
        assert!(st.files.contains_key("a.yaml") && !st.files.contains_key("A.yaml"));
    }

    #[cfg(unix)]
    #[test]
    fn a_planted_temp_symlink_is_not_followed() {
        let (_d, root) = canon_root();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("victim");
        std::fs::write(&target, "keep").unwrap();
        std::fs::create_dir(root.join(".curl-runner")).unwrap();
        std::os::unix::fs::symlink(&target, root.join(".curl-runner/pull.json.tmp")).unwrap();
        let r = apply(
            &root,
            &snap_of(&[("a.yaml", "x")]),
            None,
            "https://api.example.com",
            "t",
        );
        assert!(r.is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep");
        // Refused before anything moved.
        assert!(!root.join("a.yaml").exists());
    }

    #[test]
    fn a_stale_regular_temp_file_is_replaced() {
        let (_d, root) = canon_root();
        std::fs::create_dir(root.join(".curl-runner")).unwrap();
        std::fs::write(root.join(".curl-runner/pull.json.tmp"), "stale").unwrap();
        apply(
            &root,
            &snap_of(&[("a.yaml", "x")]),
            None,
            "https://api.example.com",
            "t",
        )
        .unwrap();
        assert!(root.join(".curl-runner/pull.json").exists());
    }

    #[test]
    fn unsafe_path_text_is_escaped() {
        let f = unsafe_path("a\u{1b}[2J.yaml");
        assert!(!f.message.contains('\u{1b}'));
        assert!(f.message.contains("\\u{1b}"));
    }

    #[test]
    fn plan_and_source_errors_are_plain_failures() {
        for code in ["plan_limit", "wrong_source"] {
            let body = format!(r#"{{"error":"{code}","message":"No."}}"#);
            let r = decode_ok::<Latest>(409, body.as_bytes());
            assert_eq!(r.err().unwrap().code, 1);
        }
    }
}
