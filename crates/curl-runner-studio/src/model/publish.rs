//! Publishing a folder from the studio (the PublishFirst and PublishStates
//! boards): where it publishes, what the sidebar foot says about it, and the
//! `cloud:` block it can write to `curl-runner.yaml`. The checks and the
//! request are `curl_runner_cloud_client::publish`, the CLI's own code.

use chrono::{DateTime, Utc};
use curl_runner_cloud_client::publish::Target;
use curl_runner_cloud_proto::{GitMeta, VersionSummary};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// The config file the `cloud:` block lives in.
pub const CONFIG_FILE: &str = "curl-runner.yaml";

/// Where a folder publishes: a team's collection on a server (API URL, no
/// trailing `/`). Kept in the collection's notes, not in the folder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CloudLink {
    pub server: String,
    pub team: String,
    pub slug: String,
}

/// The current version as the foot shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub version: u32,
    pub at: DateTime<Utc>,
    /// `CI`, `you`, or an email.
    pub by: String,
    /// `main@4f2a9c1`, when the version came with git details.
    pub git: Option<String>,
}

/// The sidebar foot for a folder that publishes (PublishStates, row 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishLine {
    /// Signed in, never published here.
    NotPublished,
    /// Checks and upload under way, making version `n`.
    Publishing(u32),
    /// The folder matches the current version.
    UpToDate(Published),
    /// `n` files differ from the current version.
    Behind(usize, Published),
    /// Signed out, or the session ended.
    SignIn,
    /// The collection syncs from the studio (source `cloud`).
    Studio,
    /// `Continue offline` was picked.
    Offline,
    /// The server couldn't be asked; the text says why.
    Unchecked(String),
}

impl PublishLine {
    /// The first line.
    pub fn text(&self) -> String {
        match self {
            PublishLine::NotPublished => "Not published".into(),
            PublishLine::Publishing(n) => format!("Publishing version {n}…"),
            PublishLine::UpToDate(p) => format!("Published v{}", p.version),
            PublishLine::Behind(1, _) => "1 file not published".into(),
            PublishLine::Behind(n, _) => format!("{n} files not published"),
            PublishLine::SignIn => "Sign in to publish".into(),
            PublishLine::Studio => "Syncs from the studio".into(),
            PublishLine::Offline => "Working offline".into(),
            PublishLine::Unchecked(_) => "Couldn't check Cloud".into(),
        }
    }

    /// The muted second line, if any.
    pub fn detail(&self, now: DateTime<Utc>) -> Option<String> {
        match self {
            // With git details the commit says enough: who would crowd it.
            PublishLine::UpToDate(p) => Some(match &p.git {
                Some(g) => format!("{} · {g}", ago(p.at, now)),
                None => format!("{} by {}", ago(p.at, now), p.by),
            }),
            PublishLine::Behind(_, p) => {
                Some(format!("v{} · {} by {}", p.version, ago(p.at, now), p.by))
            }
            PublishLine::Studio => Some("Two-way sync is coming soon".into()),
            PublishLine::Unchecked(why) => Some(why.clone()),
            _ => None,
        }
    }

    /// The dot's color token; `None` is the hollow ring.
    pub fn dot(&self) -> Option<&'static str> {
        match self {
            PublishLine::Publishing(_) => Some("accent-blue"),
            PublishLine::UpToDate(_) => Some("success"),
            PublishLine::Behind(..) => Some("warning"),
            PublishLine::Studio => Some("accent-purple"),
            _ => None,
        }
    }
}

/// `just now`, `5 min ago`, `2 hours ago`, `3 days ago`.
pub fn ago(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let s = (now - at).num_seconds().max(0);
    match s {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", s / 60),
        3600..7200 => "1 hour ago".into(),
        7200..86400 => format!("{} hours ago", s / 3600),
        86400..172800 => "1 day ago".into(),
        _ => format!("{} days ago", s / 86400),
    }
}

/// Who published, as the foot says it: a token's name (`token CI` is
/// `CI`), `you`, or the email.
pub fn publisher_label(publisher: &str, me: Option<&str>) -> String {
    if let Some(name) = publisher.strip_prefix("token ") {
        return name.to_string();
    }
    if me.is_some_and(|m| m.eq_ignore_ascii_case(publisher)) {
        return "you".into();
    }
    publisher.to_string()
}

/// `main@4f2a9c1`: the branch (or ref) and the short commit, either alone
/// when the other is missing.
pub fn git_label(git: &GitMeta) -> Option<String> {
    let branch = git.git_ref.as_deref().map(|r| {
        r.strip_prefix("refs/heads/")
            .or_else(|| r.strip_prefix("refs/tags/"))
            .unwrap_or(r)
            .to_string()
    });
    let commit = git
        .commit
        .as_deref()
        .map(|c| c.chars().take(7).collect::<String>());
    match (branch, commit) {
        (Some(b), Some(c)) => Some(format!("{b}@{c}")),
        (Some(b), None) => Some(b),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    }
}

/// The foot's view of a version summary.
pub fn published(v: &VersionSummary, me: Option<&str>) -> Published {
    Published {
        version: v.number,
        at: v.published_at,
        by: publisher_label(&v.publisher, me),
        git: v.git.as_ref().and_then(git_label),
    }
}

/// A collection name from a folder's: lowercase letters, digits and
/// dashes, at most 63, starting with a letter or digit (`Payments API` is
/// `payments-api`). Empty when nothing usable is left.
pub fn suggest_slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(63);
    out.trim_end_matches('-').to_string()
}

/// The folder's git details, as CI sends them: HEAD's commit, its branch and
/// the `origin` remote as an https URL (no credentials). Not a repo, or no
/// `git` on this Mac: `None`. Uncommitted changes under `root` leave the
/// commit out, since the version isn't that commit.
pub fn folder_git(root: &Path) -> Option<GitMeta> {
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    git(&["rev-parse", "--is-inside-work-tree"]).filter(|s| s == "true")?;
    let clean = git(&["status", "--porcelain", "--", "."]).is_some_and(|s| s.is_empty());
    let meta = GitMeta {
        commit: git(&["rev-parse", "HEAD"]).filter(|c| clean && !c.is_empty()),
        git_ref: git(&["symbolic-ref", "-q", "HEAD"]).filter(|r| !r.is_empty()),
        repo_url: git(&["config", "--get", "remote.origin.url"]).and_then(|u| web_url(&u)),
    };
    (meta.commit.is_some() || meta.git_ref.is_some() || meta.repo_url.is_some()).then_some(meta)
}

/// A remote as its web page: `git@github.com:acme/api.git` and
/// `https://user:token@github.com/acme/api.git` are both
/// `https://github.com/acme/api`. Anything else (a local path) is `None`.
fn web_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let (host, path) = if let Some(rest) = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("http://"))
        .or_else(|| remote.strip_prefix("ssh://"))
        .or_else(|| remote.strip_prefix("git://"))
    {
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        // An ssh port is no web port.
        let host = host.split_once(':').map_or(host, |(h, _)| h);
        (host.to_string(), path.to_string())
    } else {
        // scp form: `[user@]host:path`.
        let (authority, path) = remote.split_once(':')?;
        if authority.contains('/') || path.starts_with('/') && authority.is_empty() {
            return None;
        }
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        (host.to_string(), path.to_string())
    };
    let path = path.trim_matches('/').trim_end_matches(".git");
    let ok = |s: &str| !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '@');
    (ok(&host) && host.contains('.') && ok(path)).then(|| format!("https://{host}/{path}"))
}

/// What the dialog's `Ready to publish` card counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub requests: usize,
    pub folders: usize,
    pub bytes: usize,
}

impl Summary {
    pub fn of(target: &Target) -> Summary {
        let mut folders = std::collections::BTreeSet::new();
        let mut requests = 0;
        let mut bytes = 0;
        for f in &target.files {
            bytes += f.text.len();
            if f.path == CONFIG_FILE || f.path == "curl-runner.yml" {
                continue;
            }
            if let Some((dir, _)) = f.path.rsplit_once('/') {
                folders.insert(dir.to_string());
            }
            requests += crate::model::collection::request_lines(&f.text)
                .len()
                .max(1);
        }
        Summary {
            requests,
            folders: folders.len(),
            bytes,
        }
    }

    /// `14 requests in 5 folders`; `1 request`.
    pub fn line(&self) -> String {
        let r = if self.requests == 1 {
            "1 request".to_string()
        } else {
            format!("{} requests", self.requests)
        };
        match self.folders {
            0 => r,
            1 => format!("{r} in 1 folder"),
            n => format!("{r} in {n} folders"),
        }
    }

    /// `22 KB`; under a kilobyte, `812 B`.
    pub fn size(&self) -> String {
        if self.bytes < 1024 {
            format!("{} B", self.bytes)
        } else {
            format!("{} KB", self.bytes.div_ceil(1024))
        }
    }
}

/// Writes `collection` (and `name`, when given) into `root/curl-runner.yaml`'s
/// `cloud:` block, keeping everything else in the file as it was: other
/// keys of the block (description, inputs) and other sections. Creates the
/// file or the block when missing. The team is never written: CI names it.
pub fn save_cloud_block(root: &Path, collection: &str, name: Option<&str>) -> io::Result<()> {
    let path = root.join(CONFIG_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let out = with_cloud_block(&text, collection, name);
    crate::model::write_atomic(&path, &out)
}

fn yaml_scalar(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '/'))
        && !s.starts_with([' ', '-'])
        && !s.ends_with(' ');
    if plain {
        s.to_string()
    } else {
        serde_json::to_string(s).unwrap_or_default()
    }
}

/// [`save_cloud_block`] on text.
fn with_cloud_block(text: &str, collection: &str, name: Option<&str>) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim_end() == "cloud:") else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(
            "cloud:\n  collection: {}\n",
            yaml_scalar(collection)
        ));
        if let Some(n) = name {
            out.push_str(&format!("  name: {}\n", yaml_scalar(n)));
        }
        return out;
    };
    // The block runs while lines are indented (or blank).
    let end = lines[start + 1..]
        .iter()
        .position(|l| !l.is_empty() && !l.starts_with([' ', '\t']))
        .map_or(lines.len(), |i| start + 1 + i);
    let indent = lines[start + 1..end]
        .iter()
        .find(|l| !l.trim().is_empty())
        .map_or("  ".to_string(), |l| {
            l[..l.len() - l.trim_start().len()].to_string()
        });
    let key_of = |l: &str| -> Option<String> {
        let rest = l.strip_prefix(indent.as_str())?;
        if rest.starts_with([' ', '\t']) {
            return None;
        }
        rest.split_once(':').map(|(k, _)| k.trim().to_string())
    };
    let mut block: Vec<String> = Vec::new();
    let (mut wrote_c, mut wrote_n) = (false, false);
    for l in &lines[start + 1..end] {
        match key_of(l).as_deref() {
            Some("collection") => {
                block.push(format!("{indent}collection: {}", yaml_scalar(collection)));
                wrote_c = true;
            }
            Some("name") if name.is_some() => {
                block.push(format!(
                    "{indent}name: {}",
                    yaml_scalar(name.unwrap_or_default())
                ));
                wrote_n = true;
            }
            _ => block.push((*l).to_string()),
        }
    }
    let mut head = Vec::new();
    if !wrote_c {
        head.push(format!("{indent}collection: {}", yaml_scalar(collection)));
    }
    if let (Some(n), false) = (name, wrote_n) {
        head.push(format!("{indent}name: {}", yaml_scalar(n)));
    }
    let mut out: Vec<String> = lines[..=start].iter().map(|l| l.to_string()).collect();
    out.extend(head);
    out.extend(block);
    out.extend(lines[end..].iter().map(|l| l.to_string()));
    let mut s = out.join("\n");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: i64) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc.with_ymd_and_hms(2026, 11, 12, 12, 0, 0).unwrap();
        (now - chrono::Duration::hours(h), now)
    }

    #[test]
    fn foot_lines_follow_the_board() {
        let (t, now) = at(2);
        let p = Published {
            version: 14,
            at: t,
            by: "CI".into(),
            git: Some("main@4f2a9c1".into()),
        };
        let up = PublishLine::UpToDate(p.clone());
        assert_eq!(up.text(), "Published v14");
        assert_eq!(
            up.detail(now).as_deref(),
            Some("2 hours ago · main@4f2a9c1")
        );
        let behind = PublishLine::Behind(3, p);
        assert_eq!(behind.text(), "3 files not published");
        assert_eq!(
            behind.detail(now).as_deref(),
            Some("v14 · 2 hours ago by CI")
        );
        assert_eq!(PublishLine::Publishing(15).text(), "Publishing version 15…");
        assert_eq!(PublishLine::NotPublished.text(), "Not published");
        assert_eq!(PublishLine::NotPublished.dot(), None);
        assert_eq!(
            PublishLine::Studio.detail(now).as_deref(),
            Some("Two-way sync is coming soon")
        );
    }

    #[test]
    fn publisher_and_git_labels() {
        assert_eq!(publisher_label("token CI", None), "CI");
        assert_eq!(publisher_label("Ada@x.test", Some("ada@x.test")), "you");
        assert_eq!(
            publisher_label("bo@x.test", Some("ada@x.test")),
            "bo@x.test"
        );
        let g = |c: Option<&str>, r: Option<&str>| GitMeta {
            commit: c.map(Into::into),
            git_ref: r.map(Into::into),
            repo_url: None,
        };
        assert_eq!(
            git_label(&g(Some("4f2a9c1aa0"), Some("refs/heads/main"))).as_deref(),
            Some("main@4f2a9c1")
        );
        assert_eq!(git_label(&g(None, Some("v1"))).as_deref(), Some("v1"));
        assert_eq!(git_label(&g(None, None)), None);
    }

    #[test]
    fn slugs_from_folder_names() {
        assert_eq!(suggest_slug("Payments API"), "payments-api");
        assert_eq!(suggest_slug("--My_Collection!!"), "my-collection");
        assert_eq!(suggest_slug("ÀÉ"), "");
        assert_eq!(suggest_slug(&"a".repeat(80)).len(), 63);
    }

    #[test]
    fn ago_steps() {
        let now = Utc.with_ymd_and_hms(2026, 11, 12, 12, 0, 0).unwrap();
        let m = |s: i64| ago(now - chrono::Duration::seconds(s), now);
        assert_eq!(m(10), "just now");
        assert_eq!(m(300), "5 min ago");
        assert_eq!(m(3700), "1 hour ago");
        assert_eq!(m(3 * 86400), "3 days ago");
    }

    #[test]
    fn cloud_block_created_updated_and_kept() {
        assert_eq!(
            with_cloud_block("", "pay", Some("Payments API")),
            "cloud:\n  collection: pay\n  name: Payments API\n"
        );
        let before = "global:\n  timeout: 5\ncloud:\n  collection: old\n  description: Charges\n  inputs:\n    - name: TOKEN\nother: 1\n";
        assert_eq!(
            with_cloud_block(before, "pay", Some("Pay: v2")),
            "global:\n  timeout: 5\ncloud:\n  name: \"Pay: v2\"\n  collection: pay\n  description: Charges\n  inputs:\n    - name: TOKEN\nother: 1\n"
        );
        assert_eq!(
            with_cloud_block("global:\n  timeout: 5", "pay", None),
            "global:\n  timeout: 5\ncloud:\n  collection: pay\n"
        );
    }

    #[test]
    fn remotes_become_web_urls_without_credentials() {
        for (remote, want) in [
            (
                "git@github.com:acme/api.git",
                Some("https://github.com/acme/api"),
            ),
            (
                "https://user:tok3n@github.com/acme/api.git",
                Some("https://github.com/acme/api"),
            ),
            (
                "ssh://git@gitlab.com:2222/acme/sub/api.git",
                Some("https://gitlab.com/acme/sub/api"),
            ),
            (
                "https://github.com/acme/api",
                Some("https://github.com/acme/api"),
            ),
            ("/srv/git/api.git", None),
            ("../api", None),
            ("C:/repos/api", None),
        ] {
            assert_eq!(web_url(remote).as_deref(), want, "{remote}");
        }
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.test")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.test")
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn folder_git_reads_commit_branch_and_remote() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(folder_git(d.path()), None);
        git(d.path(), &["init", "-q", "-b", "main"]);
        std::fs::write(d.path().join("a.yaml"), "x: 1\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-q", "-m", "one"]);
        git(
            d.path(),
            &[
                "remote",
                "add",
                "origin",
                "https://u:secret@github.com/acme/api.git",
            ],
        );
        let g = folder_git(d.path()).unwrap();
        assert_eq!(g.commit.as_deref().map(str::len), Some(40));
        assert_eq!(g.git_ref.as_deref(), Some("refs/heads/main"));
        assert_eq!(g.repo_url.as_deref(), Some("https://github.com/acme/api"));
        // Uncommitted changes: not that commit any more.
        std::fs::write(d.path().join("a.yaml"), "x: 2\n").unwrap();
        let dirty = folder_git(d.path()).unwrap();
        assert_eq!(
            (dirty.commit, dirty.git_ref.as_deref()),
            (None, Some("refs/heads/main"))
        );
    }

    #[test]
    fn saved_block_loads_back() {
        let d = tempfile::tempdir().unwrap();
        save_cloud_block(d.path(), "payments-api", Some("Payments API")).unwrap();
        let cfg = curl_runner_core::cloud::load_cloud_config(d.path())
            .unwrap()
            .unwrap();
        assert_eq!(cfg.collection, "payments-api");
        assert_eq!(cfg.name.as_deref(), Some("Payments API"));
    }
}
