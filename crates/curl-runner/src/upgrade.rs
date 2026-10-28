//! `curl-runner upgrade [--dry-run|-n] [--force|-f]` (`commands/upgrade.ts`).
//!
//! npm/bun installs run the same package-manager command as TS. A standalone
//! binary upgrades itself: it downloads the release archive and
//! `SHA256SUMS.txt` from GitHub, verifies the checksum, then replaces the
//! running executable atomically (write next to it, rename; on Windows the
//! old file is renamed aside first). Any failure leaves the binary untouched.
//!
//! `CURL_RUNNER_TEST_REGISTRY_URL` and `CURL_RUNNER_TEST_GITHUB_BASE` (which
//! replaces both `https://api.github.com` and `https://github.com`) exist for
//! tests only.

use crate::colors::{Color, colorize};
use crate::install::{self, InstallMethod};
use crate::output::Console;
use crate::version_check::{self, HOOKS_ENABLED, REGISTRY_ENV, REGISTRY_URL, hook, is_newer};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const GITHUB_ENV: &str = "CURL_RUNNER_TEST_GITHUB_BASE";
const GITHUB_API: &str = "https://api.github.com";
const GITHUB_WEB: &str = "https://github.com";
const REPO: &str = "alexvcasillas/curl-runner";
const INSTALL_SH: &str = "curl -fsSL https://www.curl-runner.com/install.sh | bash";
const INSTALL_PS1: &str = "irm https://www.curl-runner.com/install.ps1 | iex";
const API_TIMEOUT: Duration = Duration::from_secs(5);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_ARCHIVE: usize = 300 << 20;
const MAX_BINARY: u64 = 600 << 20;

#[derive(Debug, Default, PartialEq)]
struct Options {
    dry_run: bool,
    force: bool,
}

fn parse_args(args: &[String]) -> Options {
    let mut o = Options::default();
    for a in args {
        match a.as_str() {
            "--dry-run" | "-n" => o.dry_run = true,
            "--force" | "-f" => o.force = true,
            _ => {}
        }
    }
    o
}

/// Everything the command reads from its surroundings.
pub struct Ctx {
    pub current: String,
    pub exe: PathBuf,
    pub method: InstallMethod,
    pub registry_url: String,
    pub github_api: String,
    pub github_web: String,
    pub windows: bool,
    pub platform: Option<String>,
}

impl Ctx {
    pub fn from_env(env: &dyn Fn(&str) -> Option<String>) -> Ctx {
        let exe = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .unwrap_or_default();
        let gh = hook(env, GITHUB_ENV, HOOKS_ENABLED).map(|b| b.trim_end_matches('/').to_string());
        Ctx {
            current: curl_runner_core::VERSION.to_string(),
            method: install::detect(&exe, env),
            exe,
            registry_url: hook(env, REGISTRY_ENV, HOOKS_ENABLED)
                .unwrap_or_else(|| REGISTRY_URL.to_string()),
            github_api: gh.clone().unwrap_or_else(|| GITHUB_API.to_string()),
            github_web: gh.unwrap_or_else(|| GITHUB_WEB.to_string()),
            windows: cfg!(windows),
            platform: platform(),
        }
    }
}

/// Release platform name (`linux-x64`, `darwin-arm64`, `windows-x64`, ...).
pub fn platform() -> Option<String> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

struct Paint(bool);
impl Paint {
    fn c(&self, t: &str, c: Color) -> String {
        if self.0 {
            colorize(t, c)
        } else {
            t.to_string()
        }
    }
}

fn format_method(m: InstallMethod) -> &'static str {
    match m {
        InstallMethod::Bun => "bun (global)",
        InstallMethod::Npm => "npm (global)",
        InstallMethod::Cargo => "cargo (crates.io)",
        InstallMethod::Standalone => "standalone binary",
        InstallMethod::Unknown => "unknown (development build)",
    }
}

fn manual_command(ctx: &Ctx) -> String {
    match install::upgrade_command(ctx.method) {
        Some(c) => c.to_string(),
        None if ctx.windows => INSTALL_PS1.to_string(),
        None => INSTALL_SH.to_string(),
    }
}

/// Entry point; returns the exit code. `args` are the words after `upgrade`.
pub fn run(args: &[String], color: bool, ctx: &Ctx, out: &mut Console) -> i32 {
    let o = parse_args(args);
    let p = Paint(color);
    out.out("");
    out.out(&p.c("curl-runner upgrade", Color::Bright));
    out.out("");
    out.out(&format!(
        "{} {}",
        p.c("Installation:", Color::Cyan),
        format_method(ctx.method)
    ));
    out.out(&format!(
        "{} {}",
        p.c("Current version:", Color::Cyan),
        ctx.current
    ));

    let latest = match latest_version(ctx) {
        Some(v) => v,
        None => {
            out.out(&p.c("Failed to fetch latest version", Color::Red));
            return 1;
        }
    };
    out.out(&format!("{} {latest}", p.c("Latest version:", Color::Cyan)));
    out.out("");

    if !o.force && !is_newer(&ctx.current, &latest) {
        out.out(&p.c("Already up to date!", Color::Green));
        return 0;
    }
    if ctx.method == InstallMethod::Unknown {
        out.out(&p.c(
            "Cannot upgrade: this executable is a development build, lives in Homebrew's tree or its location is unknown.",
            Color::Red,
        ));
        out.out(&format!("  {}", p.c(&manual_command(ctx), Color::Cyan)));
        return 1;
    }

    if o.dry_run {
        out.out(&p.c("Dry run - would execute:", Color::Yellow));
        let what = match install::upgrade_command(ctx.method) {
            Some(c) => c.to_string(),
            None => match standalone_plan(ctx, &latest) {
                Ok(pl) => format!(
                    "download {} and replace {}",
                    pl.archive_url,
                    ctx.exe.display()
                ),
                Err(e) => e.0,
            },
        };
        out.out(&format!("  {}", p.c(&what, Color::Cyan)));
        return 0;
    }

    out.out(&p.c("Upgrading...", Color::Yellow));
    out.out("");
    let res = match install::upgrade_command(ctx.method) {
        Some(cmd) => run_package_manager(cmd, ctx.windows),
        None => standalone_upgrade(ctx, &latest, out),
    };
    match res {
        Ok(()) => {
            out.out("");
            out.out(&p.c("Upgrade complete!", Color::Green));
            out.out("");
            out.out("Run `curl-runner --version` to verify.");
            0
        }
        Err(e) => {
            let manual = manual_command(ctx);
            if e.0.contains("EACCES") || e.0.contains("permission") {
                out.out("");
                out.out(&p.c("Permission denied. Try running with sudo:", Color::Yellow));
                out.out(&format!(
                    "  {}",
                    p.c(&format!("sudo {}", sudo_target(ctx, &manual)), Color::Cyan)
                ));
                return 1;
            }
            out.out(&p.c(&format!("Upgrade failed: {}", e.0), Color::Red));
            out.out("");
            out.out(&p.c("Manual upgrade:", Color::Yellow));
            out.out(&format!("  {}", p.c(&manual, Color::Cyan)));
            1
        }
    }
}

/// A standalone binary retries itself under sudo; package managers re-run their command.
fn sudo_target(ctx: &Ctx, manual: &str) -> String {
    if ctx.method == InstallMethod::Standalone {
        "curl-runner upgrade".to_string()
    } else {
        manual.to_string()
    }
}

#[derive(Debug)]
pub struct UpgradeError(pub String);

impl From<std::io::Error> for UpgradeError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            UpgradeError(format!("permission denied: {e}"))
        } else {
            UpgradeError(e.to_string())
        }
    }
}

fn latest_version(ctx: &Ctx) -> Option<String> {
    if matches!(ctx.method, InstallMethod::Bun | InstallMethod::Npm) {
        return version_check::fetch_latest_with(&ctx.registry_url, API_TIMEOUT);
    }
    let url = format!("{}/repos/{REPO}/releases/latest", ctx.github_api);
    let body = crate::net::get(
        &url,
        API_TIMEOUT,
        1 << 20,
        &["Accept: application/vnd.github.v3+json"],
    )
    .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let tag = v.get("tag_name")?.as_str()?;
    // A tag without x.y.z is not a CLI release: do not build URLs from it.
    let re = regex::Regex::new(r"\d+\.\d+\.\d+").ok()?;
    re.find(tag).map(|m| m.as_str().to_string())
}

fn run_package_manager(cmd: &str, windows: bool) -> Result<(), UpgradeError> {
    let mut parts = cmd.split(' ');
    let prog = parts.next().unwrap_or_default();
    let mut c = if windows {
        // npm/bun are `.cmd` shims on Windows; only the shell resolves them.
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", prog]);
        c
    } else {
        std::process::Command::new(prog)
    };
    c.args(parts);
    let status = c
        .status()
        .map_err(|e| UpgradeError(format!("{prog}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map_or("signal".to_string(), |c| c.to_string());
        Err(UpgradeError(format!(
            "{prog} install failed with exit code {code}"
        )))
    }
}

pub struct Plan {
    pub tag: String,
    pub archive: String,
    pub archive_url: String,
    pub sums_url: String,
    pub binary: String,
}

pub fn standalone_plan(ctx: &Ctx, version: &str) -> Result<Plan, UpgradeError> {
    let platform = ctx.platform.clone().ok_or_else(|| {
        UpgradeError(format!(
            "no prebuilt binary for {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;
    let ext = if platform.starts_with("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let archive = format!("curl-runner-cli-{version}-{platform}.{ext}");
    let tag = format!("@curl-runner/cli@{version}");
    let base = format!("{}/{REPO}/releases/download/{tag}", ctx.github_web);
    let exe = if platform.starts_with("windows") {
        ".exe"
    } else {
        ""
    };
    Ok(Plan {
        archive_url: format!("{base}/{archive}"),
        sums_url: format!("{base}/SHA256SUMS.txt"),
        binary: format!("curl-runner-{platform}{exe}"),
        archive,
        tag,
    })
}

fn standalone_upgrade(ctx: &Ctx, version: &str, out: &mut Console) -> Result<(), UpgradeError> {
    let plan = standalone_plan(ctx, version)?;
    out.out(&format!("Downloading {} ({})", plan.archive, plan.tag));
    let archive = crate::net::get(&plan.archive_url, DOWNLOAD_TIMEOUT, MAX_ARCHIVE, &[])
        .map_err(|e| UpgradeError(format!("download failed: {e}")))?;
    let sums = crate::net::get(&plan.sums_url, API_TIMEOUT, 1 << 20, &[])
        .map_err(|e| UpgradeError(format!("download failed: {e}")))?;
    verify_checksum(&archive, &plan.archive, &String::from_utf8_lossy(&sums))?;
    out.out("Checksum verified");
    let bin = extract_binary(&archive, &plan.archive, &plan.binary)?;
    replace_binary(&ctx.exe, &bin, ctx.windows)?;
    Ok(())
}

/// Check `data` against the `SHA256SUMS.txt` line for `name`.
pub fn verify_checksum(data: &[u8], name: &str, sums: &str) -> Result<(), UpgradeError> {
    let expected = sums
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (h, n) = (it.next()?, it.next()?);
            (n.trim_start_matches('*') == name).then(|| h.to_ascii_lowercase())
        })
        .next()
        .ok_or_else(|| UpgradeError(format!("no checksum for {name} in SHA256SUMS.txt")))?;
    let actual = Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if actual == expected {
        Ok(())
    } else {
        Err(UpgradeError(format!(
            "checksum mismatch for {name} (expected {expected}, got {actual})"
        )))
    }
}

fn read_limited(r: impl Read) -> Result<Vec<u8>, UpgradeError> {
    let mut buf = Vec::new();
    r.take(MAX_BINARY + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_BINARY {
        return Err(UpgradeError("binary in archive is too large".into()));
    }
    Ok(buf)
}

/// The executable named `binary` inside a `.tar.gz` or `.zip` archive.
pub fn extract_binary(
    archive: &[u8],
    archive_name: &str,
    binary: &str,
) -> Result<Vec<u8>, UpgradeError> {
    let missing = || UpgradeError(format!("{binary} not found in {archive_name}"));
    let bad = |e: &dyn std::fmt::Display| UpgradeError(format!("cannot read {archive_name}: {e}"));
    if archive_name.ends_with(".zip") {
        let mut z = zip::ZipArchive::new(Cursor::new(archive)).map_err(|e| bad(&e))?;
        for i in 0..z.len() {
            let f = z.by_index(i).map_err(|e| bad(&e))?;
            if f.is_file()
                && f.name()
                    .is_ok_and(|n| Path::new(&*n).file_name().is_some_and(|x| x == binary))
            {
                return read_limited(f);
            }
        }
        return Err(missing());
    }
    let mut t = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for entry in t.entries().map_err(|e| bad(&e))? {
        let entry = entry.map_err(|e| bad(&e))?;
        let is_bin = entry.header().entry_type().is_file()
            && entry
                .path()
                .is_ok_and(|p| p.file_name().is_some_and(|n| n == binary));
        if is_bin {
            return read_limited(entry);
        }
    }
    Err(missing())
}

/// Put `bytes` in place of `exe`: write a sibling temp file, then rename.
/// On Windows (`windows`), a running exe cannot be overwritten, so it is
/// renamed aside first and restored if the final rename fails. The temp file
/// is removed on any failure, leaving `exe` as it was.
pub fn replace_binary(exe: &Path, bytes: &[u8], windows: bool) -> Result<(), UpgradeError> {
    let dir = exe
        .parent()
        .ok_or_else(|| UpgradeError("cannot locate the executable's directory".into()))?;
    let name = exe.file_name().unwrap_or_default().to_string_lossy();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dir.join(format!(".{name}.upgrade-{}-{nanos}", std::process::id()));
    let result = (|| -> Result<(), UpgradeError> {
        // `create_new`: never write through a pre-planted file or symlink.
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        copy_mode(exe, &tmp);
        if windows {
            let old = dir.join(format!("{name}.old"));
            let _ = std::fs::remove_file(&old);
            std::fs::rename(exe, &old)?;
            if let Err(e) = std::fs::rename(&tmp, exe) {
                let _ = std::fs::rename(&old, exe);
                return Err(e.into());
            }
            let _ = std::fs::remove_file(&old);
        } else {
            std::fs::rename(&tmp, exe)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(unix)]
fn copy_mode(from: &Path, to: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(from).map_or(0o755, |m| m.permissions().mode() & 0o7777);
    let _ = std::fs::set_permissions(to, std::fs::Permissions::from_mode(mode | 0o111));
}

#[cfg(not(unix))]
fn copy_mode(_from: &Path, _to: &Path) {}

/// `showUpgradeHelp`.
pub fn help(color: bool, out: &mut Console) {
    let p = Paint(color);
    let y = |t: &str| p.c(t, Color::Yellow);
    out.out(&format!(
        "
{}

Automatically upgrade curl-runner to the latest version.
Detects installation source (bun, npm, curl) and uses appropriate method.

{}
  curl-runner upgrade [options]

{}
  -n, --dry-run    Show what would be executed without running
  -f, --force      Force upgrade even if already on latest version
  -h, --help       Show this help message

{}
  curl-runner upgrade           # Upgrade to latest
  curl-runner upgrade --dry-run # Preview upgrade command
  curl-runner upgrade --force   # Force reinstall latest
",
        p.c("curl-runner upgrade", Color::Bright),
        y("USAGE:"),
        y("OPTIONS:"),
        y("EXAMPLES:")
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use testkit::{Response, TestServer};

    const NEW: &[u8] = b"#!/bin/sh\necho new\n";

    fn targz(entry: &str, data: &[u8]) -> Vec<u8> {
        let mut tar_bytes = Vec::new();
        {
            let mut b = tar::Builder::new(&mut tar_bytes);
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, entry, data).unwrap();
            b.finish().unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar_bytes).unwrap();
        gz.finish().unwrap()
    }

    fn zipped(entry: &str, data: &[u8]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        z.start_file(entry, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(data).unwrap();
        z.finish().unwrap().into_inner()
    }

    fn sha(d: &[u8]) -> String {
        Sha256::digest(d)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Fake GitHub: latest tag, the archive and the sums file; records paths.
    fn github(
        version: &str,
        platform: &str,
        archive: Vec<u8>,
        sum: String,
    ) -> (TestServer, Arc<Mutex<Vec<String>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        let (version, platform) = (version.to_string(), platform.to_string());
        let name = format!("curl-runner-cli-{version}-{platform}.tar.gz");
        let srv = TestServer::with_handler(move |r| {
            s.lock().unwrap().push(r.url.clone());
            if r.url == "/repos/alexvcasillas/curl-runner/releases/latest" {
                let body = format!(r#"{{"tag_name":"@curl-runner/cli@{version}"}}"#);
                return Response::from_data(body.into_bytes());
            }
            let base =
                format!("/alexvcasillas/curl-runner/releases/download/@curl-runner/cli@{version}/");
            if r.url == format!("{base}{name}") {
                return Response::from_data(archive.clone());
            }
            if r.url == format!("{base}SHA256SUMS.txt") {
                return Response::from_data(
                    format!("{sum}  {name}\n{}  other.zip\n", "0".repeat(64)).into_bytes(),
                );
            }
            Response::from_data(b"nf".to_vec()).with_status_code(404)
        });
        (srv, seen)
    }

    fn ctx(srv: &TestServer, exe: PathBuf, method: InstallMethod, current: &str) -> Ctx {
        Ctx {
            current: current.into(),
            exe,
            method,
            registry_url: format!("{}/registry", srv.base_url),
            github_api: srv.base_url.clone(),
            github_web: srv.base_url.clone(),
            windows: false,
            platform: Some("linux-x64".into()),
        }
    }

    fn run_capture(args: &[&str], c: &Ctx) -> (i32, String) {
        let (mut console, cap) = Console::capture();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let code = run(&args, false, c, &mut console);
        (code, cap.stdout())
    }

    fn installed(dir: &Path) -> PathBuf {
        let exe = dir.join("curl-runner");
        std::fs::write(&exe, b"OLD").unwrap();
        exe
    }

    #[test]
    fn standalone_upgrade_replaces_binary() {
        let archive = targz("curl-runner-linux-x64", NEW);
        let (srv, seen) = github("9.9.9", "linux-x64", archive.clone(), sha(&archive));
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        let c = ctx(&srv, exe.clone(), InstallMethod::Standalone, "1.0.0");
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 0, "{out}");
        assert_eq!(std::fs::read(&exe).unwrap(), NEW);
        assert!(out.contains("Latest version: 9.9.9") && out.contains("Upgrade complete!"));
        assert!(
            seen.lock()
                .unwrap()
                .iter()
                .any(|u| u.ends_with("SHA256SUMS.txt"))
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111,
                0
            );
        }
        // Nothing but the binary is left behind.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn standalone_verifies_checksum() {
        let archive = targz("curl-runner-linux-x64", NEW);
        let (srv, _) = github("9.9.9", "linux-x64", archive, "a".repeat(64));
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        let c = ctx(&srv, exe.clone(), InstallMethod::Standalone, "1.0.0");
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("checksum mismatch"), "{out}");
        assert!(out.contains("Manual upgrade:"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn missing_checksum_entry_or_archive_leaves_binary() {
        let archive = targz("curl-runner-linux-x64", NEW);
        let (srv, _) = github("9.9.9", "linux-x64", archive.clone(), sha(&archive));
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        // The server only has linux-x64: darwin-arm64 gives a 404.
        let mut c = ctx(&srv, exe.clone(), InstallMethod::Standalone, "1.0.0");
        c.platform = Some("darwin-arm64".into());
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("Upgrade failed: download failed"), "{out}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
    }

    #[test]
    fn wrong_entry_in_archive_leaves_binary() {
        let archive = targz("something-else", NEW);
        let (srv, _) = github("9.9.9", "linux-x64", archive.clone(), sha(&archive));
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        let c = ctx(&srv, exe.clone(), InstallMethod::Standalone, "1.0.0");
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("curl-runner-linux-x64 not found"), "{out}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
    }

    #[test]
    fn up_to_date_and_force_and_dry_run() {
        let archive = targz("curl-runner-linux-x64", NEW);
        let (srv, seen) = github("1.0.0", "linux-x64", archive.clone(), sha(&archive));
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        let c = ctx(&srv, exe.clone(), InstallMethod::Standalone, "1.0.0");
        let (code, out) = run_capture(&[], &c);
        assert_eq!((code, out.contains("Already up to date!")), (0, true));
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
        // Dry run: reports the download, changes nothing, downloads nothing.
        let n = seen.lock().unwrap().len();
        let (code, out) = run_capture(&["--force", "--dry-run"], &c);
        assert_eq!(code, 0);
        assert!(
            out.contains("Dry run - would execute:")
                && out.contains("curl-runner-cli-1.0.0-linux-x64.tar.gz"),
            "{out}"
        );
        assert_eq!(seen.lock().unwrap().len(), n + 1); // only the latest-version lookup
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
        // Force reinstalls the same version.
        let (code, _) = run_capture(&["-f"], &c);
        assert_eq!(code, 0);
        assert_eq!(std::fs::read(&exe).unwrap(), NEW);
    }

    #[test]
    fn npm_and_bun_dry_run_print_ts_commands() {
        let srv =
            TestServer::with_handler(|_| Response::from_data(br#"{"version":"9.9.9"}"#.to_vec()));
        for (m, cmd, label) in [
            (
                InstallMethod::Npm,
                "npm install -g @curl-runner/cli@latest",
                "npm (global)",
            ),
            (
                InstallMethod::Bun,
                "bun install -g @curl-runner/cli@latest",
                "bun (global)",
            ),
        ] {
            let c = ctx(&srv, PathBuf::from("/x/node_modules/y"), m, "1.0.0");
            let (code, out) = run_capture(&["-n"], &c);
            assert_eq!(code, 0);
            assert!(out.contains(&format!("Installation: {label}")), "{out}");
            assert!(out.contains(&format!("  {cmd}")), "{out}");
        }
    }

    #[test]
    fn fetch_failure_and_unknown_install_exit_1() {
        let srv =
            TestServer::with_handler(|_| Response::from_data(b"x".to_vec()).with_status_code(500));
        let c = ctx(
            &srv,
            PathBuf::from("/x/curl-runner"),
            InstallMethod::Standalone,
            "1.0.0",
        );
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("Failed to fetch latest version"));
        let ok =
            TestServer::with_handler(|_| Response::from_data(br#"{"tag_name":"v9.0.0"}"#.to_vec()));
        let c = ctx(
            &ok,
            PathBuf::from("/x/target/debug/curl-runner"),
            InstallMethod::Unknown,
            "1.0.0",
        );
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("Cannot upgrade"), "{out}");
    }

    #[test]
    fn github_tag_without_version_is_rejected() {
        let srv = TestServer::with_handler(|_| {
            Response::from_data(br#"{"tag_name":"nightly"}"#.to_vec())
        });
        let c = ctx(
            &srv,
            PathBuf::from("/x/curl-runner"),
            InstallMethod::Standalone,
            "1.0.0",
        );
        let (code, out) = run_capture(&[], &c);
        assert_eq!(code, 1);
        assert!(out.contains("Failed to fetch latest version"), "{out}");
    }

    #[test]
    fn release_builds_ignore_test_hooks() {
        let env = |k: &str| Some(format!("http://evil/{k}"));
        assert_eq!(hook(&env, GITHUB_ENV, false), None);
        assert!(hook(&env, GITHUB_ENV, true).is_some());
        // Ctx reads them through HOOKS_ENABLED (true in this debug test build).
        let c = Ctx::from_env(&env);
        assert_eq!(HOOKS_ENABLED, c.github_api.starts_with("http://evil"));
    }

    #[test]
    fn checksum_file_formats() {
        let d = b"hello";
        let h = sha(d);
        assert!(verify_checksum(d, "a.tgz", &format!("{h}  a.tgz\n")).is_ok());
        assert!(verify_checksum(d, "a.tgz", &format!("{} *a.tgz\n", h.to_uppercase())).is_ok());
        assert!(verify_checksum(d, "a.tgz", &format!("{h}  b.tgz\n")).is_err());
        assert!(verify_checksum(d, "a.tgz", "").is_err());
    }

    #[test]
    fn extracts_from_zip_and_nested_tar() {
        let z = zipped("dist/curl-runner-windows-x64.exe", NEW);
        assert_eq!(
            extract_binary(&z, "x.zip", "curl-runner-windows-x64.exe").unwrap(),
            NEW
        );
        assert!(extract_binary(&z, "x.zip", "nope.exe").is_err());
        let t = targz("pkg/curl-runner-linux-x64", NEW);
        assert_eq!(
            extract_binary(&t, "x.tar.gz", "curl-runner-linux-x64").unwrap(),
            NEW
        );
        assert!(extract_binary(b"garbage", "x.tar.gz", "b").is_err());
        assert!(extract_binary(b"garbage", "x.zip", "b").is_err());
    }

    #[test]
    fn windows_replace_renames_aside_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("curl-runner.exe");
        std::fs::write(&exe, b"OLD").unwrap();
        std::fs::write(dir.path().join("curl-runner.exe.old"), b"stale").unwrap();
        replace_binary(&exe, NEW, true).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), NEW);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn failed_replace_leaves_binary_and_no_temp() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let exe = installed(dir.path());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let r = replace_binary(&exe, NEW, false);
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        if let Err(e) = r {
            assert!(e.0.contains("permission"), "{}", e.0);
            assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        } // (running as root: the write succeeds, nothing to check)
    }

    #[test]
    fn platform_names() {
        if let Some(p) = platform() {
            assert!(
                ["linux", "darwin", "windows"]
                    .iter()
                    .any(|o| p.starts_with(o))
            );
            assert!(p.ends_with("x64") || p.ends_with("arm64"));
        }
    }

    #[test]
    fn args() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(
            a(&["-n", "-f"]),
            Options {
                dry_run: true,
                force: true
            }
        );
        assert_eq!(
            a(&["--dry-run"]),
            Options {
                dry_run: true,
                force: false
            }
        );
        assert_eq!(a(&["--bogus"]), Options::default());
    }
}
