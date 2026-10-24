use super::memory_policy;
use rig::completion::CompletionError;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Default)]
pub struct NativeCapabilities {
    pub hosted_search: bool,
}

impl NativeCapabilities {
    pub(super) fn apply(self, request: &mut Value) {
        if self.hosted_search {
            if !request["tools"].is_array() {
                request["tools"] = serde_json::json!([]);
            }
            if let Some(tools) = request["tools"].as_array_mut() {
                tools.push(serde_json::json!({"type":"web_search"}));
            }
        }
    }
}

#[derive(Default)]
pub struct NativeOutput {
    search: bool,
    custom: bool,
    presented: HashSet<String>,
}

impl NativeOutput {
    pub fn from_request(request: &Value) -> Self {
        Self {
            search: has_type(&request["tools"], "web_search"),
            custom: request["tools"].as_array().is_some_and(|tools| {
                tools
                    .iter()
                    .any(|tool| tool["type"] == "custom" || tool["type"] == "namespace")
            }),
            presented: HashSet::new(),
        }
    }

    pub fn event(&mut self, kind: &str, event: &Value) -> Result<Option<String>, CompletionError> {
        if let Some(status) = kind.strip_prefix("response.web_search_call.") {
            self.require("web_search_call", self.search)?;
            let id = required(event, "item_id")?;
            return Ok(self.notice(format!(
                "\n[Codex hosted search {}: {}]\n",
                visible(id),
                visible(status)
            )));
        }
        if kind == "response.output_item.done" {
            let notices = self.terminal(&[event["item"].clone()])?;
            return Ok((!notices.is_empty()).then(|| notices.join("")));
        }
        if kind == "response.output_item.added" {
            self.validate(&event["item"])?;
        }
        Ok(None)
    }

    pub fn terminal(&mut self, items: &[Value]) -> Result<Vec<String>, CompletionError> {
        for item in items {
            self.validate(item)?;
        }
        let mut notices = Vec::new();
        for item in items {
            if item["type"] == "web_search_call" {
                let id = required(item, "id")?;
                let status = item["status"].as_str().unwrap_or("completed");
                let notice = format!(
                    "\n[Codex hosted search {}: {}]\n",
                    visible(id),
                    visible(status)
                );
                if let Some(notice) = self.notice(notice) {
                    notices.push(notice);
                }
            }
            if item["type"] == "message" && self.search {
                let id = item["id"].as_str().unwrap_or("assistant");
                if let Some(content) = item["content"].as_array() {
                    for part in content {
                        if let Some(annotations) = part["annotations"].as_array() {
                            for annotation in annotations {
                                if annotation["type"] != "url_citation" {
                                    continue;
                                }
                                let url = required(annotation, "url")?;
                                if !url.starts_with("https://") && !url.starts_with("http://") {
                                    return Err(compatibility(
                                        "url_citation",
                                        "invalid source URL",
                                    ));
                                }
                                let title = annotation["title"].as_str().unwrap_or("Source");
                                let notice = format!(
                                    "\n[Codex source {}] {}: {}\n",
                                    visible(id),
                                    visible(title),
                                    visible(url)
                                );
                                if let Some(notice) = self.notice(notice) {
                                    notices.push(notice);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(notices)
    }

    fn validate(&self, item: &Value) -> Result<(), CompletionError> {
        memory_policy::validate_item(item)?;
        let family = required(item, "type")?;
        match family {
            "message" | "reasoning" | "function_call" | "function_call_output" => Ok(()),
            "custom_tool_call" | "custom_tool_call_output" => self.require(family, self.custom),
            "web_search_call" => {
                self.require(family, self.search)?;
                required(item, "id")?;
                Ok(())
            }
            _ => Err(compatibility(family, "deferred or unknown output family")),
        }
    }

    fn require(&self, family: &str, enabled: bool) -> Result<(), CompletionError> {
        if enabled {
            Ok(())
        } else {
            Err(compatibility(family, "capability not negotiated"))
        }
    }

    fn notice(&mut self, notice: String) -> Option<String> {
        self.presented.insert(notice.clone()).then_some(notice)
    }
}

fn has_type(items: &Value, kind: &str) -> bool {
    items
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["type"] == kind))
}

fn required<'value>(value: &'value Value, key: &str) -> Result<&'value str, CompletionError> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| compatibility("native output", &format!("missing or invalid {key}")))
}

fn visible(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .take(512)
        .collect()
}

fn compatibility(family: &str, reason: &str) -> CompletionError {
    CompletionError::ResponseError(format!(
        "Codex compatibility: {}: {reason}; disable this native capability or upgrade the adapter; no local tool was executed",
        visible(family)
    ))
}
