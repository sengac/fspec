//! OpenAI Provider implementation using rig
//!
//! Implements the LlmProvider trait for OpenAI API communication.
//! Uses rig::providers::openai for HTTP communication.

use crate::{CompletionResponse, LlmProvider, StopReason};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

/// Default OpenAI model
const DEFAULT_MODEL: &str = "gpt-4-turbo";

/// GPT-4 Turbo context window size
pub const CONTEXT_WINDOW: usize = 128_000;

/// GPT-4 Turbo max output tokens
const MAX_OUTPUT_TOKENS: usize = 4096;

/// OpenAI Provider for OpenAI API (using rig)
#[derive(Clone)]
pub struct OpenAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
}

impl std::fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("model", &self.model_name)
            .finish()
    }
}

impl OpenAIProvider {
    /// Create a new OpenAIProvider using API key from environment
    ///
    /// Checks for API key in OPENAI_API_KEY environment variable.
    /// Model can be overridden via OPENAI_MODEL environment variable.
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY environment variable not set"))?;

        // Allow model override via OPENAI_MODEL env var
        let model_name =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new OpenAIProvider with an explicit API key and model
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Build rig completions client (standard OpenAI Chat Completions API)
        let rig_client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| anyhow!("Failed to build OpenAI client: {e}"))?;

        // Create completion model using the client
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
        })
    }

    /// Get the configured rig openai::CompletionsClient
    ///
    /// This client is fully configured with:
    /// - API key authentication
    ///
    /// Use this client to create rig agents with tools.
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 7 tools configured for this provider
    ///
    /// This method encapsulates all OpenAI-specific configuration:
    /// - Model name (gpt-4-turbo or custom)
    /// - Max tokens (4096)
    /// - All 7 tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep)
    ///
    /// Returns a fully configured rig::agent::Agent ready for use with RigAgent.
    pub fn create_rig_agent(&self) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WriteTool,
        };
        use rig::client::CompletionClient;

        // Build agent with all 8 tools using rig's builder pattern
        self.rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new())
            .tool(WriteTool::new())
            .tool(EditTool::new())
            .tool(BashTool::new())
            .tool(GrepTool::new())
            .tool(GlobTool::new())
            .tool(LsTool::new())
            .tool(AstGrepTool::new())
            .build()
    }

    /// Extract text content from a message (DRY helper)
    fn extract_text_from_content(content: &MessageContent) -> String {
        match content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| {
                    if let ContentPart::Text { text } = p {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Extract preamble and last user prompt from messages
    fn extract_prompt_data(&self, messages: &[Message]) -> (Option<String>, String) {
        let mut system_prompt: Option<String> = None;
        let mut user_messages: Vec<String> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    let text = Self::extract_text_from_content(&msg.content);
                    system_prompt = Some(text);
                }
                MessageRole::User => {
                    let text = Self::extract_text_from_content(&msg.content);
                    user_messages.push(text);
                }
                MessageRole::Assistant => {
                    // Multi-turn conversation history (including assistant messages) is out of
                    // scope for PROV-003. Multi-turn support will be added in REFAC-004 using
                    // rig::agent::Agent which handles conversation history automatically.
                    // For now, we only extract the last user message for single-turn completions.
                }
            }
        }

        let prompt = if user_messages.is_empty() {
            String::new()
        } else {
            user_messages.join("\n\n")
        };

        (system_prompt, prompt)
    }

    /// Convert rig response to our CompletionResponse format
    fn rig_response_to_completion(
        &self,
        response: rig::completion::CompletionResponse<openai::completion::CompletionResponse>,
    ) -> Result<CompletionResponse> {
        // Convert rig AssistantContent to our ContentPart format
        let mut content_parts: Vec<ContentPart> = vec![];

        for content in response.choice.into_iter() {
            match content {
                rig::completion::AssistantContent::Text(text) => {
                    content_parts.push(ContentPart::Text { text: text.text });
                }
                rig::completion::AssistantContent::ToolCall(tool_call) => {
                    content_parts.push(ContentPart::ToolUse {
                        id: tool_call.id,
                        name: tool_call.function.name,
                        input: tool_call.function.arguments,
                    });
                }
                rig::completion::AssistantContent::Reasoning(reasoning) => {
                    // Rig returns reasoning separately, we can include it as text
                    // or handle it specially if needed
                    for r in reasoning.reasoning {
                        content_parts.push(ContentPart::Text { text: r });
                    }
                }
                rig::completion::AssistantContent::Image(_) => {
                    // OpenAI doesn't support assistant images in chat completions
                    return Err(anyhow!("Assistant images not supported"));
                }
            }
        }

        // Map OpenAI's finish_reason to our StopReason enum
        // OpenAI CompletionResponse has choices[].finish_reason field
        let stop_reason = match response.raw_response.choices.first() {
            Some(choice) => match choice.finish_reason.as_str() {
                "tool_calls" => StopReason::ToolUse,
                "length" => StopReason::MaxTokens,
                "stop" | "end_turn" => StopReason::EndTurn,
                other => {
                    // Unknown stop reason - log and default to EndTurn
                    warn!(finish_reason = %other, "Unknown finish_reason from OpenAI API");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No choices in OpenAI response");
                StopReason::EndTurn
            }
        };

        Ok(CompletionResponse {
            content: MessageContent::Parts(content_parts),
            stop_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    fn context_window(&self) -> usize {
        CONTEXT_WINDOW
    }

    fn max_output_tokens(&self) -> usize {
        MAX_OUTPUT_TOKENS
    }

    fn supports_caching(&self) -> bool {
        false // OpenAI does not support prompt caching
    }

    fn supports_streaming(&self) -> bool {
        true // Streaming support via rig
    }

    async fn complete(&self, messages: &[Message]) -> Result<String> {
        // Reuse complete_with_tools with no tools to avoid code duplication
        let response = self.complete_with_tools(messages, &[]).await?;

        // Extract text using helper method to maintain DRY
        Ok(Self::extract_text_from_content(&response.content))
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[OurToolDefinition],
    ) -> Result<CompletionResponse> {
        // Extract prompt data
        let (preamble, prompt) = self.extract_prompt_data(messages);

        // Convert tools to rig format
        let rig_tools: Vec<rig::completion::ToolDefinition> = tools
            .iter()
            .map(|t| rig::completion::ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.clone(),
            })
            .collect();

        // Build and send completion request using rig's builder pattern
        let mut builder = CompletionRequestBuilder::new(self.completion_model.clone(), prompt)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tools(rig_tools);

        if let Some(preamble_text) = preamble {
            builder = builder.preamble(preamble_text);
        }

        // Send request and get response
        let response = builder
            .send()
            .await
            .map_err(|e| anyhow!("Rig completion with tools failed: {e}"))?;

        // Convert rig response to our CompletionResponse format
        self.rig_response_to_completion(response)
    }
}
