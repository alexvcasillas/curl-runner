//! Starter text for new request files and the file names they get.

use curl_runner_core::convert::{YamlOptions, normalize, parse_curl, serialize_to_yaml};
use curl_runner_core::yaml_text::yaml_str;
use unicode_normalization::UnicodeNormalization as _;
use unicode_normalization::char::is_combining_mark;

/// A single-request file with `name`, `url` and `method`.
pub fn request_template(name: &str, method: &str, url: &str) -> String {
    format!(
        "request:\n  name: {}\n  url: {}\n  method: {}\n",
        yaml_str(name),
        yaml_str(url),
        yaml_str(&method.to_ascii_uppercase()),
    )
}

/// The file name for a request called `name`: a kebab-case ASCII slug plus
/// `.yaml` (`Create charge` → `create-charge.yaml`). Latin diacritics are
/// dropped (`Ü` → `u`); an empty slug becomes `request.yaml`.
pub fn file_name_for(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for c in name.nfd().filter(|&c| !is_combining_mark(c)) {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    format!("{}.yaml", if slug.is_empty() { "request" } else { slug })
}

/// A curl command converted to a request file.
#[derive(Debug, Clone, PartialEq)]
pub struct FromCurl {
    pub yaml: String,
    /// What the conversion could not carry over (unsupported flags).
    pub warnings: Vec<String>,
}

/// Converts `curl` with the CLI's `convert curl` pipeline and names the
/// request `name`. Warnings also stay in the file as `# Warning:` comments,
/// as `convert` writes them.
pub fn request_from_curl(name: &str, curl: &str) -> Result<FromCurl, String> {
    let curl = curl.trim();
    if curl.split_whitespace().next() != Some("curl") {
        return Err("Paste a command that starts with curl.".into());
    }
    let mut ir = normalize(&parse_curl(curl)).map_err(|e| e.to_string())?;
    if ir.url.is_empty() {
        return Err("The command has no URL.".into());
    }
    ir.name = Some(name.to_string()).filter(|n| !n.is_empty());
    Ok(FromCurl {
        yaml: serialize_to_yaml(&ir, YamlOptions::default()),
        warnings: ir.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::collection::Collection;
    use crate::model::diagnostics::diagnose;
    use std::path::Path;

    #[test]
    fn template_lints_clean() {
        let text = request_template("First request", "get", "https://api.example.com");
        assert_eq!(
            text,
            "request:\n  name: First request\n  url: https://api.example.com\n  method: GET\n"
        );
        assert_eq!(diagnose(&text), []);
        // Names that need quoting still read back and lint clean.
        let text = request_template("a: b # c", "POST", "https://api.example.com/x?y=1");
        assert_eq!(diagnose(&text), []);
        assert!(text.contains("name: \"a: b # c\""), "{text}");
    }

    #[test]
    fn file_name_slugs() {
        assert_eq!(file_name_for("Create charge"), "create-charge.yaml");
        assert_eq!(file_name_for("  Ünïcode / x "), "unicode-x.yaml");
        assert_eq!(
            file_name_for("GET  /v1/charges/:id"),
            "get-v1-charges-id.yaml"
        );
        assert_eq!(file_name_for(" / "), "request.yaml");
        assert_eq!(file_name_for(""), "request.yaml");
    }

    #[test]
    fn from_curl_creates_lintable_file() {
        let curl = "curl -X POST https://api.example.com/v1/charges \
                    -H 'Content-Type: application/json' -d '{\"amount\":2500}'";
        let converted = request_from_curl("Create charge", curl).unwrap();
        assert!(converted.warnings.is_empty(), "{:?}", converted.warnings);
        assert!(
            converted.yaml.contains("name: Create charge"),
            "{}",
            converted.yaml
        );
        assert!(
            converted.yaml.contains("method: POST"),
            "{}",
            converted.yaml
        );

        let dir = tempfile::tempdir().unwrap();
        let mut c = Collection::open(dir.path()).unwrap();
        let rel = Path::new("charges").join(file_name_for("Create charge"));
        c.create_file(&rel, &converted.yaml).unwrap();
        let written = std::fs::read_to_string(dir.path().join(&rel)).unwrap();
        assert_eq!(diagnose(&written), []);
    }

    #[test]
    fn from_curl_reports_unsupported_flags() {
        let converted =
            request_from_curl("x", "curl --compressed https://api.example.com").unwrap();
        assert!(!converted.warnings.is_empty());
        assert_eq!(diagnose(&converted.yaml), []);
    }

    #[test]
    fn from_curl_rejects_text_without_a_url() {
        assert_eq!(
            request_from_curl("x", "  curl  "),
            Err("The command has no URL.".into())
        );
        assert_eq!(
            request_from_curl("x", "hello https://api.example.com"),
            Err("Paste a command that starts with curl.".into())
        );
    }
}
