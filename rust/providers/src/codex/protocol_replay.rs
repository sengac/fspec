use super::history::CodexItem;
use rig::completion::CompletionError;
use serde_json::Value;

fn keys(value: &Value, allowed: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.keys().all(|key| allowed.contains(&key.as_str())))
}

fn annotations(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        items.iter().all(|item| {
            keys(item, &["type", "url", "title", "start_index", "end_index"])
                && item["type"] == "url_citation"
                && item["url"]
                    .as_str()
                    .is_some_and(|url| url.starts_with("https://") || url.starts_with("http://"))
                && item["title"].is_string()
                && item["start_index"]
                    .as_u64()
                    .is_some_and(|start| item["end_index"].as_u64().is_some_and(|end| start <= end))
        })
    })
}

fn logprob(value: &Value, top: bool) -> bool {
    keys(
        value,
        if top {
            &["token", "logprob", "bytes"]
        } else {
            &["token", "logprob", "bytes", "top_logprobs"]
        },
    ) && value["token"].is_string()
        && value["logprob"].is_number()
        && value.get("bytes").is_none_or(|bytes| {
            bytes.is_null()
                || bytes.as_array().is_some_and(|bytes| {
                    bytes
                        .iter()
                        .all(|byte| byte.as_u64().is_some_and(|byte| byte <= 255))
                })
        })
        && (top
            || value.get("top_logprobs").is_none_or(|items| {
                items
                    .as_array()
                    .is_some_and(|items| items.iter().all(|item| logprob(item, true)))
            }))
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.is_empty())
}

fn optional_id(value: &Value) -> bool {
    value.get("id").is_none_or(text)
}

fn status(value: &Value) -> bool {
    value
        .get("status")
        .is_none_or(|status| matches!(status.as_str(), Some("completed" | "incomplete")))
}

fn string_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| !items.is_empty() && items.iter().all(text))
}

fn action(value: &Value) -> bool {
    match value["type"].as_str() {
        Some("search") => {
            keys(value, &["type", "query", "queries", "sources"])
                && (value.get("query").is_some_and(text)
                    || value.get("queries").is_some_and(string_array))
                && value.get("query").is_none_or(text)
                && value.get("queries").is_none_or(string_array)
                && value.get("sources").is_none_or(|sources| {
                    sources.as_array().is_some_and(|sources| {
                        sources.iter().all(|source| {
                            keys(source, &["type", "url", "title"])
                                && source["type"] == "url"
                                && text(&source["url"])
                                && source.get("title").is_none_or(Value::is_string)
                        })
                    })
                })
        }
        Some("open_page") => keys(value, &["type", "url"]) && text(&value["url"]),
        Some("find") => {
            keys(value, &["type", "url", "pattern"])
                && text(&value["url"])
                && text(&value["pattern"])
        }
        _ => false,
    }
}

fn valid(value: &Value) -> bool {
    let Ok(item) = CodexItem::parse(value.clone()) else {
        return false;
    };
    match item {
        CodexItem::Message { role, .. } => {
            value["type"] == "message"
                && role == "assistant"
                && keys(value, &["type", "id", "role", "content", "phase", "status"])
                && optional_id(value)
                && status(value)
                && value.get("phase").is_none_or(text)
                && value["content"].as_array().is_some_and(|parts| {
                    !parts.is_empty()
                        && parts.iter().all(|part| {
                            keys(part, &["type", "text", "annotations", "logprobs"])
                                && part["type"] == "output_text"
                                && text(&part["text"])
                                && part.get("annotations").is_none_or(annotations)
                                && part.get("logprobs").is_none_or(|items| {
                                    items.as_array().is_some_and(|items| {
                                        items.iter().all(|item| logprob(item, false))
                                    })
                                })
                        })
                })
        }
        CodexItem::Reasoning {
            summary,
            encrypted_content,
            ..
        } => {
            keys(
                value,
                &["type", "id", "summary", "encrypted_content", "status"],
            ) && optional_id(value)
                && status(value)
                && value.get("encrypted_content").is_none_or(text)
                && encrypted_content
                    .as_ref()
                    .is_none_or(|content| !content.is_empty())
                && (!summary.is_empty() || encrypted_content.is_some())
                && summary.iter().all(|part| {
                    keys(part, &["type", "text"])
                        && part["type"] == "summary_text"
                        && text(&part["text"])
                })
        }
        CodexItem::WebSearchCall { id, .. } => {
            !id.is_empty()
                && keys(value, &["type", "id", "status", "action"])
                && value["status"] == "completed"
                && action(&value["action"])
        }
        _ => false,
    }
}

pub(super) fn append(request: &mut Value, output: &[Value]) -> Result<(), CompletionError> {
    if output.is_empty() || !output.iter().all(valid) {
        return Err(CompletionError::ResponseError(
            "Codex protocol continuation requires supported well-formed typed output".into(),
        ));
    }
    request["input"]
        .as_array_mut()
        .ok_or_else(|| {
            CompletionError::RequestError("Codex continuation requires an input array".into())
        })?
        .extend_from_slice(output);
    Ok(())
}
