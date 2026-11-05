//! `curl-runner login [--server URL] [--email ADDR | --browser]` and
//! `curl-runner logout [--server URL]`.

use super::client::{self, Client, CloudError};
use super::credentials::{self, Credentials, Entry};
use super::{Env, escape_control, flag_value, server_from_args};
use crate::output::Console;
use std::io::{BufRead, IsTerminal};

const NO_TTY: &str = "Run curl-runner login in a terminal, or set CURL_RUNNER_TOKEN.";

/// "CLI on {host}" for the session list; no dependency, so only env vars.
fn label(env: Env) -> String {
    let host = ["HOSTNAME", "COMPUTERNAME"]
        .iter()
        .find_map(|k| env(k).filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "this computer".into());
    format!("CLI on {}", host.trim())
}

fn prompt(message: &str, tty: bool) -> Option<String> {
    if tty {
        return cliclack::input(message).interact::<String>().ok();
    }
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok()?;
    Some(line)
}

pub(super) fn login(args: &[String], env: Env, console: &mut Console) -> i32 {
    let (server, email_flag) = match (flag_value(args, "--email"), server_from_args(args, env)) {
        (Ok(e), Ok(s)) => (s, e),
        (Err(m), _) | (_, Err(m)) => {
            console.err(&m);
            return 1;
        }
    };
    let tty = std::io::stdin().is_terminal();
    if args.iter().any(|a| a == "--browser") {
        return login_browser(&server, tty, env, console);
    }
    let email = match email_flag {
        Some(e) => e.trim().to_string(),
        None if tty => match prompt("Email address", true) {
            Some(e) => e.trim().to_string(),
            None => return 1,
        },
        None => {
            console.err(NO_TTY);
            return 1;
        }
    };
    if email.is_empty() {
        console.err("An email address is required.");
        return 1;
    }
    let Some(path) = credentials::path(env) else {
        console.err("Cannot find a place to save your session.");
        return 1;
    };
    let api = Client::new(&server, None);
    if let Err(e) = api.email_start(&email, &label(env)) {
        console.err(&client::describe(&e, &server));
        return 1;
    }
    console.out(&format!("We sent a 6-digit code to {email}."));
    let code = match prompt("Code", tty) {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => {
            console.err("A code is required.");
            return 1;
        }
    };
    let verified = match api.email_verify(&email, &code) {
        Ok(v) => v,
        Err(e) => {
            console.err(&client::describe(&e, &server));
            return 1;
        }
    };
    save_session(&server, &path, verified, console)
}

fn save_session(
    server: &str,
    path: &std::path::Path,
    verified: client::Verified,
    console: &mut Console,
) -> i32 {
    let saved = Credentials::load(path).and_then(|mut c| {
        c.servers.insert(
            server.to_string(),
            Entry {
                session: verified.session,
                email: verified.user.email.clone(),
            },
        );
        c.save(path)
    });
    if let Err(e) = saved {
        console.err(&e);
        return 1;
    }
    console.out(&format!(
        "Signed in to {server} as {}.",
        escape_control(&verified.user.email)
    ));
    0
}

const MAX_CODE_TRIES: usize = 3;

/// Best effort: only on macOS and only from a terminal. Printing the URL is
/// what the sign-in relies on, so every failure here is ignored.
fn try_open(url: &str, tty: bool) {
    if cfg!(target_os = "macos") && tty {
        let _ = std::process::Command::new("open")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// `login --browser`: sign in on the dashboard, then paste the hand-off code.
fn login_browser(server: &str, tty: bool, env: Env, console: &mut Console) -> i32 {
    let Some(path) = credentials::path(env) else {
        console.err("Cannot find a place to save your session.");
        return 1;
    };
    let api = Client::new(server, None);
    let info = match api.info() {
        Ok(i) => i,
        Err(e) => {
            console.err(&client::describe(&e, server));
            return 1;
        }
    };
    let Some(app_url) = info.app_url.filter(|u| !u.trim().is_empty()) else {
        console.err(
            "This server doesn't support browser sign-in. Run curl-runner login without --browser.",
        );
        return 1;
    };
    let url = format!(
        "{}/sign-in?client=device",
        escape_control(app_url.trim_end_matches('/'))
    );
    console.out(&format!("Open this page to sign in:\n  {url}"));
    try_open(&url, tty);
    for attempt in 1..=MAX_CODE_TRIES {
        let code = match prompt("Paste the code from your browser:", tty) {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => {
                console.err("A code is required.");
                return 1;
            }
        };
        match api.handoff_redeem(&code) {
            Ok(verified) => return save_session(server, &path, verified, console),
            Err(CloudError::Api(b, _))
                if matches!(
                    b.error,
                    curl_runner_cloud_proto::ErrorCode::CodeInvalid
                        | curl_runner_cloud_proto::ErrorCode::CodeExpired
                ) =>
            {
                console.err(&escape_control(&b.message));
                if attempt == MAX_CODE_TRIES {
                    return 1;
                }
            }
            Err(e) => {
                console.err(&client::describe(&e, server));
                return 1;
            }
        }
    }
    1
}

pub fn logout(args: &[String], env: Env, console: &mut Console) -> i32 {
    let server = match server_from_args(args, env) {
        Ok(s) => s,
        Err(m) => {
            console.err(&m);
            return 1;
        }
    };
    let Some(path) = credentials::path(env) else {
        console.out(&format!("Not signed in to {server}."));
        return 0;
    };
    let mut creds = match Credentials::load(&path) {
        Ok(c) => c,
        Err(e) => {
            console.err(&e);
            return 1;
        }
    };
    let Some(entry) = creds.servers.remove(&server) else {
        console.out(&format!("Not signed in to {server}."));
        return 0;
    };
    let mut reached = true;
    match Client::new(&server, Some(entry.session)).sign_out() {
        Ok(()) | Err(CloudError::Api(..)) => {}
        Err(CloudError::Unreachable(_)) => reached = false,
    }
    if let Err(e) = creds.save(&path) {
        console.err(&e);
        return 1;
    }
    if reached {
        console.out(&format!("Signed out of {server}."));
    } else {
        console.out(&format!(
            "Signed out of {server} on this computer. The server could not be reached, so the session may still be active there."
        ));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_uses_env_host_or_fallback() {
        assert_eq!(
            label(&|k| (k == "HOSTNAME").then(|| "laptop\n".to_string())),
            "CLI on laptop"
        );
        assert_eq!(
            label(&|k| (k == "COMPUTERNAME").then(|| "WIN1".to_string())),
            "CLI on WIN1"
        );
        assert_eq!(label(&|_| None), "CLI on this computer");
    }
}
