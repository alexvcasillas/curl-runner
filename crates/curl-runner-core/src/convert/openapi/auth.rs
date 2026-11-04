//! Security requirements as curl-runner auth, headers, params and cookies.
//! Every credential is a secret variable named from the API prefix and the
//! scheme's key.

use serde_json::{Map, Value};

use super::map::Mapper;
use crate::convert::import::{Attention, Field, FieldValue, MappedAuth, MappedRequest};

impl Mapper<'_> {
    /// Apply `op`'s security, else the document's. The first requirement
    /// is used, with every scheme it lists; `security: []` means none.
    pub(super) fn security(
        &mut self,
        op: &Value,
        r: &mut MappedRequest,
        cookies: &mut Vec<String>,
    ) {
        let doc = self.doc;
        let Some(alts) = op
            .get("security")
            .or_else(|| doc.root.get("security"))
            .and_then(Value::as_array)
        else {
            return;
        };
        let Some(first) = alts.first().and_then(Value::as_object) else {
            return;
        };
        if alts.len() > 1 {
            let used = names(first);
            let others = alts[1..]
                .iter()
                .filter_map(Value::as_object)
                .map(names)
                .collect::<Vec<_>>()
                .join(", ");
            self.tally.hit(
                format!("choice\0{used}\0{others}"),
                self.req,
                &self.at,
                true,
                || Attention::AuthChoice {
                    used: used.clone(),
                    others: others.clone(),
                    count: 1,
                },
            );
        }
        for key in first.keys() {
            let Some(raw) = doc
                .root
                .pointer("/components/securitySchemes")
                .and_then(|s| s.get(key))
            else {
                self.unresolved(
                    format!(
                        "#/components/securitySchemes/{}",
                        key.replace('~', "~0").replace('/', "~1")
                    ),
                    false,
                );
                continue;
            };
            let Some(scheme) = self.resolve(raw) else {
                continue;
            };
            self.scheme(key, scheme, r, cookies);
        }
    }

    fn scheme(&mut self, key: &str, s: &Value, r: &mut MappedRequest, cookies: &mut Vec<String>) {
        let base = format!("{}_{}", self.prefix, key.to_ascii_uppercase());
        let var = |suffix: &str| format!("{base}{suffix}");
        let kind = s.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "apiKey" => {
                let name = s.get("name").and_then(Value::as_str).unwrap_or("");
                if name.is_empty() {
                    return self.unsupported(key, "apiKey");
                }
                let v = self.secret_var(&var(""));
                let value = format!("${{{v}}}");
                match s.get("in").and_then(Value::as_str) {
                    Some("query") => r.params.push(field(name, value)),
                    Some("cookie") => cookies.push(format!("{name}={value}")),
                    _ => r.headers.push(field(name, value)),
                }
            }
            "http" => {
                let http = s
                    .get("scheme")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let auth = match http.as_str() {
                    "bearer" => {
                        let token = self.secret_var(&var("_TOKEN"));
                        MappedAuth::Bearer(format!("${{{token}}}"))
                    }
                    "basic" => {
                        let user = self.add_var(&var("_USERNAME"), false);
                        let pass = self.secret_var(&var("_PASSWORD"));
                        MappedAuth::Basic {
                            username: format!("${{{user}}}"),
                            password: format!("${{{pass}}}"),
                        }
                    }
                    other => return self.unsupported(key, other),
                };
                self.set_auth(key, &http, auth, r);
            }
            "oauth2" | "openIdConnect" => {
                let token = self.secret_var(&var(""));
                self.tally.push(Attention::AccessToken {
                    var: token.clone(),
                    kind: if kind == "oauth2" {
                        "OAuth 2"
                    } else {
                        "OpenID Connect"
                    }
                    .into(),
                });
                if !self.tokens.contains(&token) {
                    self.tokens.push(token.clone());
                }
                self.set_auth(key, kind, MappedAuth::Bearer(format!("${{{token}}}")), r);
            }
            other => self.unsupported(key, other),
        }
    }

    /// `auth` as the request's auth, or reported when another scheme of
    /// the same requirement already took that place.
    fn set_auth(&mut self, key: &str, kind: &str, auth: MappedAuth, r: &mut MappedRequest) {
        if r.auth.is_some() {
            self.unsupported(key, kind);
        } else {
            r.auth = Some(auth);
        }
    }

    fn unsupported(&mut self, scheme: &str, kind: &str) {
        let kind = if kind.is_empty() { "unknown" } else { kind };
        self.tally.hit(
            format!("scheme\0{scheme}"),
            self.req,
            &self.at,
            true,
            || Attention::UnsupportedScheme {
                scheme: scheme.to_string(),
                kind: kind.to_string(),
                count: 1,
            },
        );
    }
}

/// A requirement's schemes, joined with ` + `; `no` for an empty one.
fn names(req: &Map<String, Value>) -> String {
    if req.is_empty() {
        "no".into()
    } else {
        req.keys().cloned().collect::<Vec<_>>().join(" + ")
    }
}

fn field(key: &str, value: String) -> Field {
    Field {
        key: key.into(),
        value: FieldValue::Text(value),
        disabled: false,
        note: None,
    }
}
