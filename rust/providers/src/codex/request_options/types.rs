use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequestOptionsError {
    #[error("Invalid Codex option {option}: {reason}; correct the request configuration")]
    Invalid { option: String, reason: String },
    #[error("Unsupported Codex option {option}={value}; supported values: {supported:?}. Refresh model capabilities or choose a supported value")]
    Unsupported {
        option: String,
        value: Value,
        supported: Vec<Value>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Effort {
    Named(String),
    Numeric(u64),
}

impl Effort {
    pub fn wire(&self) -> Value {
        match self {
            Self::Named(value) => value
                .parse::<u64>()
                .map_or_else(|_| json!(value), |value| json!(value)),
            Self::Numeric(value) => json!(value),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReasoningOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryDelivery {
    SequentialCutoff,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StreamOptions {
    pub reasoning_summary_delivery: SummaryDelivery,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CodexRequestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(flatten)]
    pub additional: Map<String, Value>,
}

impl CodexRequestOptions {
    pub fn parse(value: &Value) -> Result<Self, RequestOptionsError> {
        let options: Self = serde_json::from_value(value.clone())
            .map_err(|error| invalid("configuration", error))?;
        for key in options.additional.keys() {
            if ![
                "background",
                "top_p",
                "truncation",
                "user",
                "metadata",
                "parallel_tool_calls",
                "previous_response_id",
            ]
            .contains(&key.as_str())
            {
                return Err(invalid(key, "unknown option"));
            }
        }
        serde_json::from_value::<rig::providers::openai::responses_api::AdditionalParameters>(
            Value::Object(options.additional.clone()),
        )
        .map_err(|error| invalid("configuration", error))?;
        if let Some(format) = options.text.as_ref().and_then(|text| text.format.as_ref()) {
            serde_json::from_value::<rig::providers::openai::responses_api::TextFormat>(
                format.clone(),
            )
            .map_err(|error| invalid("text.format", error))?;
        }
        Ok(options)
    }
}

fn invalid(option: &str, reason: impl std::fmt::Display) -> RequestOptionsError {
    RequestOptionsError::Invalid {
        option: option.into(),
        reason: reason.to_string(),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CodexCapabilities {
    pub stale: bool,
    pub efforts: Vec<Effort>,
    pub contexts: Vec<String>,
    pub summaries: Vec<String>,
    pub verbosities: Vec<String>,
    pub service_tiers: Vec<String>,
    pub sequential_cutoff: bool,
    pub defaults: CodexRequestOptions,
}

impl CodexCapabilities {
    pub fn conservative() -> Self {
        Self {
            efforts: ["none", "minimal", "low", "medium", "high"]
                .into_iter()
                .map(|level| Effort::Named(level.into()))
                .collect(),
            summaries: vec!["auto".into()],
            defaults: CodexRequestOptions {
                reasoning: Some(ReasoningOptions {
                    effort: Some(Effort::Named("high".into())),
                    ..ReasoningOptions::default()
                }),
                ..CodexRequestOptions::default()
            },
            ..Self::default()
        }
    }
}
