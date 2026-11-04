//! curl-runner Cloud commands: `login`, `logout` (and, in later tasks,
//! `publish` and `pull`). `client` talks to the API, `credentials` keeps the
//! saved sessions.

pub mod client;
pub mod credentials;
mod login;

use crate::output::Console;

/// Environment lookup, injectable for tests.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Dispatch a cloud subcommand; `argv[0]` is its name.
pub fn run(argv: &[String], env: Env, console: &mut Console) -> i32 {
    let rest = &argv[1..];
    match argv[0].as_str() {
        "login" => login::login(rest, env, console),
        "logout" => login::logout(rest, env, console),
        other => {
            console.err(&format!("curl-runner {other} is not available yet."));
            1
        }
    }
}

/// Value of `--flag <value>` in `args`.
pub fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == flag)?;
    args.get(i + 1).map(String::as_str)
}
