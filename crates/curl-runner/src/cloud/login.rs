//! `curl-runner login [--server URL] [--email ADDR]` and
//! `curl-runner logout [--server URL]`.

use super::client::{self, Client, CloudError};
use super::credentials::{self, Credentials, Entry};
use super::{Env, flag_value};
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

pub fn login(args: &[String], env: Env, console: &mut Console) -> i32 {
    let server = client::resolve_server(flag_value(args, "--server"), env);
    let tty = std::io::stdin().is_terminal();
    let email = match flag_value(args, "--email") {
        Some(e) => e.trim().to_string(),
        None if tty => match prompt("Email address", true) {
            Some(e) => e.trim().to_string(),
            None => return 1,
        },
        None => {
            console.err(NO_TTY);
            return 2;
        }
    };
    if email.is_empty() {
        console.err("An email address is required.");
        return 2;
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
    let saved = Credentials::load(&path).and_then(|mut c| {
        c.servers.insert(
            server.clone(),
            Entry {
                session: verified.session,
                email: verified.user.email.clone(),
            },
        );
        c.save(&path)
    });
    if let Err(e) = saved {
        console.err(&e);
        return 1;
    }
    console.out(&format!(
        "Signed in to {server} as {}.",
        verified.user.email
    ));
    0
}

pub fn logout(args: &[String], env: Env, console: &mut Console) -> i32 {
    let server = client::resolve_server(flag_value(args, "--server"), env);
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
