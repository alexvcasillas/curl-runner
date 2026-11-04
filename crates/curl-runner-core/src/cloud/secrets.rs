//! A narrow check: a secret input must not be given a literal value in a
//! variables block of the bundle.

use curl_runner_cloud_proto::Input;

use super::scan::Bundle;
use crate::model::Json;
use crate::parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretLeak {
    pub name: String,
    pub file: String,
}

fn is_literal(v: &Json) -> bool {
    match v {
        Json::String(s) => {
            let s = s.trim();
            !s.is_empty() && !(s.starts_with("${") && s.ends_with('}'))
        }
        Json::Number(_) | Json::Bool(_) => true,
        _ => false,
    }
}

/// Files that assign a literal to a secret input's name under
/// `global.variables`, `collection.variables` or the top-level `variables`.
pub fn find_secret_literals(bundle: &Bundle, inputs: &[Input]) -> Vec<SecretLeak> {
    let secrets: Vec<&str> = inputs
        .iter()
        .filter(|i| i.secret)
        .map(|i| i.name.as_str())
        .collect();
    let mut leaks = Vec::new();
    if secrets.is_empty() {
        return leaks;
    }
    for file in &bundle.files {
        let Ok(doc) = parser::parse_raw(&file.text) else {
            continue;
        };
        let blocks = [
            doc.get("global").and_then(|g| g.get("variables")),
            doc.get("collection").and_then(|c| c.get("variables")),
            doc.get("variables"),
        ];
        for name in &secrets {
            if blocks
                .iter()
                .flatten()
                .any(|b| b.get(*name).is_some_and(is_literal))
            {
                leaks.push(SecretLeak {
                    name: (*name).to_string(),
                    file: file.path.clone(),
                });
            }
        }
    }
    leaks
}
