//! Auth and TLS rules (TS `validateAuth`, `validateSSLConfig`).

use super::issues::{Ctx, Res, Severity, as_str, truthy_prop};
use crate::model::Json;

const VALID_AUTH_TYPES: &[&str] = &["basic", "bearer", "digest", "ntlm", "oauth2"];

pub fn validate_auth(cx: &mut Ctx, auth: &Json, path: &str) -> Res {
    cx.check_unknown_keys(auth, &["type", "username", "password", "token"], path)?;

    let Some(ty) = truthy_prop(auth, "type") else {
        cx.add(
            &format!("{path}.type"),
            Severity::Error,
            "Auth type is required",
        );
        return Ok(());
    };
    let ty_str = as_str(ty).filter(|t| VALID_AUTH_TYPES.contains(t));
    let Some(ty) = ty_str else {
        return cx.invalid_choice(ty, VALID_AUTH_TYPES, &format!("{path}.type"), "auth type");
    };

    // A hardcoded (non-`${...}`) string credential.
    let hardcoded = |k: &str| {
        truthy_prop(auth, k)
            .and_then(as_str)
            .is_some_and(|s| !s.contains("${"))
    };
    if ty == "basic" {
        if truthy_prop(auth, "username").is_none() {
            cx.add(
                &format!("{path}.username"),
                Severity::Error,
                "Username required for basic auth",
            );
        }
        if truthy_prop(auth, "password").is_none() {
            cx.add(
                &format!("{path}.password"),
                Severity::Warning,
                "Password missing for basic auth",
            );
        }
        if hardcoded("password") {
            cx.add(
                &format!("{path}.password"),
                Severity::Warning,
                "Hardcoded password. Consider using \"${env.PASSWORD}\"",
            );
        }
    }
    if ty == "bearer" {
        if truthy_prop(auth, "token").is_none() {
            cx.add(
                &format!("{path}.token"),
                Severity::Error,
                "Token required for bearer auth",
            );
        }
        if hardcoded("token") {
            cx.add(
                &format!("{path}.token"),
                Severity::Warning,
                "Hardcoded token. Consider using \"${env.TOKEN}\"",
            );
        }
    }
    Ok(())
}

pub fn validate_ssl(cx: &mut Ctx, ssl: &Json, path: &str) -> Res {
    cx.check_unknown_keys(ssl, &["verify", "ca", "cert", "key"], path)?;
    cx.boolean_field(ssl, "verify", path);

    for k in ["ca", "cert", "key"] {
        if truthy_prop(ssl, k).is_some_and(|v| !v.is_string()) {
            cx.add(
                &format!("{path}.{k}"),
                Severity::Error,
                "Must be a string path",
            );
        }
    }
    let cert = truthy_prop(ssl, "cert").is_some();
    let key = truthy_prop(ssl, "key").is_some();
    if cert && !key {
        cx.add(
            path,
            Severity::Warning,
            "cert provided without key; mTLS may fail",
        );
    }
    if key && !cert {
        cx.add(
            path,
            Severity::Warning,
            "key provided without cert; mTLS may fail",
        );
    }
    for k in ["ca", "cert", "key"] {
        if let Some(s) = truthy_prop(ssl, k).and_then(as_str) {
            cx.file_path(s, &format!("{path}.{k}"));
        }
    }
    Ok(())
}
