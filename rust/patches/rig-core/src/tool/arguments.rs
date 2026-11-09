//! BUG-303: tool-call argument normalization.
//!
//! Tool calls must carry a JSON object as arguments. Models sometimes emit
//! `null`, a non-object value, or JSON truncated mid-object (for example when
//! the output limit is reached while writing a large file). Such a call is
//! never executed: its recorded arguments become an empty object so every
//! later provider request stays wire-valid, and the model receives guidance
//! built from the tool's own definition explaining how to retry.

use crate::completion::ToolDefinition;
use serde_json::{Map, Value};

const EXCERPT_CHARS: usize = 160;

/// What the model sent instead of a JSON object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidToolArguments {
    Null,
    /// A text payload that is not a complete JSON object.
    Text {
        chars: usize,
        excerpt: String,
    },
    /// A complete JSON value of the wrong type (array, number, boolean).
    Value {
        kind: &'static str,
    },
}

/// Replace non-object arguments with an empty object, returning what was
/// received when the arguments were invalid. A string holding a JSON object
/// (also when encoded twice) is accepted as that object.
pub fn normalize(arguments: &mut Value) -> Option<InvalidToolArguments> {
    let invalid = match std::mem::take(arguments) {
        Value::Object(object) => {
            *arguments = Value::Object(object);
            return None;
        }
        Value::String(text) => match decode_object(&text) {
            Some(object) => {
                *arguments = Value::Object(object);
                return None;
            }
            None => InvalidToolArguments::Text {
                chars: text.chars().count(),
                excerpt: text.chars().take(EXCERPT_CHARS).collect(),
            },
        },
        Value::Null => InvalidToolArguments::Null,
        Value::Array(_) => InvalidToolArguments::Value { kind: "array" },
        Value::Number(_) => InvalidToolArguments::Value { kind: "number" },
        Value::Bool(_) => InvalidToolArguments::Value { kind: "boolean" },
    };
    *arguments = Value::Object(Map::new());
    Some(invalid)
}

/// Wire form of possibly-invalid arguments: always a JSON object.
pub fn wire_object(arguments: &Value) -> Value {
    let mut normalized = arguments.clone();
    normalize(&mut normalized);
    normalized
}

fn decode_object(text: &str) -> Option<Map<String, Value>> {
    match serde_json::from_str::<Value>(text).ok()? {
        Value::Object(object) => Some(object),
        Value::String(inner) => match serde_json::from_str::<Value>(&inner).ok()? {
            Value::Object(object) => Some(object),
            _ => None,
        },
        _ => None,
    }
}

impl InvalidToolArguments {
    /// The tool result returned to the model in place of executing the tool.
    pub fn guidance(&self, tool_name: &str, definition: Option<&ToolDefinition>) -> String {
        let received = match self {
            Self::Null => "received null".to_string(),
            Self::Text { chars, excerpt } => format!(
                "received a {chars}-character value that is not a complete JSON object (it begins: {excerpt:?})"
            ),
            Self::Value { kind } => format!("received a JSON {kind}"),
        };
        let mut guidance = format!(
            "Tool call rejected before execution: `{tool_name}` arguments must be one JSON object, but the call {received}. \
             The tool was NOT run and nothing changed. Call `{tool_name}` again with a single complete JSON object."
        );
        if let Some(definition) = definition {
            let required = definition
                .parameters
                .get("required")
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|fields| !fields.is_empty());
            if let Some(required) = required {
                guidance.push_str(&format!(" Required parameters: {required}."));
            }
            guidance.push_str(&format!(" Parameter schema: {}", definition.parameters));
        }
        if matches!(self, Self::Text { .. }) {
            guidance.push_str(
                " If the arguments were cut off because the content is large, split the work into smaller calls \
                 (for example write the first part of a file, then append or edit the rest).",
            );
        }
        guidance
    }
}
