use super::{
    extract_thinking_config, provider_error_to_rig, tools_for_rhai, RhaiCustomCompletion,
    RhaiCustomProviderModel,
};
use crate::custom::rig_message_convert::rig_messages_to_internal;
use crate::custom::stream::{RhaiStreamProcessor, StreamChunk, StreamUsage};
use crate::custom::stream_http::open_stream;
use crate::StopReason;
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionRequest, GetTokenUsage};
use rig::streaming::{
    RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse, StreamingResult,
};

impl RhaiCustomProviderModel {
    pub(super) async fn stream_request(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<RhaiCustomCompletion>, CompletionError> {
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
            "[rhai-dispatch] CompletionModel::stream ENTER");
        let url = provider
            .invoke_build_url()
            .await
            .map_err(provider_error_to_rig)?;
        let headers = provider
            .invoke_build_headers()
            .await
            .map_err(provider_error_to_rig)?;
        let model = self.clone();
        let body = tokio::task::spawn_blocking(move || model.streaming_request_body(request))
            .await
            .map_err(|error| {
                CompletionError::ProviderError(format!("custom serializer task failed: {error}"))
            })??;
        let (status, byte_stream) = crate::custom::http::post_sse(
            provider.http_client_handle(),
            provider.config_name(),
            &url,
            headers,
            &body,
        )
        .await
        .map_err(provider_error_to_rig)?;
        tracing::warn!(provider = %provider.config_name(), status,
            "[rhai-dispatch] CompletionModel::stream opened SSE response");
        let processor = RhaiStreamProcessor::new(
            provider.engine_handle(),
            provider.ast_handle(),
            provider.config_name().to_string(),
            provider.config_dynamic_accessor(),
        );
        let provider_for_err = self.inner.clone();
        let chunk_stream = open_stream(
            processor,
            status,
            byte_stream,
            move |status, body_text| async move {
                provider_for_err.invoke_map_error(status, &body_text).await
            },
        );
        let provider_name = provider.config_name().to_string();
        let rig_stream: StreamingResult<RhaiCustomCompletion> = Box::pin(async_stream::stream! {
            let mut chunk_stream = chunk_stream;
            let mut final_stop = StopReason::EndTurn;
            let mut aggregate = StreamUsage::default();
            let mut pending_calls = Vec::new();
            while let Some(item) = chunk_stream.next().await {
                match item {
                    Ok(StreamChunk::TextDelta(text)) => yield Ok(RawStreamingChoice::Message(text)),
                    Ok(StreamChunk::ReasoningDelta(reasoning)) => {
                        yield Ok(RawStreamingChoice::ReasoningDelta { id: None, reasoning });
                    }
                    Ok(StreamChunk::ToolCallStart { .. }) => {}
                    Ok(StreamChunk::ToolCallArgsDelta { id, chunk }) => {
                        yield Ok(RawStreamingChoice::ToolCallDelta { id,
                            content: rig::streaming::ToolCallDeltaContent::Delta(chunk) });
                    }
                    Ok(StreamChunk::ToolCallComplete { id, name, input }) => {
                        tracing::warn!(provider = %provider_name, tool_id = %id, tool_name = %name,
                            "[rhai-dispatch] stream yielding ToolCallComplete tool_name={name} tool_id={id}");
                        pending_calls.push(RawStreamingToolCall::new(id, name, input)
                            .with_signature(None));
                    }
                    Ok(StreamChunk::UsageDelta(delta)) => {
                        aggregate.input_tokens = delta.input_tokens.or(aggregate.input_tokens);
                        aggregate.output_tokens = delta.output_tokens.or(aggregate.output_tokens);
                        aggregate.cache_read_input_tokens = delta.cache_read_input_tokens.or(aggregate.cache_read_input_tokens);
                        aggregate.cache_creation_input_tokens = delta.cache_creation_input_tokens.or(aggregate.cache_creation_input_tokens);
                        aggregate.reasoning_tokens = delta.reasoning_tokens.or(aggregate.reasoning_tokens);
                        if let Some(usage) = RhaiCustomCompletion::from_snapshot(final_stop, &aggregate).token_usage() {
                            yield Ok(RawStreamingChoice::Usage(usage));
                        }
                    }
                    Ok(StreamChunk::StopReason(reason)) => final_stop = reason,
                    Err(error) => { yield Err(provider_error_to_rig(error)); return; }
                }
            }
            yield Ok(RawStreamingChoice::FinalResponse(RhaiCustomCompletion::from_snapshot(final_stop, &aggregate)));
            for call in pending_calls {
                yield Ok(RawStreamingChoice::ToolCall(call));
            }
        });
        Ok(StreamingCompletionResponse::stream(rig_stream))
    }
}
