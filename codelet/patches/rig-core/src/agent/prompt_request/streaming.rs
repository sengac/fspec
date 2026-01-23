use crate::{
    OneOrMany,
    agent::CancelSignal,
    completion::GetTokenUsage,
    json_utils,
    message::{AssistantContent, ImageMediaType, MimeType, Reasoning, ToolResult, ToolResultContent, UserContent},
    streaming::{StreamedAssistantContent, StreamedUserContent, StreamingCompletion},
    wasm_compat::{WasmBoxedFuture, WasmCompatSend},
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc};
use tokio::sync::RwLock;
use tracing::info_span;
use tracing_futures::Instrument;

use crate::{
    agent::Agent,
    completion::{CompletionError, CompletionModel, PromptError},
    message::{Message, Text},
    tool::ToolSetError,
};

/// Parse tool result string and convert to appropriate ToolResultContent(s).
/// Detects image responses from Read tool and converts to ToolResultContent::Image.
/// Returns a Vec to support multiple images (e.g., PDF visual mode pages).
fn parse_tool_result_content(result: &str) -> Vec<ToolResultContent> {
    // Try to parse as JSON to detect structured tool output
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(result) {
        // Handle FileOperationResult wrapper: {"success":true,"content":"..."}
        // The content field may contain a JSON string with image data
        let json = if json.get("success").is_some() {
            if let Some(content_str) = json.get("content").and_then(|c| c.as_str()) {
                // Parse the content field as JSON
                match serde_json::from_str::<serde_json::Value>(content_str) {
                    Ok(inner_json) => inner_json,
                    Err(_) => return vec![ToolResultContent::text(content_str)],
                }
            } else {
                json
            }
        } else if let Some(inner_str) = json.as_str() {
            // Handle double-serialization: if the parsed JSON is a string, parse it again
            // This happens when tool results are JSON-encoded strings containing JSON
            match serde_json::from_str::<serde_json::Value>(inner_str) {
                Ok(inner_json) => inner_json,
                Err(_) => return vec![ToolResultContent::text(inner_str)],
            }
        } else {
            json
        };

        // Check for PDF visual mode: {"path":"...", "total_pages":N, "pages":[{"page_number":N, "data":"...", "media_type":"..."}]}
        if json.get("pages").is_some() && json.get("total_pages").is_some() {
            if let Some(pages) = json.get("pages").and_then(|p| p.as_array()) {
                let mut contents = Vec::with_capacity(pages.len());
                for page in pages {
                    let data = page.get("data").and_then(|d| d.as_str());
                    let media_type_str = page.get("media_type").and_then(|m| m.as_str());

                    if let (Some(data), Some(media_type_str)) = (data, media_type_str) {
                        if let Some(media_type) = ImageMediaType::from_mime_type(media_type_str) {
                            let page_num = page.get("page_number").and_then(|n| n.as_u64()).unwrap_or(0);
                            tracing::info!("parse_tool_result_content: PDF page {} as Image, media_type={:?}", page_num, media_type);
                            contents.push(ToolResultContent::image_base64(data, Some(media_type), None));
                        }
                    }
                }
                if !contents.is_empty() {
                    return contents;
                }
            }
        }

        // Check for image type from Read tool: {"type":"image","data":"...","media_type":"..."}
        let type_field = json.get("type").and_then(|t| t.as_str());

        if type_field == Some("image") {
            let data = json.get("data").and_then(|d| d.as_str());
            let media_type_str = json.get("media_type").and_then(|m| m.as_str());

            if let (Some(data), Some(media_type_str)) = (data, media_type_str) {
                // Parse the media type string to ImageMediaType
                if let Some(media_type) = ImageMediaType::from_mime_type(media_type_str) {
                    tracing::info!("parse_tool_result_content: returning Image, media_type={:?}", media_type);
                    return vec![ToolResultContent::image_base64(data, Some(media_type), None)];
                }
            }
        }
        // Check for text type from Read tool: {"type":"text","content":"..."}
        // The content might contain nested JSON (e.g., PDF visual mode output)
        if type_field == Some("text") {
            if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                // Try to parse content as JSON to check for nested PDF pages
                if let Ok(inner_json) = serde_json::from_str::<serde_json::Value>(content) {
                    // Check if the nested JSON is PDF visual mode output
                    if inner_json.get("pages").is_some() && inner_json.get("total_pages").is_some() {
                        if let Some(pages) = inner_json.get("pages").and_then(|p| p.as_array()) {
                            let mut contents = Vec::with_capacity(pages.len());
                            for page in pages {
                                let data = page.get("data").and_then(|d| d.as_str());
                                let media_type_str = page.get("media_type").and_then(|m| m.as_str());

                                if let (Some(data), Some(media_type_str)) = (data, media_type_str) {
                                    if let Some(media_type) = ImageMediaType::from_mime_type(media_type_str) {
                                        let page_num = page.get("page_number").and_then(|n| n.as_u64()).unwrap_or(0);
                                        tracing::info!("parse_tool_result_content: nested PDF page {} as Image, media_type={:?}", page_num, media_type);
                                        contents.push(ToolResultContent::image_base64(data, Some(media_type), None));
                                    }
                                }
                            }
                            if !contents.is_empty() {
                                return contents;
                            }
                        }
                    }
                }
                // Not nested PDF, return as plain text
                return vec![ToolResultContent::text(content)];
            }
        }
    }
    // Default: treat as plain text
    vec![ToolResultContent::text(result)]
}

/// Convert Vec<ToolResultContent> to OneOrMany<ToolResultContent>
fn vec_to_one_or_many(contents: Vec<ToolResultContent>) -> OneOrMany<ToolResultContent> {
    match contents.len() {
        0 => OneOrMany::one(ToolResultContent::text("")), // Fallback for empty
        1 => OneOrMany::one(contents.into_iter().next().expect("length checked")),
        _ => OneOrMany::many(contents).expect("Vec with >1 items should succeed"),
    }
}

#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
pub type StreamingResult<R> =
    Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>> + Send>>;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub type StreamingResult<R> =
    Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>>>>;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum MultiTurnStreamItem<R> {
    /// A streamed assistant content item.
    StreamAssistantItem(StreamedAssistantContent<R>),
    /// A streamed user content item (mostly for tool results).
    StreamUserItem(StreamedUserContent),
    /// Early usage information (emitted at stream start with input tokens).
    Usage(crate::completion::Usage),
    /// The final result from the stream.
    FinalResponse(FinalResponse),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FinalResponse {
    response: String,
    aggregated_usage: crate::completion::Usage,
}

impl FinalResponse {
    pub fn empty() -> Self {
        Self {
            response: String::new(),
            aggregated_usage: crate::completion::Usage::new(),
        }
    }

    pub fn response(&self) -> &str {
        &self.response
    }

    pub fn usage(&self) -> crate::completion::Usage {
        self.aggregated_usage
    }
}

impl<R> MultiTurnStreamItem<R> {
    pub(crate) fn stream_item(item: StreamedAssistantContent<R>) -> Self {
        Self::StreamAssistantItem(item)
    }

    pub fn final_response(response: &str, aggregated_usage: crate::completion::Usage) -> Self {
        Self::FinalResponse(FinalResponse {
            response: response.to_string(),
            aggregated_usage,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamingError {
    #[error("{0}")]
    Completion(#[from] CompletionError),
    #[error("{0}")]
    Prompt(#[from] Box<PromptError>),
    #[error("{0}")]
    Tool(#[from] ToolSetError),
}

/// A builder for creating prompt requests with customizable options.
/// Uses generics to track which options have been set during the build process.
///
/// If you expect to continuously call tools, you will want to ensure you use the `.multi_turn()`
/// argument to add more turns as by default, it is 0 (meaning only 1 tool round-trip). Otherwise,
/// attempting to await (which will send the prompt request) can potentially return
/// [`crate::completion::request::PromptError::MaxDepthError`] if the agent decides to call tools
/// back to back.
pub struct StreamingPromptRequest<M, P>
where
    M: CompletionModel,
    P: StreamingPromptHook<M> + 'static,
{
    /// The prompt message to send to the model
    prompt: Message,
    /// Optional chat history to include with the prompt
    /// Note: chat history needs to outlive the agent as it might be used with other agents
    chat_history: Option<Vec<Message>>,
    /// Maximum depth for multi-turn conversations (0 means no multi-turn)
    max_depth: usize,
    /// The agent to use for execution
    agent: Arc<Agent<M>>,
    /// Optional per-request hook for events
    hook: Option<P>,
}

impl<M, P> StreamingPromptRequest<M, P>
where
    M: CompletionModel + 'static,
    <M as CompletionModel>::StreamingResponse: WasmCompatSend + GetTokenUsage,
    P: StreamingPromptHook<M>,
{
    /// Create a new PromptRequest with the given prompt and model
    pub fn new(agent: Arc<Agent<M>>, prompt: impl Into<Message>) -> Self {
        Self {
            prompt: prompt.into(),
            chat_history: None,
            max_depth: 0,
            agent,
            hook: None,
        }
    }

    /// Set the maximum depth for multi-turn conversations (ie, the maximum number of turns an LLM can have calling tools before writing a text response).
    /// If the maximum turn number is exceeded, it will return a [`crate::completion::request::PromptError::MaxDepthError`].
    pub fn multi_turn(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Add chat history to the prompt request
    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        self.chat_history = Some(history);
        self
    }

    /// Attach a per-request hook for tool call events
    pub fn with_hook<P2>(self, hook: P2) -> StreamingPromptRequest<M, P2>
    where
        P2: StreamingPromptHook<M>,
    {
        StreamingPromptRequest {
            prompt: self.prompt,
            chat_history: self.chat_history,
            max_depth: self.max_depth,
            agent: self.agent,
            hook: Some(hook),
        }
    }

    async fn send(self) -> StreamingResult<M::StreamingResponse> {
        let agent_span = if tracing::Span::current().is_disabled() {
            info_span!(
                "invoke_agent",
                gen_ai.operation.name = "invoke_agent",
                gen_ai.agent.name = self.agent.name(),
                gen_ai.system_instructions = self.agent.preamble,
                gen_ai.prompt = tracing::field::Empty,
                gen_ai.completion = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        let prompt = self.prompt;
        if let Some(text) = prompt.rag_text() {
            agent_span.record("gen_ai.prompt", text);
        }

        let agent = self.agent;

        let chat_history = if let Some(history) = self.chat_history {
            Arc::new(RwLock::new(history))
        } else {
            Arc::new(RwLock::new(vec![]))
        };

        let mut current_max_depth = 0;
        let mut last_prompt_error = String::new();

        let mut last_text_response = String::new();
        let mut is_text_response = false;
        let mut max_depth_reached = false;

        let mut aggregated_usage = crate::completion::Usage::new();

        let cancel_signal = CancelSignal::new();

        // NOTE: We use .instrument(agent_span) instead of span.enter() to avoid
        // span context leaking to other concurrent tasks. Using span.enter() inside
        // async_stream::stream! holds the guard across yield points, which causes
        // thread-local span context to leak when other tasks run on the same thread.
        // See: https://docs.rs/tracing/latest/tracing/span/struct.Span.html#in-asynchronous-code
        // See also: https://github.com/rust-lang/rust-clippy/issues/8722
        let stream = async_stream::stream! {
            let mut current_prompt = prompt.clone();
            let mut did_call_tool = false;

            'outer: loop {
                if current_max_depth > self.max_depth + 1 {
                    last_prompt_error = current_prompt.rag_text().unwrap_or_default();
                    max_depth_reached = true;
                    break;
                }

                current_max_depth += 1;

                if self.max_depth > 1 {
                    tracing::trace!(
                        "Conversation depth: {}/{}",
                        current_max_depth,
                        self.max_depth
                    );
                }

                if let Some(ref hook) = self.hook {
                    let reader = chat_history.read().await;
                    hook.on_completion_call(&current_prompt, &reader.to_vec(), cancel_signal.clone())
                        .await;

                    if cancel_signal.is_cancelled() {
                        yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                    }
                }

                let chat_stream_span = info_span!(
                    target: "rig::agent_chat",
                    parent: tracing::Span::current(),
                    "chat_streaming",
                    gen_ai.operation.name = "chat",
                    gen_ai.system_instructions = &agent.preamble,
                    gen_ai.provider.name = tracing::field::Empty,
                    gen_ai.request.model = tracing::field::Empty,
                    gen_ai.response.id = tracing::field::Empty,
                    gen_ai.response.model = tracing::field::Empty,
                    gen_ai.usage.output_tokens = tracing::field::Empty,
                    gen_ai.usage.input_tokens = tracing::field::Empty,
                    gen_ai.input.messages = tracing::field::Empty,
                    gen_ai.output.messages = tracing::field::Empty,
                );

                let mut stream = tracing::Instrument::instrument(
                    agent
                    .stream_completion(current_prompt.clone(), (*chat_history.read().await).clone())
                    .await?
                    .stream(), chat_stream_span
                )

                .await?;

                chat_history.write().await.push(current_prompt.clone());

                let mut tool_calls = vec![];
                let mut tool_results = vec![];
                let mut reasoning_blocks: Vec<AssistantContent> = vec![];

                while let Some(content) = stream.next().await {
                    match content {
                        Ok(StreamedAssistantContent::Text(text)) => {
                            if !is_text_response {
                                last_text_response = String::new();
                                is_text_response = true;
                            }
                            last_text_response.push_str(&text.text);
                            if let Some(ref hook) = self.hook {
                                hook.on_text_delta(&text.text, &last_text_response, cancel_signal.clone()).await;
                                if cancel_signal.is_cancelled() {
                                    yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                                }
                            }
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Text(text)));
                            did_call_tool = false;
                        },
                        Ok(StreamedAssistantContent::ToolCall(tool_call)) => {
                            let tool_span = info_span!(
                                parent: tracing::Span::current(),
                                "execute_tool",
                                gen_ai.operation.name = "execute_tool",
                                gen_ai.tool.type = "function",
                                gen_ai.tool.name = tracing::field::Empty,
                                gen_ai.tool.call.id = tracing::field::Empty,
                                gen_ai.tool.call.arguments = tracing::field::Empty,
                                gen_ai.tool.call.result = tracing::field::Empty
                            );

                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::ToolCall(tool_call.clone())));

                            let tc_result = async {
                                let tool_span = tracing::Span::current();
                                let tool_args = json_utils::value_to_json_string(&tool_call.function.arguments);
                                if let Some(ref hook) = self.hook {
                                    hook.on_tool_call(&tool_call.function.name, tool_call.call_id.clone(), &tool_args, cancel_signal.clone()).await;
                                    if cancel_signal.is_cancelled() {
                                        return Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                                    }
                                }

                                tool_span.record("gen_ai.tool.name", &tool_call.function.name);
                                tool_span.record("gen_ai.tool.call.arguments", &tool_args);

                                let tool_result = match
                                agent.tool_server_handle.call_tool(&tool_call.function.name, &tool_args).await {
                                    Ok(thing) => thing,
                                    Err(e) => {
                                        tracing::warn!("Error while calling tool: {e}");
                                        e.to_string()
                                    }
                                };

                                tool_span.record("gen_ai.tool.call.result", &tool_result);

                                if let Some(ref hook) = self.hook {
                                    hook.on_tool_result(&tool_call.function.name, tool_call.call_id.clone(), &tool_args, &tool_result.to_string(), cancel_signal.clone())
                                    .await;

                                    if cancel_signal.is_cancelled() {
                                        return Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                                    }
                                }

                                let tool_call_msg = AssistantContent::ToolCall(tool_call.clone());

                                tool_calls.push(tool_call_msg);
                                tool_results.push((tool_call.id.clone(), tool_call.call_id.clone(), tool_result.clone()));

                                did_call_tool = true;
                                Ok(tool_result)
                            }.instrument(tool_span).await;

                            match tc_result {
                                Ok(text) => {
                                    let tr = ToolResult { id: tool_call.id, call_id: tool_call.call_id, content: vec_to_one_or_many(parse_tool_result_content(&text)) };
                                    yield Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(tr)));
                                }
                                Err(e) => {
                                    yield Err(e);
                                }
                            }
                        },
                        Ok(StreamedAssistantContent::ToolCallDelta { id, content }) => {
                            if let Some(ref hook) = self.hook {
                                let (name, delta) = match &content {
                                    rig::streaming::ToolCallDeltaContent::Name(n) => (Some(n.as_str()), ""),
                                    rig::streaming::ToolCallDeltaContent::Delta(d) => (None, d.as_str()),
                                };
                                hook.on_tool_call_delta(&id, name, delta, cancel_signal.clone())
                                .await;

                                if cancel_signal.is_cancelled() {
                                    yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                                }
                            }
                        }
                        Ok(StreamedAssistantContent::Reasoning(rig::message::Reasoning { reasoning, id, signature })) => {
                            // Only capture FINAL reasoning blocks (those with signature) for chat history
                            // Claude requires signature when sending thinking blocks back in multi-turn
                            // Delta chunks have signature: None, final blocks have signature: Some(...)
                            if signature.is_some() {
                                reasoning_blocks.push(AssistantContent::Reasoning(Reasoning {
                                    reasoning: reasoning.clone(),
                                    id: id.clone(),
                                    signature: signature.clone(),
                                }));
                            }
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Reasoning(rig::message::Reasoning { reasoning, id, signature })));
                            did_call_tool = false;
                        },
                        Ok(StreamedAssistantContent::ReasoningDelta { reasoning, id }) => {
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::ReasoningDelta { reasoning, id }));
                            did_call_tool = false;
                        },
                        Ok(StreamedAssistantContent::Usage(usage)) => {
                            // Forward usage events for real-time token streaming
                            yield Ok(MultiTurnStreamItem::Usage(usage));
                        },
                        Ok(StreamedAssistantContent::Final(final_resp)) => {
                            if let Some(usage) = final_resp.token_usage() { aggregated_usage += usage; };

                            // Always call on_stream_completion_response_finish for token tracking
                            // This ensures compaction hooks get updated token counts even for
                            // tool-only responses (where is_text_response is false)
                            if let Some(ref hook) = self.hook {
                                hook.on_stream_completion_response_finish(&prompt, &final_resp, cancel_signal.clone()).await;

                                if cancel_signal.is_cancelled() {
                                    yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
                                }
                            }

                            if is_text_response {
                                tracing::Span::current().record("gen_ai.completion", &last_text_response);
                                yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Final(final_resp)));
                                is_text_response = false;
                            }
                        }
                        Err(e) => {
                            yield Err(e.into());
                            break 'outer;
                        }
                    }
                }

                // Add (parallel) tool calls to chat history
                // When thinking is enabled, reasoning blocks MUST come before tool_use blocks
                if !tool_calls.is_empty() {
                    let mut assistant_content = reasoning_blocks.clone();
                    assistant_content.extend(tool_calls.clone());
                    chat_history.write().await.push(Message::Assistant {
                        id: None,
                        content: OneOrMany::many(assistant_content).expect("Impossible EmptyListError"),
                    });
                    reasoning_blocks.clear();
                }

                // Add tool results to chat history
                for (id, call_id, tool_result) in tool_results {
                    if let Some(call_id) = call_id {
                        chat_history.write().await.push(Message::User {
                            content: OneOrMany::one(UserContent::tool_result_with_call_id(
                                &id,
                                call_id.clone(),
                                vec_to_one_or_many(parse_tool_result_content(&tool_result)),
                            )),
                        });
                    } else {
                        chat_history.write().await.push(Message::User {
                            content: OneOrMany::one(UserContent::tool_result(
                                &id,
                                vec_to_one_or_many(parse_tool_result_content(&tool_result)),
                            )),
                        });
                    }
                }

                // Set the current prompt to the last message in the chat history
                current_prompt = match chat_history.write().await.pop() {
                    Some(prompt) => prompt,
                    None => unreachable!("Chat history should never be empty at this point"),
                };

                if !did_call_tool {
                    let current_span = tracing::Span::current();
                    current_span.record("gen_ai.usage.input_tokens", aggregated_usage.input_tokens);
                    current_span.record("gen_ai.usage.output_tokens", aggregated_usage.output_tokens);
                    tracing::info!("Agent multi-turn stream finished");
                    yield Ok(MultiTurnStreamItem::final_response(&last_text_response, aggregated_usage));
                    break;
                }
            }

            if max_depth_reached {
                yield Err(Box::new(PromptError::MaxDepthError {
                    max_depth: self.max_depth,
                    chat_history: Box::new((*chat_history.read().await).clone()),
                    prompt: Box::new(last_prompt_error.clone().into()),
                }).into());
            }
        };

        Box::pin(stream.instrument(agent_span))
    }
}

impl<M, P> IntoFuture for StreamingPromptRequest<M, P>
where
    M: CompletionModel + 'static,
    <M as CompletionModel>::StreamingResponse: WasmCompatSend,
    P: StreamingPromptHook<M> + 'static,
{
    type Output = StreamingResult<M::StreamingResponse>; // what `.await` returns
    type IntoFuture = WasmBoxedFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        // Wrap send() in a future, because send() returns a stream immediately
        Box::pin(async move { self.send().await })
    }
}

/// Helper function to stream a completion request to stdout.
pub async fn stream_to_stdout<R>(
    stream: &mut StreamingResult<R>,
) -> Result<FinalResponse, std::io::Error> {
    let mut final_res = FinalResponse::empty();
    print!("Response: ");
    while let Some(content) = stream.next().await {
        match content {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                Text { text },
            ))) => {
                print!("{text}");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(
                Reasoning { reasoning, .. },
            ))) => {
                let reasoning = reasoning.join("\n");
                print!("{reasoning}");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            Ok(MultiTurnStreamItem::FinalResponse(res)) => {
                final_res = res;
            }
            Err(err) => {
                eprintln!("Error: {err}");
            }
            _ => {}
        }
    }

    Ok(final_res)
}

// dead code allowed because of functions being left empty to allow for users to not have to implement every single function
/// Trait for per-request hooks to observe tool call events.
pub trait StreamingPromptHook<M>: Clone + Send + Sync
where
    M: CompletionModel,
{
    #[allow(unused_variables)]
    /// Called before the prompt is sent to the model
    fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    /// Called when receiving a text delta
    fn on_text_delta(
        &self,
        text_delta: &str,
        aggregated_text: &str,
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    /// Called when receiving a tool call delta.
    /// `tool_name` is Some on the first delta for a tool call, None on subsequent deltas.
    fn on_tool_call_delta(
        &self,
        tool_call_id: &str,
        tool_name: Option<&str>,
        tool_call_delta: &str,
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    /// Called after the model provider has finished streaming a text response from their completion API to the client.
    fn on_stream_completion_response_finish(
        &self,
        prompt: &Message,
        response: &<M as CompletionModel>::StreamingResponse,
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    /// Called before a tool is invoked.
    fn on_tool_call(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        args: &str,
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    /// Called after a tool is invoked (and a result has been returned).
    fn on_tool_result(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        args: &str,
        result: &str,
        cancel_sig: CancelSignal,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }
}

impl<M> StreamingPromptHook<M> for () where M: CompletionModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ProviderClient;
    use crate::client::completion::CompletionClient;
    use crate::providers::anthropic;
    use crate::streaming::StreamingPrompt;
    use futures::StreamExt;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::time::Duration;

    /// Background task that logs periodically to detect span leakage.
    /// If span leakage occurs, these logs will be prefixed with `invoke_agent{...}`.
    async fn background_logger(stop: Arc<AtomicBool>, leak_count: Arc<AtomicU32>) {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        let mut count = 0u32;

        while !stop.load(Ordering::Relaxed) {
            interval.tick().await;
            count += 1;

            tracing::event!(
                target: "background_logger",
                tracing::Level::INFO,
                count = count,
                "Background tick"
            );

            // Check if we're inside an unexpected span
            let current = tracing::Span::current();
            if !current.is_disabled() && !current.is_none() {
                leak_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        tracing::info!(target: "background_logger", total_ticks = count, "Background logger stopped");
    }

    /// Test that span context doesn't leak to concurrent tasks during streaming.
    ///
    /// This test verifies that using `.instrument()` instead of `span.enter()` in
    /// async_stream prevents thread-local span context from leaking to other tasks.
    ///
    /// Uses single-threaded runtime to force all tasks onto the same thread,
    /// making the span leak deterministic (it only occurs when tasks share a thread).
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "This requires an API key"]
    async fn test_span_context_isolation() {
        let stop = Arc::new(AtomicBool::new(false));
        let leak_count = Arc::new(AtomicU32::new(0));

        // Start background logger
        let bg_stop = stop.clone();
        let bg_leak = leak_count.clone();
        let bg_handle = tokio::spawn(async move {
            background_logger(bg_stop, bg_leak).await;
        });

        // Small delay to let background logger start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Make streaming request WITHOUT an outer span so rig creates its own invoke_agent span
        // (rig reuses current span if one exists, so we need to ensure there's no current span)
        let client = anthropic::Client::from_env();
        let agent = client
            .agent(anthropic::completion::CLAUDE_3_5_HAIKU)
            .preamble("You are a helpful assistant.")
            .temperature(0.1)
            .max_tokens(100)
            .build();

        let mut stream = agent
            .stream_prompt("Say 'hello world' and nothing else.")
            .await;

        let mut full_content = String::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                    text,
                ))) => {
                    full_content.push_str(&text.text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        tracing::info!("Got response: {:?}", full_content);

        // Stop background logger
        stop.store(true, Ordering::Relaxed);
        bg_handle.await.unwrap();

        let leaks = leak_count.load(Ordering::Relaxed);
        assert_eq!(
            leaks, 0,
            "SPAN LEAK DETECTED: Background logger was inside unexpected spans {leaks} times. \
             This indicates that span.enter() is being used inside async_stream instead of .instrument()"
        );
    }
}
