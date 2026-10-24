use super::completion::{
    CompletionModel, Message, SystemContent, ToolChoice, ToolDefinition, apply_cache_control,
};
use crate::completion::{CompletionError, CompletionRequest};
use crate::http_client::HttpClientExt;
use crate::json_utils::merge_inplace;
use serde_json::json;

impl<T: HttpClientExt> CompletionModel<T> {
    pub(super) fn streaming_request_bytes(
        &self,
        request: CompletionRequest,
    ) -> Result<Vec<u8>, CompletionError> {
        request.validate_tool_catalog_overrides()?;
        let max_tokens = request
            .max_tokens
            .or(self.default_max_tokens)
            .ok_or_else(|| {
                CompletionError::RequestError("`max_tokens` must be set for Anthropic".into())
            })?;
        let mut full_history = Vec::new();
        if let Some(documents) = request.normalized_documents() {
            full_history.push(documents);
        }
        full_history.extend(request.chat_history);
        let mut messages = full_history
            .into_iter()
            .map(Message::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let mut system = request
            .preamble
            .filter(|preamble| !preamble.is_empty())
            .map(|text| {
                vec![SystemContent::Text {
                    text,
                    cache_control: None,
                }]
            })
            .unwrap_or_default();
        if self.prompt_caching {
            apply_cache_control(&mut system, &mut messages);
        }
        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "stream": true,
        });
        if !system.is_empty() {
            merge_inplace(&mut body, json!({"system": system}));
        }
        if !self.client.ext().oauth_mode
            && let Some(temperature) = request.temperature
        {
            merge_inplace(&mut body, json!({"temperature": temperature}));
        }
        let tool_choice = if self.client.ext().oauth_mode {
            None
        } else {
            request.tool_choice.map(ToolChoice::try_from).transpose()?
        };
        let tools = request
            .tools
            .into_iter()
            .map(|tool| ToolDefinition {
                name: tool.name,
                description: Some(tool.description),
                input_schema: tool.parameters,
            })
            .collect::<Vec<_>>();
        if self.client.ext().oauth_mode {
            merge_inplace(&mut body, json!({"tools": tools}));
        } else if !tools.is_empty() {
            merge_inplace(
                &mut body,
                json!({"tools": tools, "tool_choice": tool_choice.unwrap_or_default()}),
            );
        }
        if let Some(params) = request.additional_params {
            merge_inplace(&mut body, params);
        }
        if self.client.ext().oauth_mode {
            merge_inplace(
                &mut body,
                json!({"metadata": {"user_id": self.oauth_user_id}}),
            );
        }
        Ok(self
            .client
            .http_client()
            .prepare_request_body(serde_json::to_vec(&body)?)?)
    }
}
