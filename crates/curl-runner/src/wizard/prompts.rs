//! The wizard's prompts (`wizard/prompts.ts`): same questions, order,
//! defaults, choices and validation messages. Each prompt asks through
//! [`Ui`] and writes the answer into the [`WizardAnswers`], the way the TS
//! flow spreads each result over the answers.

use super::templates::get_template_choices;
use super::types::{AuthType, BodyType, ExpectStatus, WizardAnswers, WizardTemplate};
use super::ui::{Choice, Res, Ui, Validator};
use curl_runner_core::jsval::{str_to_number, stringify};
use curl_runner_core::model::Json;
use indexmap::IndexMap;
use std::rc::Rc;

pub const HTTP_METHODS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

// ---- validators (the TS `validate` callbacks) ----

fn v(f: impl Fn(&str) -> Option<String> + 'static) -> Option<Validator> {
    Some(Rc::new(f))
}

/// `(v) => !v ? message : undefined`.
pub fn required(message: &'static str) -> Option<Validator> {
    v(move |s| s.is_empty().then(|| message.to_string()))
}

/// URL prompt: required, and what `new URL(value)` accepts.
pub fn validate_url(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("URL is required".into());
    }
    match url::Url::parse(value) {
        Ok(_) => None,
        Err(_) => Some("Invalid URL format".into()),
    }
}

/// Timeout / delay: empty is fine, else a number >= 0.
pub fn validate_non_negative(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let n = str_to_number(value);
    (n.is_nan() || n < 0.0).then(|| "Must be a positive number".into())
}

/// Retry attempts: a number >= 1 (an empty answer is `Number('') = 0`).
pub fn validate_attempts(value: &str) -> Option<String> {
    let n = str_to_number(value);
    (n.is_nan() || n < 1.0).then(|| "Must be at least 1".into())
}

/// Backoff multiplier: empty is fine, else a number >= 1.
pub fn validate_backoff(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let n = str_to_number(value);
    (n.is_nan() || n < 1.0).then(|| "Must be at least 1".into())
}

/// Output file: required, `.yaml` or `.yml`.
pub fn validate_yaml_path(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("File path required".into());
    }
    (!value.ends_with(".yaml") && !value.ends_with(".yml"))
        .then(|| "File must end with .yaml or .yml".into())
}

/// quickInit's filename prompt.
pub fn validate_filename(value: &str) -> Option<String> {
    if value.is_empty() {
        return Some("Filename required".into());
    }
    (!value.ends_with(".yaml") && !value.ends_with(".yml"))
        .then(|| "Must end with .yaml or .yml".into())
}

/// Status codes: `200` or `200,201`. TS accepted anything and wrote `NaN`
/// for what was not a number; whole numbers are required here.
pub fn validate_status(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    parse_status(value)
        .is_none()
        .then(|| "Use whole status codes like 200 or 200,201".into())
}

/// `status.split(',').map((s) => Number(s.trim()))`, one code as a number.
pub fn parse_status(value: &str) -> Option<ExpectStatus> {
    let codes: Vec<f64> = value.split(',').map(str_to_number).collect();
    if codes.iter().any(|c| !c.is_finite() || c.fract() != 0.0) {
        return None;
    }
    Some(if codes.len() == 1 {
        ExpectStatus::One(codes[0])
    } else {
        ExpectStatus::Many(codes)
    })
}

fn choices(items: &[(&str, &str, &str)]) -> Vec<Choice> {
    items.iter().map(|(v, l, h)| Choice::new(v, l, h)).collect()
}

fn string_json(s: &str) -> Json {
    Json::String(s.to_string())
}

fn num_text(x: Option<f64>) -> String {
    x.map(curl_runner_core::jsval::num_to_string)
        .unwrap_or_default()
}

// ---- prompts ----

/// `promptTemplate`: `None` is "Blank".
pub fn prompt_template(ui: &mut dyn Ui) -> Res<Option<WizardTemplate>> {
    let mut options = vec![Choice::new("blank", "Blank", "Start from scratch")];
    options.extend(
        get_template_choices()
            .into_iter()
            .map(|c| Choice::new(c.value.key(), c.label, c.hint)),
    );
    let picked = ui.select("Start from a template?", &options, None)?;
    Ok(WizardTemplate::from_key(&picked))
}

/// `promptBasics`: URL, method, name.
pub fn prompt_basics(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    answers.url = ui.text(
        "Request URL",
        "https://api.example.com/endpoint",
        &defaults.url,
        v(validate_url),
    )?;
    let methods: Vec<Choice> = HTTP_METHODS.iter().map(|m| Choice::new(m, m, "")).collect();
    let initial = if defaults.method.is_empty() {
        "GET"
    } else {
        &defaults.method
    };
    answers.method = ui.select("HTTP method", &methods, Some(initial))?;
    let name = ui.text(
        "Request name (optional)",
        "My API Request",
        defaults.name.as_deref().unwrap_or(""),
        None,
    )?;
    answers.name = (!name.is_empty()).then_some(name);
    Ok(())
}

/// `promptHeaders`: the headers to use, `None` for none.
pub fn prompt_headers(
    ui: &mut dyn Ui,
    defaults: Option<&IndexMap<String, Json>>,
) -> Res<Option<IndexMap<String, Json>>> {
    let initial = defaults.is_some_and(|d| !d.is_empty());
    if !ui.confirm("Add headers?", initial)? {
        return Ok(defaults.cloned());
    }
    let mut headers = defaults.cloned().unwrap_or_default();

    let presets = ui.multiselect(
        "Add common headers (space to select)",
        &choices(&[
            ("content-type-json", "Content-Type: application/json", ""),
            ("accept-json", "Accept: application/json", ""),
            ("user-agent", "User-Agent: curl-runner", ""),
        ]),
    )?;
    for preset in presets {
        let (k, val) = match preset.as_str() {
            "content-type-json" => ("Content-Type", "application/json"),
            "accept-json" => ("Accept", "application/json"),
            "user-agent" => ("User-Agent", "curl-runner"),
            _ => continue,
        };
        headers.insert(k.to_string(), string_json(val));
    }

    while ui.confirm("Add custom header?", false)? {
        let key = ui.text(
            "Header name",
            "X-Custom-Header",
            "",
            required("Header name required"),
        )?;
        let val = ui.text(
            "Header value",
            "value",
            "",
            required("Header value required"),
        )?;
        if !key.is_empty() && !val.is_empty() {
            headers.insert(key, string_json(&val));
        }
    }
    Ok((!headers.is_empty()).then_some(headers))
}

/// `promptBody`.
pub fn prompt_body(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    let initial = match defaults.body_type {
        Some(BodyType::Json) => "json",
        Some(BodyType::Form) => "form",
        Some(BodyType::Raw) => "raw",
        Some(BodyType::None) | None => "none",
    };
    let kind = ui.select(
        "Request body type",
        &choices(&[
            ("none", "None", "No request body"),
            ("json", "JSON", "JSON object"),
            ("form", "Form Data", "Multipart form / file upload"),
            ("raw", "Raw", "Plain text or other"),
        ]),
        Some(initial),
    )?;

    match kind.as_str() {
        "json" => prompt_json_body(ui, defaults, answers),
        "form" => prompt_form_body(ui, answers),
        "raw" => {
            let initial = match defaults.body.as_ref() {
                Some(Json::String(s)) => s.clone(),
                Some(b @ (Json::Number(_) | Json::Bool(_))) => {
                    curl_runner_core::jsval::JsVal::from_json(b).to_js_string()
                }
                _ => String::new(),
            };
            let raw = ui.text("Raw body content", "Your content here", &initial, None)?;
            answers.body_type = Some(BodyType::Raw);
            answers.body = Some(Json::String(raw));
            Ok(())
        }
        _ => {
            answers.body_type = Some(BodyType::None);
            Ok(())
        }
    }
}

fn prompt_json_body(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    let initial = match defaults.body.as_ref() {
        Some(b) if super::yaml_builder::js_truthy(b) => stringify(b),
        _ => String::new(),
    };
    let text = ui.text(
        "JSON body (or press enter for interactive)",
        "{\"key\": \"value\"}",
        &initial,
        None,
    )?;
    if !text.is_empty() {
        match serde_json::from_str::<Json>(&text) {
            Ok(parsed) => {
                answers.body_type = Some(BodyType::Json);
                answers.body = Some(parsed);
            }
            Err(_) => {
                ui.warn("Invalid JSON, storing as raw string");
                answers.body_type = Some(BodyType::Raw);
                answers.body = Some(Json::String(text));
            }
        }
        return Ok(());
    }

    // Interactive JSON builder
    let mut body = serde_json::Map::new();
    loop {
        let key = ui.text("Field name", "key", "", None)?;
        let value = ui.text("Field value", "value", "", None)?;
        let kind = ui.select(
            "Value type",
            &choices(&[
                ("string", "String", ""),
                ("number", "Number", ""),
                ("boolean", "Boolean", ""),
            ]),
            None,
        )?;
        if !key.is_empty() {
            body.insert(key, typed_value(ui, &value, &kind));
        }
        if !ui.confirm("Add another field?", false)? {
            break;
        }
    }
    answers.body_type = Some(BodyType::Json);
    answers.body = Some(Json::Object(body));
    Ok(())
}

/// A field value of the chosen type. TS stored `Number('abc')` (`NaN`, which
/// it then wrote as `NaN`); a non-number stays a string here, with a warning.
fn typed_value(ui: &mut dyn Ui, value: &str, kind: &str) -> Json {
    match kind {
        "number" => {
            let n = str_to_number(value);
            if n.is_finite() {
                super::yaml_builder::num(n)
            } else {
                ui.warn(&format!(
                    "{value:?} is not a number, storing it as a string"
                ));
                string_json(value)
            }
        }
        "boolean" => Json::Bool(value == "true"),
        _ => string_json(value),
    }
}

fn prompt_form_body(ui: &mut dyn Ui, answers: &mut WizardAnswers) -> Res<()> {
    let mut form: IndexMap<String, Json> = IndexMap::new();
    loop {
        let kind = ui.select(
            "Field type",
            &choices(&[("text", "Text field", ""), ("file", "File upload", "")]),
            None,
        )?;
        let name = ui.text(
            "Field name",
            if kind == "file" { "file" } else { "field" },
            "",
            required("Field name required"),
        )?;
        if kind == "file" {
            let path = ui.text(
                "File path",
                "./upload.txt",
                "",
                required("File path required"),
            )?;
            form.insert(name, serde_json::json!({ "file": path }));
        } else {
            let value = ui.text("Field value", "value", "", None)?;
            form.insert(name, string_json(&value));
        }
        if !ui.confirm("Add another field?", false)? {
            break;
        }
    }
    answers.body_type = Some(BodyType::Form);
    answers.form_data = Some(form);
    Ok(())
}

/// `promptAuth`.
pub fn prompt_auth(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    let initial = match defaults.auth_type {
        Some(AuthType::Bearer) => "bearer",
        Some(AuthType::Basic) => "basic",
        _ => "none",
    };
    let kind = ui.select(
        "Authentication",
        &choices(&[
            ("none", "None", ""),
            ("bearer", "Bearer Token", "Authorization: Bearer <token>"),
            ("basic", "Basic Auth", "Username/password"),
        ]),
        Some(initial),
    )?;
    match kind.as_str() {
        "bearer" => {
            let token = ui.text(
                "Bearer token",
                "your-token-here",
                defaults.token.as_deref().unwrap_or(""),
                required("Token required"),
            )?;
            answers.auth_type = Some(AuthType::Bearer);
            answers.token = Some(token);
        }
        "basic" => {
            let username = ui.text(
                "Username",
                "",
                defaults.username.as_deref().unwrap_or(""),
                required("Username required"),
            )?;
            let password = ui.password("Password", required("Password required"))?;
            answers.auth_type = Some(AuthType::Basic);
            answers.username = Some(username);
            answers.password = Some(password);
        }
        _ => answers.auth_type = Some(AuthType::None),
    }
    Ok(())
}

/// `promptAdvanced`. TS only ever turned a flag on, so an edited file with
/// `followRedirects: false` (or `insecure`/`http2`) could not be switched
/// back; the answers are taken as given here.
pub fn prompt_advanced(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    if !ui.confirm("Configure advanced options?", false)? {
        return Ok(());
    }
    let timeout = ui.text(
        "Timeout (ms)",
        "30000",
        &num_text(defaults.timeout),
        v(validate_non_negative),
    )?;
    let follow = ui.confirm(
        "Follow redirects?",
        defaults.follow_redirects.unwrap_or(true),
    )?;
    let insecure = ui.confirm(
        "Skip SSL verification? (insecure)",
        defaults.insecure.unwrap_or(false),
    )?;
    let http2 = ui.confirm("Use HTTP/2?", defaults.http2.unwrap_or(false))?;

    if !timeout.is_empty() {
        answers.timeout = Some(str_to_number(&timeout));
    }
    answers.follow_redirects = Some(follow);
    answers.insecure = Some(insecure);
    answers.http2 = Some(http2);
    Ok(())
}

/// `promptRetry`.
pub fn prompt_retry(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    if !ui.confirm(
        "Configure retry strategy?",
        defaults.retry_enabled.unwrap_or(false),
    )? {
        return Ok(());
    }
    let initial_count = defaults
        .retry_count
        .map_or("3".to_string(), |n| num_text(Some(n)));
    let initial_delay = defaults
        .retry_delay
        .map_or("1000".to_string(), |n| num_text(Some(n)));
    let count = ui.text(
        "Max retry attempts",
        "3",
        &initial_count,
        v(validate_attempts),
    )?;
    let delay = ui.text(
        "Retry delay (ms)",
        "1000",
        &initial_delay,
        v(validate_non_negative),
    )?;
    let backoff = ui.text(
        "Backoff multiplier",
        "2 (exponential)",
        &num_text(defaults.retry_backoff),
        v(validate_backoff),
    )?;
    answers.retry_enabled = Some(true);
    answers.retry_count = Some(str_to_number(&count));
    answers.retry_delay = (!delay.is_empty()).then(|| str_to_number(&delay));
    answers.retry_backoff = (!backoff.is_empty()).then(|| str_to_number(&backoff));
    Ok(())
}

/// `promptValidation`.
pub fn prompt_validation(
    ui: &mut dyn Ui,
    defaults: &WizardAnswers,
    answers: &mut WizardAnswers,
) -> Res<()> {
    let configure = defaults
        .expect_status
        .as_ref()
        .is_some_and(|s| !matches!(s, ExpectStatus::One(n) if *n == 0.0))
        || defaults
            .expect_response_time
            .as_deref()
            .is_some_and(|t| !t.is_empty());
    if !ui.confirm("Add response validation?", configure)? {
        return Ok(());
    }
    let initial_status = match &defaults.expect_status {
        Some(ExpectStatus::One(n)) if *n != 0.0 => num_text(Some(*n)),
        Some(ExpectStatus::Many(l)) => l
            .iter()
            .map(|n| num_text(Some(*n)))
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    };
    let status = ui.text(
        "Expected status code(s)",
        "200 or 200,201",
        &initial_status,
        v(validate_status),
    )?;
    let time = ui.text(
        "Response time constraint",
        "< 2000 (ms)",
        defaults.expect_response_time.as_deref().unwrap_or(""),
        None,
    )?;
    if !status.is_empty() {
        answers.expect_status = parse_status(&status);
    }
    if !time.is_empty() {
        answers.expect_response_time = Some(time);
    }
    Ok(())
}

/// `promptOutput`: the file to write and whether to run it afterwards.
pub fn prompt_output(ui: &mut dyn Ui, output_file: Option<&str>) -> Res<(String, bool)> {
    let initial = output_file
        .filter(|f| !f.is_empty())
        .unwrap_or("request.yaml");
    let path = ui.text(
        "Save to file",
        "api-request.yaml",
        initial,
        v(validate_yaml_path),
    )?;
    let run_after = ui.confirm("Run request after creating?", true)?;
    Ok((path, run_after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wizard::types::WizardAnswers;
    use crate::wizard::ui::{Ans, ScriptedUi};
    use serde_json::json;

    #[test]
    fn validators_carry_the_ts_messages() {
        assert_eq!(validate_url("").as_deref(), Some("URL is required"));
        assert_eq!(
            validate_url("not a url").as_deref(),
            Some("Invalid URL format")
        );
        assert_eq!(
            validate_url("example.com").as_deref(),
            Some("Invalid URL format")
        );
        assert_eq!(validate_url("https://api.example.com/x?y=1"), None);
        assert_eq!(validate_non_negative(""), None);
        assert_eq!(validate_non_negative("0"), None);
        assert_eq!(
            validate_non_negative("-1").as_deref(),
            Some("Must be a positive number")
        );
        assert_eq!(
            validate_non_negative("abc").as_deref(),
            Some("Must be a positive number")
        );
        assert_eq!(validate_attempts("").as_deref(), Some("Must be at least 1"));
        assert_eq!(
            validate_attempts("0").as_deref(),
            Some("Must be at least 1")
        );
        assert_eq!(validate_attempts("3"), None);
        assert_eq!(validate_backoff(""), None);
        assert_eq!(
            validate_backoff("0.5").as_deref(),
            Some("Must be at least 1")
        );
        assert_eq!(validate_backoff("2"), None);
        assert_eq!(
            validate_yaml_path("").as_deref(),
            Some("File path required")
        );
        assert_eq!(
            validate_yaml_path("a.json").as_deref(),
            Some("File must end with .yaml or .yml")
        );
        assert_eq!(validate_yaml_path("a.yml"), None);
        assert_eq!(validate_filename("").as_deref(), Some("Filename required"));
        assert_eq!(
            validate_filename("a.txt").as_deref(),
            Some("Must end with .yaml or .yml")
        );
        assert_eq!(
            validate_status("200,abc").as_deref(),
            Some("Use whole status codes like 200 or 200,201")
        );
        assert_eq!(validate_status("200, 201"), None);
        assert_eq!(parse_status("200"), Some(ExpectStatus::One(200.0)));
        assert_eq!(
            parse_status("200, 404"),
            Some(ExpectStatus::Many(vec![200.0, 404.0]))
        );
        let r = required("Token required").unwrap();
        assert_eq!(r("").as_deref(), Some("Token required"));
        assert_eq!(r("x"), None);
    }

    #[test]
    fn template_choices_start_with_blank() {
        let mut ui = ScriptedUi::new(vec![Ans::S("blank"), Ans::S("api-test")]);
        assert_eq!(prompt_template(&mut ui).unwrap(), None);
        assert_eq!(
            prompt_template(&mut ui).unwrap(),
            Some(WizardTemplate::ApiTest)
        );
    }

    #[test]
    fn basics_use_defaults_and_validate_the_url() {
        let defaults = WizardAnswers {
            url: "https://d.test".into(),
            method: "POST".into(),
            name: Some("Old".into()),
            ..Default::default()
        };
        let mut a = WizardAnswers::default();
        let mut ui = ScriptedUi::new(vec![Ans::T(""), Ans::S(""), Ans::T("")]);
        prompt_basics(&mut ui, &defaults, &mut a).unwrap();
        assert_eq!(
            (a.url.as_str(), a.method.as_str(), a.name.as_deref()),
            ("https://d.test", "POST", Some("Old"))
        );

        let mut a = WizardAnswers::default();
        let mut ui = ScriptedUi::new(vec![
            Ans::T("nope"),
            Ans::T("https://x.io"),
            Ans::S("PATCH"),
            Ans::T(""),
        ]);
        prompt_basics(&mut ui, &WizardAnswers::default(), &mut a).unwrap();
        assert_eq!(a.url, "https://x.io");
        assert_eq!(a.method, "PATCH");
        assert_eq!(a.name, None);
        assert!(ui.log.contains(&"invalid:Invalid URL format".to_string()));
    }

    #[test]
    fn headers_no_keeps_defaults_presets_and_custom() {
        let d: IndexMap<String, Json> = IndexMap::from([("A".to_string(), json!("1"))]);
        let mut ui = ScriptedUi::new(vec![Ans::N]);
        assert_eq!(prompt_headers(&mut ui, Some(&d)).unwrap(), Some(d.clone()));
        assert_eq!(
            prompt_headers(&mut ScriptedUi::new(vec![Ans::N]), None).unwrap(),
            None
        );

        let mut ui = ScriptedUi::new(vec![
            Ans::Y,
            Ans::M(&["content-type-json", "user-agent"]),
            Ans::Y,
            Ans::T("X-Custom"),
            Ans::T("v"),
            Ans::N,
        ]);
        let h = prompt_headers(&mut ui, Some(&d)).unwrap().unwrap();
        let keys: Vec<_> = h.keys().map(String::as_str).collect();
        assert_eq!(keys, ["A", "Content-Type", "User-Agent", "X-Custom"]);
        assert_eq!(h["User-Agent"], "curl-runner");
        assert_eq!(ui.leftover(), 0);

        // asked "yes" but added nothing: no headers at all
        let mut ui = ScriptedUi::new(vec![Ans::Y, Ans::M(&[]), Ans::N]);
        assert_eq!(prompt_headers(&mut ui, None).unwrap(), None);
        // initial answer follows the defaults
        let mut ui = ScriptedUi::new(vec![Ans::N]);
        prompt_headers(&mut ui, Some(&d)).unwrap();
        assert_eq!(ui.log[0], "initial:true");
    }

    #[test]
    fn custom_header_needs_name_and_value() {
        let mut ui = ScriptedUi::new(vec![
            Ans::Y,
            Ans::M(&[]),
            Ans::Y,
            Ans::T(""),
            Ans::T("n"),
            Ans::T(""),
            Ans::T("v"),
            Ans::N,
        ]);
        let h = prompt_headers(&mut ui, None).unwrap().unwrap();
        assert_eq!(h["n"], "v");
        assert!(ui.log.contains(&"invalid:Header name required".to_string()));
        assert!(
            ui.log
                .contains(&"invalid:Header value required".to_string())
        );
    }

    fn body(answers: Vec<Ans>, defaults: &WizardAnswers) -> (WizardAnswers, ScriptedUi) {
        let mut ui = ScriptedUi::new(answers);
        let mut a = defaults.clone();
        prompt_body(&mut ui, defaults, &mut a).unwrap();
        assert_eq!(ui.leftover(), 0);
        (a, ui)
    }

    #[test]
    fn body_none_json_text_invalid_json_and_raw() {
        let d = WizardAnswers::default();
        let (a, _) = body(vec![Ans::S("none")], &d);
        assert_eq!(a.body_type, Some(BodyType::None));
        let (a, _) = body(vec![Ans::S("json"), Ans::T("{\"a\": [1, 2]}")], &d);
        assert_eq!(
            (a.body_type, a.body),
            (Some(BodyType::Json), Some(json!({"a": [1, 2]})))
        );
        let (a, ui) = body(vec![Ans::S("json"), Ans::T("{oops")], &d);
        assert_eq!(
            (a.body_type, a.body),
            (Some(BodyType::Raw), Some(json!("{oops")))
        );
        assert!(
            ui.log
                .contains(&"warn:Invalid JSON, storing as raw string".to_string())
        );
        let (a, _) = body(vec![Ans::S("raw"), Ans::T("hello")], &d);
        assert_eq!(
            (a.body_type, a.body),
            (Some(BodyType::Raw), Some(json!("hello")))
        );
    }

    #[test]
    fn body_json_defaults_to_the_existing_body() {
        let d = WizardAnswers {
            body_type: Some(BodyType::Json),
            body: Some(json!({"k": 1})),
            ..Default::default()
        };
        // enter on the select (initial json) and on the text (initial {"k":1})
        let (a, _) = body(vec![Ans::S(""), Ans::T("")], &d);
        assert_eq!(a.body, Some(json!({"k": 1})));
    }

    #[test]
    fn body_interactive_json_builder() {
        let (a, ui) = body(
            vec![
                Ans::S("json"),
                Ans::T(""),
                Ans::T("name"),
                Ans::T("ann"),
                Ans::S("string"),
                Ans::Y,
                Ans::T("age"),
                Ans::T("31"),
                Ans::S("number"),
                Ans::Y,
                Ans::T("ok"),
                Ans::T("true"),
                Ans::S("boolean"),
                Ans::Y,
                Ans::T("n"),
                Ans::T("abc"),
                Ans::S("number"),
                Ans::N,
            ],
            &WizardAnswers::default(),
        );
        assert_eq!(
            a.body,
            Some(json!({"name": "ann", "age": 31, "ok": true, "n": "abc"}))
        );
        assert!(
            ui.log
                .iter()
                .any(|l| l.starts_with("warn:\"abc\" is not a number"))
        );
    }

    #[test]
    fn body_form_fields() {
        let (a, _) = body(
            vec![
                Ans::S("form"),
                Ans::S("text"),
                Ans::T("title"),
                Ans::T("hi"),
                Ans::Y,
                Ans::S("file"),
                Ans::T("avatar"),
                Ans::T("./me.png"),
                Ans::N,
            ],
            &WizardAnswers::default(),
        );
        assert_eq!(a.body_type, Some(BodyType::Form));
        let f = a.form_data.unwrap();
        assert_eq!(f["title"], "hi");
        assert_eq!(f["avatar"], json!({"file": "./me.png"}));
    }

    #[test]
    fn auth_variants() {
        let d = WizardAnswers::default();
        let run = |answers: Vec<Ans>| {
            let mut a = WizardAnswers::default();
            prompt_auth(&mut ScriptedUi::new(answers), &d, &mut a).unwrap();
            a
        };
        assert_eq!(run(vec![Ans::S("none")]).auth_type, Some(AuthType::None));
        let a = run(vec![Ans::S("bearer"), Ans::T(""), Ans::T("tok")]);
        assert_eq!(
            (a.auth_type, a.token.as_deref()),
            (Some(AuthType::Bearer), Some("tok"))
        );
        let a = run(vec![Ans::S("basic"), Ans::T("me"), Ans::T("pw")]);
        assert_eq!(
            (a.auth_type, a.username.as_deref(), a.password.as_deref()),
            (Some(AuthType::Basic), Some("me"), Some("pw"))
        );
    }

    #[test]
    fn auth_defaults_come_from_the_template_or_file() {
        let d = WizardAnswers {
            auth_type: Some(AuthType::Bearer),
            token: Some("old".into()),
            ..Default::default()
        };
        let mut a = d.clone();
        prompt_auth(
            &mut ScriptedUi::new(vec![Ans::S(""), Ans::T("")]),
            &d,
            &mut a,
        )
        .unwrap();
        assert_eq!(a.token.as_deref(), Some("old"));
    }

    #[test]
    fn advanced_skipped_keeps_answers() {
        let d = WizardAnswers {
            timeout: Some(5.0),
            ..Default::default()
        };
        let mut a = d.clone();
        prompt_advanced(&mut ScriptedUi::new(vec![Ans::N]), &d, &mut a).unwrap();
        assert_eq!(a, d);
    }

    #[test]
    fn advanced_sets_values_and_can_switch_flags_back() {
        let d = WizardAnswers {
            follow_redirects: Some(false),
            insecure: Some(true),
            ..Default::default()
        };
        let mut a = d.clone();
        let mut ui = ScriptedUi::new(vec![
            Ans::Y,
            Ans::T("-3"),
            Ans::T("2500"),
            Ans::Y,
            Ans::N,
            Ans::Y,
        ]);
        prompt_advanced(&mut ui, &d, &mut a).unwrap();
        assert_eq!(a.timeout, Some(2500.0));
        assert_eq!(
            (a.follow_redirects, a.insecure, a.http2),
            (Some(true), Some(false), Some(true))
        );
        assert!(
            ui.log
                .contains(&"invalid:Must be a positive number".to_string())
        );
    }

    #[test]
    fn retry_prompts_and_blank_optional_values() {
        let d = WizardAnswers::default();
        let mut a = WizardAnswers::default();
        let mut ui = ScriptedUi::new(vec![
            Ans::Y,
            Ans::T("0"),
            Ans::T("4"),
            Ans::T("250"),
            Ans::T(""),
        ]);
        prompt_retry(&mut ui, &d, &mut a).unwrap();
        assert_eq!(
            (
                a.retry_enabled,
                a.retry_count,
                a.retry_delay,
                a.retry_backoff
            ),
            (Some(true), Some(4.0), Some(250.0), None)
        );
        assert!(ui.log.contains(&"invalid:Must be at least 1".to_string()));
        // defaults are 3 attempts and 1000 ms
        let mut a = WizardAnswers::default();
        prompt_retry(
            &mut ScriptedUi::new(vec![Ans::Y, Ans::T(""), Ans::T(""), Ans::T("2")]),
            &d,
            &mut a,
        )
        .unwrap();
        assert_eq!(
            (a.retry_count, a.retry_delay, a.retry_backoff),
            (Some(3.0), Some(1000.0), Some(2.0))
        );
        // no: untouched
        let mut a = WizardAnswers::default();
        prompt_retry(&mut ScriptedUi::new(vec![Ans::N]), &d, &mut a).unwrap();
        assert_eq!(a.retry_enabled, None);
    }

    #[test]
    fn validation_prompts() {
        let d = WizardAnswers::default();
        let mut a = WizardAnswers::default();
        let mut ui = ScriptedUi::new(vec![Ans::Y, Ans::T("200, 201"), Ans::T("< 2000")]);
        prompt_validation(&mut ui, &d, &mut a).unwrap();
        assert_eq!(
            a.expect_status,
            Some(ExpectStatus::Many(vec![200.0, 201.0]))
        );
        assert_eq!(a.expect_response_time.as_deref(), Some("< 2000"));

        let mut a = WizardAnswers::default();
        let mut ui = ScriptedUi::new(vec![Ans::Y, Ans::T("2xx"), Ans::T("200"), Ans::T("")]);
        prompt_validation(&mut ui, &d, &mut a).unwrap();
        assert_eq!(a.expect_status, Some(ExpectStatus::One(200.0)));
        assert!(
            ui.log
                .contains(&"invalid:Use whole status codes like 200 or 200,201".to_string())
        );

        // initial answer follows the template's checks
        let d = WizardAnswers {
            expect_status: Some(ExpectStatus::One(200.0)),
            ..Default::default()
        };
        let mut ui = ScriptedUi::new(vec![Ans::N]);
        prompt_validation(&mut ui, &d, &mut d.clone()).unwrap();
        assert_eq!(ui.log[0], "initial:true");
    }

    #[test]
    fn output_prompt_validates_and_defaults() {
        let mut ui = ScriptedUi::new(vec![Ans::T(""), Ans::Y]);
        assert_eq!(
            prompt_output(&mut ui, None).unwrap(),
            ("request.yaml".to_string(), true)
        );
        let mut ui = ScriptedUi::new(vec![Ans::T("bad.txt"), Ans::T("ok.yml"), Ans::N]);
        assert_eq!(
            prompt_output(&mut ui, Some("x.yaml")).unwrap(),
            ("ok.yml".to_string(), false)
        );
        assert!(
            ui.log
                .contains(&"invalid:File must end with .yaml or .yml".to_string())
        );
        let mut ui = ScriptedUi::new(vec![Ans::T(""), Ans::N]);
        assert_eq!(
            prompt_output(&mut ui, Some("keep.yaml")).unwrap().0,
            "keep.yaml"
        );
    }

    #[test]
    fn cancel_stops_a_prompt() {
        let mut ui = ScriptedUi::new(vec![Ans::Cancel]);
        assert!(prompt_template(&mut ui).is_err());
    }
}
