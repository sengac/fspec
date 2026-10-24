use super::{
    extract_thinking_config, internal_response_to_rig_choice, provider_error_to_rig,
    tools_for_rhai, RhaiCustomCompletion, RhaiCustomProviderModel,
};
use crate::custom::rig_message_convert::rig_messages_to_internal;
use rig::completion::{CompletionError, CompletionRequest, CompletionResponse, GetTokenUsage};

impl RhaiCustomProviderModel {
    pub(super) async fn complete_request(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<RhaiCustomCompletion>, CompletionError> {
        let preamble = request.preamble.as_deref();
        let history: Vec<rig::completion::Message> =
            request.chat_history.clone().into_iter().collect();
        let messages = rig_messages_to_internal(preamble, &history);
        let tools = tools_for_rhai(&request);
        let thinking = extract_thinking_config(&request);
        let provider = self.provider();
        let tool_names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        tracing::warn!(provider = %provider.config_name(), history_len = history.len(),
            internal_msg_count = messages.len(), tool_count = tools.len(), tool_names = ?tool_names,
            has_preamble = preamble.is_some(), has_thinking = thinking.is_some(),
            "[rhai-dispatch] CompletionModel::completion ENTER");
        let url = provider
            .invoke_build_url()
            .await
            .map_err(provider_error_to_rig)?;
        let headers = provider
            .invoke_build_headers()
            .await
            .map_err(provider_error_to_rig)?;
        let body = provider
            .invoke_request_builder("build_request", &messages, &tools, thinking)
            .await
            .map_err(provider_error_to_rig)?;
        let (status, body_text) = crate::custom::http::post_json(
            provider.http_client_handle(),
            provider.config_name(),
            &url,
            headers,
            &body,
        )
        .await
        .map_err(provider_error_to_rig)?;
        tracing::warn!(provider = %provider.config_name(), status, body_len = body_text.len(),
            "[rhai-dispatch] CompletionModel::completion received HTTP response");
        if !(200..300).contains(&status) {
            return Err(provider_error_to_rig(
                provider.invoke_map_error(status, &body_text).await,
            ));
        }
        let body_json = serde_json::from_str(&body_text).map_err(|error| {
            CompletionError::ResponseError(format!(
                "custom provider response was not JSON: {error}"
            ))
        })?;
        let (internal, usage_snapshot) = provider
            .invoke_parse_response_with_usage(&body_json)
            .await
            .map_err(provider_error_to_rig)?;
        let choice = internal_response_to_rig_choice(&internal);
        let raw = RhaiCustomCompletion::from_snapshot(internal.stop_reason, &usage_snapshot);
        let usage = raw.token_usage().unwrap_or_default();
        tracing::warn!(provider = %provider.config_name(), choice_len = choice.iter().count(),
            stop_reason = %raw.stop_reason, input_tokens = raw.input_tokens,
            output_tokens = raw.output_tokens, "[rhai-dispatch] CompletionModel::completion EXIT ok");
        Ok(CompletionResponse {
            choice,
            usage,
            raw_response: raw,
        })
    }
}
