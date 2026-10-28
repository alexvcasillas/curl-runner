//! Wizard types (`wizard/types.ts`).

use curl_runner_core::model::Json;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyType {
    None,
    Json,
    Form,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    None,
    Basic,
    Bearer,
}

/// `expect.status`: one code or a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExpectStatus {
    One(f64),
    Many(Vec<f64>),
}

/// `WizardAnswers`, also used as `Partial<WizardAnswers>` (template defaults):
/// every field is optional, an unset `url`/`method` is the empty string.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WizardAnswers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, Json>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_type: Option<BodyType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_data: Option<IndexMap<String, Json>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<AuthType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<IndexMap<String, Json>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_redirects: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http2: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_delay: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_backoff: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_status: Option<ExpectStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_response_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_headers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_body: Option<bool>,
}

/// `WizardTemplate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardTemplate {
    BasicGet,
    BasicPost,
    ApiTest,
    FileUpload,
    AuthFlow,
}

impl WizardTemplate {
    /// The TS keys, in `TEMPLATES` order.
    pub const ALL: [WizardTemplate; 5] = [
        WizardTemplate::BasicGet,
        WizardTemplate::BasicPost,
        WizardTemplate::ApiTest,
        WizardTemplate::FileUpload,
        WizardTemplate::AuthFlow,
    ];

    pub fn key(self) -> &'static str {
        match self {
            WizardTemplate::BasicGet => "basic-get",
            WizardTemplate::BasicPost => "basic-post",
            WizardTemplate::ApiTest => "api-test",
            WizardTemplate::FileUpload => "file-upload",
            WizardTemplate::AuthFlow => "auth-flow",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.key() == key)
    }
}

/// `TemplateConfig`.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateConfig {
    pub name: &'static str,
    pub description: &'static str,
    pub defaults: WizardAnswers,
}

/// What the wizard needs to run (`WizardOptions`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WizardOptions {
    /// File to edit (edit mode).
    pub edit_file: Option<String>,
    /// Output file path.
    pub output_path: Option<String>,
}
