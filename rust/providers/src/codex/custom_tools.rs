use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Codex tool dialect: {0}")]
pub struct ToolDialectError(String);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRegistration {
    pub namespace: Option<String>,
    pub name: String,
    pub executor: String,
    pub grammar: Option<String>,
    pub raw_input: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CustomToolRegistry {
    registrations: Vec<ToolRegistration>,
}

fn invalid(message: impl Into<String>) -> ToolDialectError {
    ToolDialectError(message.into())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

impl CustomToolRegistry {
    pub(super) fn attributed(mut self) -> Self {
        for registration in &mut self.registrations {
            registration.raw_input = false;
            registration.grammar = None;
        }
        self
    }

    pub fn new(registrations: Vec<ToolRegistration>) -> Result<Self, ToolDialectError> {
        let mut names = HashSet::new();
        let mut executors = HashSet::new();
        for registration in &registrations {
            if !valid_name(&registration.name)
                || !valid_name(&registration.executor)
                || registration
                    .namespace
                    .as_ref()
                    .is_some_and(|name| !valid_name(name))
            {
                return Err(invalid("tool and namespace names must be ASCII letters, digits, underscores or hyphens"));
            }
            if !names.insert((registration.namespace.clone(), registration.name.clone()))
                || !executors.insert(registration.executor.clone())
            {
                return Err(invalid("duplicate qualified tool or executor registration"));
            }
            if registration.raw_input && registration.executor != "apply_patch" {
                return Err(invalid(
                    "raw input is supported only by the apply_patch executor",
                ));
            }
            if registration
                .grammar
                .as_ref()
                .is_some_and(|grammar| grammar.trim().is_empty())
                || (!registration.raw_input && registration.grammar.is_some())
            {
                return Err(invalid(
                    "grammar requires a raw-input tool and a nonempty definition",
                ));
            }
        }
        Ok(Self { registrations })
    }

    pub fn register(&self, tools: &mut Value) -> Result<(), ToolDialectError> {
        let originals = tools
            .as_array()
            .ok_or_else(|| invalid("tools must be an array"))?;
        let mut result = originals.clone();
        for registration in &self.registrations {
            let index = result
                .iter()
                .position(|tool| {
                    tool["type"] == "function" && tool["name"] == registration.executor
                })
                .ok_or_else(|| {
                    invalid(format!(
                        "executor {} is not registered",
                        registration.executor
                    ))
                })?;
            let mut definition = result.remove(index);
            if registration.raw_input {
                definition = json!({"type":"custom", "name":registration.name,
                    "description":definition["description"],
                    "format": registration.grammar.as_ref().map_or_else(
                        || json!({"type":"text"}),
                        |grammar| json!({"type":"grammar","syntax":"lark","definition":grammar}))});
            } else {
                definition["name"] = json!(registration.name);
            }
            if let Some(namespace) = &registration.namespace {
                if let Some(existing) = result
                    .iter_mut()
                    .find(|tool| tool["type"] == "namespace" && tool["name"] == *namespace)
                {
                    existing["tools"]
                        .as_array_mut()
                        .ok_or_else(|| invalid("invalid namespace tools"))?
                        .push(definition);
                } else {
                    result.push(json!({"type":"namespace","name":namespace,"description":"Local tools","tools":[definition]}));
                }
            } else {
                result.push(definition);
            }
        }
        *tools = Value::Array(result);
        Ok(())
    }

    pub fn normalize_call(&self, item: &Value) -> Result<Value, ToolDialectError> {
        let kind = item["type"]
            .as_str()
            .ok_or_else(|| invalid("missing call type"))?;
        if !matches!(kind, "function_call" | "custom_tool_call") {
            return Err(invalid("unsupported executable item"));
        }
        let namespace = item["namespace"].as_str();
        let name = item["name"]
            .as_str()
            .ok_or_else(|| invalid("missing tool name"))?;
        let registration = self.registrations.iter().find(|registration| {
            registration.namespace.as_deref() == namespace && registration.name == name
        });
        let Some(registration) = registration else {
            if kind == "function_call"
                && namespace.is_none()
                && !self
                    .registrations
                    .iter()
                    .any(|entry| entry.executor == name)
            {
                return Ok(item.clone());
            }
            return Err(invalid(
                "unregistered custom or namespaced tool; enable a supported dialect",
            ));
        };
        if (kind == "custom_tool_call") != registration.raw_input {
            return Err(invalid("call type does not match registered tool dialect"));
        }
        if item["status"]
            .as_str()
            .is_some_and(|status| status != "completed")
        {
            return Err(invalid("only completed tool items may execute"));
        }
        for key in ["id", "call_id"] {
            if item[key].as_str().is_none_or(str::is_empty) {
                return Err(invalid(format!("missing {key}")));
            }
        }
        let mut normalized = item.clone();
        normalized["type"] = json!("function_call");
        normalized["name"] = json!(registration.executor);
        if registration.raw_input {
            let input = item["input"]
                .as_str()
                .ok_or_else(|| invalid("custom input must be raw text"))?;
            normalized["arguments"] = json!(json!({"patch":input}).to_string());
        }
        if let Some(object) = normalized.as_object_mut() {
            object.remove("input");
            object.remove("namespace");
        }
        Ok(normalized)
    }

    pub fn restore_history(
        &self,
        input: &mut Value,
        originals: &[Value],
    ) -> Result<(), ToolDialectError> {
        let input = input
            .as_array_mut()
            .ok_or_else(|| invalid("history must be an array"))?;
        for item in input {
            let Some(original) = originals.iter().find(|original| {
                original["call_id"].is_string() && original["call_id"] == item["call_id"]
            }) else {
                continue;
            };
            if item["type"] == "function_call" {
                *item = original.clone();
            } else if item["type"] == "function_call_output"
                && original["type"] == "custom_tool_call"
            {
                item["type"] = json!("custom_tool_call_output");
            }
        }
        Ok(())
    }
}
