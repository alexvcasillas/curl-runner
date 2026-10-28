//! Wizard templates for common use cases (`wizard/templates.ts`).

use super::types::{
    AuthType, BodyType, ExpectStatus, TemplateConfig, WizardAnswers, WizardTemplate,
};
use curl_runner_core::model::Json;
use indexmap::IndexMap;

fn json_content_type() -> Option<IndexMap<String, Json>> {
    Some(IndexMap::from([(
        "Content-Type".to_string(),
        Json::String("application/json".into()),
    )]))
}

pub fn get_template(t: WizardTemplate) -> TemplateConfig {
    match t {
        WizardTemplate::BasicGet => TemplateConfig {
            name: "Basic GET Request",
            description: "Simple GET request to fetch data",
            defaults: WizardAnswers {
                method: "GET".into(),
                follow_redirects: Some(true),
                ..Default::default()
            },
        },
        WizardTemplate::BasicPost => TemplateConfig {
            name: "Basic POST Request",
            description: "POST request with JSON body",
            defaults: WizardAnswers {
                method: "POST".into(),
                body_type: Some(BodyType::Json),
                headers: json_content_type(),
                ..Default::default()
            },
        },
        WizardTemplate::ApiTest => TemplateConfig {
            name: "API Test",
            description: "Request with validation and response time check",
            defaults: WizardAnswers {
                method: "GET".into(),
                expect_status: Some(ExpectStatus::One(200.0)),
                expect_response_time: Some("< 2000".into()),
                verbose: Some(true),
                show_headers: Some(true),
                ..Default::default()
            },
        },
        WizardTemplate::FileUpload => TemplateConfig {
            name: "File Upload",
            description: "Multipart form upload request",
            defaults: WizardAnswers {
                method: "POST".into(),
                body_type: Some(BodyType::Form),
                ..Default::default()
            },
        },
        WizardTemplate::AuthFlow => TemplateConfig {
            name: "Auth Flow",
            description: "Request with authentication",
            defaults: WizardAnswers {
                method: "POST".into(),
                auth_type: Some(AuthType::Bearer),
                headers: json_content_type(),
                ..Default::default()
            },
        },
    }
}

/// A template choice for the picker.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateChoice {
    pub value: WizardTemplate,
    pub label: &'static str,
    pub hint: &'static str,
}

pub fn get_template_choices() -> Vec<TemplateChoice> {
    WizardTemplate::ALL
        .into_iter()
        .map(|value| {
            let c = get_template(value);
            TemplateChoice {
                value,
                label: c.name,
                hint: c.description,
            }
        })
        .collect()
}

// templates.test.ts
#[cfg(test)]
mod tests {
    use super::*;

    fn t(key: &str) -> TemplateConfig {
        get_template(WizardTemplate::from_key(key).unwrap())
    }

    #[test]
    fn has_basic_get_template() {
        assert_eq!(t("basic-get").name, "Basic GET Request");
        assert_eq!(t("basic-get").defaults.method, "GET");
    }

    #[test]
    fn has_basic_post_template() {
        let d = t("basic-post");
        assert_eq!(d.name, "Basic POST Request");
        assert_eq!(d.defaults.method, "POST");
        assert_eq!(d.defaults.body_type, Some(BodyType::Json));
    }

    #[test]
    fn has_api_test_template() {
        let d = t("api-test").defaults;
        assert_eq!(d.expect_status, Some(ExpectStatus::One(200.0)));
        assert_eq!(d.verbose, Some(true));
        assert_eq!(d.show_headers, Some(true));
        assert_eq!(d.expect_response_time.as_deref(), Some("< 2000"));
    }

    #[test]
    fn has_file_upload_template() {
        let d = t("file-upload").defaults;
        assert_eq!(d.method, "POST");
        assert_eq!(d.body_type, Some(BodyType::Form));
    }

    #[test]
    fn has_auth_flow_template() {
        assert_eq!(t("auth-flow").defaults.auth_type, Some(AuthType::Bearer));
    }

    #[test]
    fn get_template_returns_by_name() {
        assert_eq!(
            get_template(WizardTemplate::BasicGet).name,
            "Basic GET Request"
        );
    }

    #[test]
    fn basic_post_default_headers() {
        let h = t("basic-post").defaults.headers.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h["Content-Type"], "application/json");
    }

    #[test]
    fn choices_are_five_in_ts_order_with_label_and_hint() {
        let c = get_template_choices();
        assert_eq!(c.len(), 5);
        let keys: Vec<_> = c.iter().map(|c| c.value.key()).collect();
        assert_eq!(
            keys,
            [
                "basic-get",
                "basic-post",
                "api-test",
                "file-upload",
                "auth-flow"
            ]
        );
        for choice in &c {
            assert!(!choice.label.is_empty() && !choice.hint.is_empty());
        }
        let g = c.iter().find(|c| c.value.key() == "basic-get").unwrap();
        assert_eq!(g.label, "Basic GET Request");
        assert_eq!(g.hint, "Simple GET request to fetch data");
    }

    #[test]
    fn unknown_key_is_none() {
        assert_eq!(WizardTemplate::from_key("nope"), None);
    }
}
