use super::{
    extract_thinking_config, provider_error_to_rig, tools_for_rhai, RhaiCustomProviderModel,
};
use crate::custom::conversion::dynamic_to_json_value;
use crate::custom::error_mapping::map_rhai_error_to_provider;
use crate::custom::request_bridge::request_to_rhai;
use crate::custom::rig_message_convert::rig_messages_to_internal;
use rhai::{Dynamic, Scope};
use rig::completion::{CompletionError, CompletionRequest};

impl RhaiCustomProviderModel {
    pub(super) fn streaming_request_body(
        &self,
        request: CompletionRequest,
    ) -> Result<serde_json::Value, CompletionError> {
        let mut history: Vec<_> = request
            .documents
            .iter()
            .map(|document| rig::message::Message::user(document.to_string()))
            .collect();
        history.extend(request.chat_history.clone());
        let messages = rig_messages_to_internal(request.preamble.as_deref(), &history);
        let tools = tools_for_rhai(&request);
        let argument = request_to_rhai(
            &messages,
            &tools,
            extract_thinking_config(&request).as_ref(),
        )
        .map_err(crate::error::ProviderError::from)
        .map_err(provider_error_to_rig)?;
        let provider = self.provider();
        let body = provider
            .engine_handle()
            .call_fn::<Dynamic>(
                &mut Scope::new(),
                &provider.ast_handle(),
                "build_stream_request",
                (argument,),
            )
            .map_err(|error| {
                provider_error_to_rig(map_rhai_error_to_provider(
                    provider.config_name(),
                    "build_stream_request",
                    &error,
                ))
            })?;
        let body = dynamic_to_json_value(&body);
        crate::tool_contract::validate_catalog(&body, &tools).map_err(provider_error_to_rig)?;
        Ok(body)
    }
}
