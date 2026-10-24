use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexItem {
    Message {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        role: String,
        content: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phase: Option<String>,
    },
    CustomToolCall {
        id: String,
        call_id: String,
        name: String,
        input: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        namespace: Option<String>,
    },
    CustomToolCallOutput {
        call_id: String,
        output: Value,
    },
    Reasoning {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        summary: Vec<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
    },
    FunctionCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        name: String,
        arguments: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        namespace: Option<String>,
    },
    FunctionCallOutput {
        call_id: String,
        output: Value,
    },
    WebSearchCall {
        id: String,
        #[serde(flatten)]
        details: serde_json::Map<String, Value>,
    },
    #[serde(alias = "compaction_summary", alias = "context_compaction")]
    Compaction {
        encrypted_content: String,
        #[serde(flatten)]
        details: serde_json::Map<String, Value>,
    },
}

impl CodexItem {
    pub(crate) fn parse(mut value: Value) -> Result<Self, serde_json::Error> {
        if value.get("type").is_none() && value.get("role").is_some() {
            value["type"] = Value::String("message".into());
        }
        serde_json::from_value(value)
    }

    pub(super) fn is_reminder(&self) -> bool {
        let Self::Message { role, content, .. } = self else {
            return false;
        };
        role == "user"
            && (content.as_str().is_some_and(reminder_text)
                || content.as_array().is_some_and(|parts| {
                    !parts.is_empty()
                        && parts.iter().all(|part| {
                            part["type"] == "input_text"
                                && part["text"].as_str().is_some_and(reminder_text)
                        })
                }))
    }
}

fn reminder_text(text: &str) -> bool {
    let mut remaining = text.trim();
    if !remaining.starts_with("<system-reminder>") {
        return false;
    }
    while let Some(reminder) = remaining.strip_prefix("<system-reminder>") {
        let Some((body, rest)) = reminder.split_once("</system-reminder>") else {
            return false;
        };
        if body.contains("<!-- type:compaction-dag -->") {
            return false;
        }
        remaining = rest.trim();
    }
    remaining.is_empty()
}

impl std::fmt::Debug for CodexItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CodexItem { .. }")
    }
}
