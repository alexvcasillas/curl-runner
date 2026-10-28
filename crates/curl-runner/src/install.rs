//! Install-method detection (`utils/installation-detector.ts`), extended for
//! the npm layout where the binary lives in a platform package
//! (`node_modules/@curl-runner/cli-<platform>/bin/`, exec'd by the npm
//! package's `bin/curl-runner` launcher).

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    Npm,
    Bun,
    /// `cargo install curl-runner` (`$CARGO_HOME/bin`, default `~/.cargo/bin`).
    Cargo,
    /// A downloaded binary (TS `curl` installer and `standalone`).
    Standalone,
    /// A development build or an unreadable path: never upgraded.
    Unknown,
}

/// Classify the running executable. Path checks come first (bun, then any
/// `node_modules` tree, which covers the `@curl-runner/cli-<platform>`
/// packages of npm, pnpm and yarn); the curl installer's `~/.local/bin` is
/// standalone even when npm variables are set in the shell; only then the
/// TS env heuristics (`npm_config_prefix`, `npm_execpath`) apply.
pub fn detect(exe: &Path, env: &dyn Fn(&str) -> Option<String>) -> InstallMethod {
    let p = exe.to_string_lossy().replace('\\', "/");
    if p.is_empty() || is_dev_build(&p) {
        return InstallMethod::Unknown;
    }
    if is_bun(&p, env) {
        return InstallMethod::Bun;
    }
    if p.contains("node_modules") {
        return InstallMethod::Npm;
    }
    if is_cargo(&p, env) {
        return InstallMethod::Cargo;
    }
    // Never self-replace inside Homebrew's tree.
    if p.contains("/Cellar/") {
        return InstallMethod::Unknown;
    }
    if p.contains("/.local/bin/") {
        return InstallMethod::Standalone;
    }
    if env("npm_config_prefix").is_some_and(|v| !v.is_empty())
        || env("npm_execpath").is_some_and(|v| !v.is_empty())
    {
        return InstallMethod::Npm;
    }
    InstallMethod::Standalone
}

fn is_dev_build(p: &str) -> bool {
    p.contains("/target/debug/") || p.contains("/target/release/")
}

fn is_cargo(p: &str, env: &dyn Fn(&str) -> Option<String>) -> bool {
    if p.contains("/.cargo/bin/") {
        return true;
    }
    env("CARGO_HOME")
        .filter(|h| !h.is_empty())
        .is_some_and(|h| {
            p.starts_with(&format!(
                "{}/bin/",
                h.replace('\\', "/").trim_end_matches('/')
            ))
        })
}

fn is_bun(p: &str, env: &dyn Fn(&str) -> Option<String>) -> bool {
    if p.contains(".bun") {
        return true;
    }
    if let Some(b) = env("BUN_INSTALL").filter(|b| !b.is_empty())
        && p.contains(&b.replace('\\', "/"))
    {
        return true;
    }
    ["/bun/install/", "/.bun/bin/", "/bun/bin/"]
        .iter()
        .any(|s| p.contains(s))
}

/// The package-manager command TS runs per method (same on Windows). The
/// standalone binary upgrades itself in-process.
pub fn upgrade_command(m: InstallMethod) -> Option<&'static str> {
    match m {
        InstallMethod::Bun => Some("bun install -g @curl-runner/cli@latest"),
        InstallMethod::Npm => Some("npm install -g @curl-runner/cli@latest"),
        InstallMethod::Cargo => Some("cargo install curl-runner --locked"),
        InstallMethod::Standalone | InstallMethod::Unknown => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn no_env(_: &str) -> Option<String> {
        None
    }
    fn d(p: &str) -> InstallMethod {
        detect(&PathBuf::from(p), &no_env)
    }

    #[test]
    fn detects_npm_platform_package_path() {
        assert_eq!(
            d("/usr/local/lib/node_modules/@curl-runner/cli-darwin-arm64/bin/curl-runner"),
            InstallMethod::Npm
        );
        assert_eq!(
            d(
                "/home/u/.nvm/versions/node/v22/lib/node_modules/@curl-runner/cli-linux-x64/bin/curl-runner"
            ),
            InstallMethod::Npm
        );
        assert_eq!(
            d(
                r"C:\Users\u\AppData\Roaming\npm\node_modules\@curl-runner\cli-win32-x64\bin\curl-runner.exe"
            ),
            InstallMethod::Npm
        );
        // pnpm and yarn global layouts are node_modules trees too (treated as npm).
        assert_eq!(
            d(
                "/home/u/.local/share/pnpm/global/5/node_modules/.pnpm/@curl-runner+cli-linux-x64@1.0.0/node_modules/@curl-runner/cli-linux-x64/bin/curl-runner"
            ),
            InstallMethod::Npm
        );
        assert_eq!(
            d(
                "/home/u/.config/yarn/global/node_modules/@curl-runner/cli-linux-x64/bin/curl-runner"
            ),
            InstallMethod::Npm
        );
    }

    #[test]
    fn detects_bun_global_path() {
        assert_eq!(
            d(
                "/home/u/.bun/install/global/node_modules/@curl-runner/cli-linux-x64/bin/curl-runner"
            ),
            InstallMethod::Bun
        );
        assert_eq!(d("/home/u/.bun/bin/curl-runner"), InstallMethod::Bun);
        assert_eq!(d("/opt/bun/bin/curl-runner"), InstallMethod::Bun);
        assert_eq!(d("/opt/bun/install/x/curl-runner"), InstallMethod::Bun);
        assert_eq!(
            d(
                r"C:\Users\u\.bun\install\global\node_modules\@curl-runner\cli-win32-x64\bin\curl-runner.exe"
            ),
            InstallMethod::Bun
        );
    }

    #[test]
    fn bun_install_env_prefix() {
        let env = |k: &str| (k == "BUN_INSTALL").then(|| "/custom/b".to_string());
        assert_eq!(
            detect(Path::new("/custom/b/bin/curl-runner"), &env),
            InstallMethod::Bun
        );
        let empty = |k: &str| (k == "BUN_INSTALL").then(String::new);
        assert_eq!(
            detect(Path::new("/usr/bin/curl-runner"), &empty),
            InstallMethod::Standalone
        );
    }

    #[test]
    fn detects_old_npm_heuristics() {
        assert_eq!(d("/usr/lib/node_modules/curl-runner/x"), InstallMethod::Npm);
        assert_eq!(d("/p/node_modules/.bin/curl-runner"), InstallMethod::Npm);
        let env = |k: &str| (k == "npm_config_prefix").then(|| "/usr".to_string());
        assert_eq!(
            detect(Path::new("/somewhere/curl-runner"), &env),
            InstallMethod::Npm
        );
        let env = |k: &str| (k == "npm_execpath").then(|| "/usr/bin/npm".to_string());
        assert_eq!(
            detect(Path::new("/somewhere/curl-runner"), &env),
            InstallMethod::Npm
        );
    }

    #[test]
    fn standalone_paths() {
        assert_eq!(
            d("/home/u/.local/bin/curl-runner"),
            InstallMethod::Standalone
        );
        assert_eq!(d("/usr/local/bin/curl-runner"), InstallMethod::Standalone);
        assert_eq!(
            d(r"C:\Users\u\.local\bin\curl-runner.exe"),
            InstallMethod::Standalone
        );
    }

    #[test]
    fn homebrew_tree_is_unknown() {
        assert_eq!(
            d("/opt/homebrew/Cellar/curl-runner/1.2.3/bin/curl-runner"),
            InstallMethod::Unknown
        );
        assert_eq!(
            d("/home/linuxbrew/.linuxbrew/Cellar/curl-runner/1.2.3/bin/curl-runner"),
            InstallMethod::Unknown
        );
        // npm installed through Homebrew's node is still npm.
        assert_eq!(
            d(
                "/opt/homebrew/Cellar/node/22/lib/node_modules/@curl-runner/cli-darwin-arm64/bin/curl-runner"
            ),
            InstallMethod::Npm
        );
    }

    #[test]
    fn detects_cargo_install_path() {
        assert_eq!(d("/home/u/.cargo/bin/curl-runner"), InstallMethod::Cargo);
        assert_eq!(
            d(r"C:\Users\u\.cargo\bin\curl-runner.exe"),
            InstallMethod::Cargo
        );
        let env = |k: &str| (k == "CARGO_HOME").then(|| "/opt/cargo/".to_string());
        assert_eq!(
            detect(Path::new("/opt/cargo/bin/curl-runner"), &env),
            InstallMethod::Cargo
        );
        // Outside `$CARGO_HOME/bin` it is not a cargo install.
        assert_eq!(
            detect(Path::new("/opt/cargo-tools/curl-runner"), &env),
            InstallMethod::Standalone
        );
    }

    #[test]
    fn dev_builds_and_empty_are_unknown() {
        assert_eq!(
            d("/code/curl-runner/target/release/curl-runner"),
            InstallMethod::Unknown
        );
        assert_eq!(
            d("/code/x/target/debug/curl-runner"),
            InstallMethod::Unknown
        );
        assert_eq!(d(""), InstallMethod::Unknown);
    }

    #[test]
    fn upgrade_commands_match_ts() {
        assert_eq!(
            upgrade_command(InstallMethod::Bun),
            Some("bun install -g @curl-runner/cli@latest")
        );
        assert_eq!(
            upgrade_command(InstallMethod::Npm),
            Some("npm install -g @curl-runner/cli@latest")
        );
        assert_eq!(
            upgrade_command(InstallMethod::Cargo),
            Some("cargo install curl-runner --locked")
        );
        assert_eq!(upgrade_command(InstallMethod::Standalone), None);
        assert_eq!(upgrade_command(InstallMethod::Unknown), None);
    }
}
