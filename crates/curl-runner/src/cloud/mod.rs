//! curl-runner Cloud commands: `login`, `logout` (and, in later tasks,
//! `publish` and `pull`). `client` talks to the API, `credentials` keeps the
//! saved sessions.

pub mod client;
pub mod credentials;
mod login;
mod publish;
mod pull;

use crate::output::Console;

/// Environment lookup, injectable for tests.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Dispatch a cloud subcommand; `argv[0]` is its name.
pub fn run(argv: &[String], env: Env, console: &mut Console) -> i32 {
    let rest = &argv[1..];
    match argv[0].as_str() {
        "login" => login::login(rest, env, console),
        "logout" => login::logout(rest, env, console),
        "publish" => publish::publish(rest, env, console),
        "pull" => pull::pull(rest, env, console),
        other => {
            console.err(&format!("curl-runner {other} is not available yet."));
            1
        }
    }
}

/// Value of `--flag value` or `--flag=value`. A missing value, or one that
/// looks like another flag, is an error.
pub fn flag_value(args: &[String], flag: &str) -> Result<Option<String>, String> {
    let prefix = format!("{flag}=");
    for (i, a) in args.iter().enumerate() {
        let value = if a == flag {
            args.get(i + 1).map(String::as_str)
        } else if let Some(v) = a.strip_prefix(&prefix) {
            Some(v)
        } else {
            continue;
        };
        return match value {
            Some(v) if !v.is_empty() && !v.starts_with("--") => Ok(Some(v.to_string())),
            _ => Err(format!("{flag} needs a value.")),
        };
    }
    Ok(None)
}

/// The API base URL from `--server`, the environment or the default; plain
/// http is refused unless it is loopback.
pub fn server_from_args(args: &[String], env: Env) -> Result<String, String> {
    let flag = flag_value(args, "--server")?;
    let server = client::resolve_server(flag.as_deref(), env);
    client::check_server(&server)?;
    Ok(server)
}
