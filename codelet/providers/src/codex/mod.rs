//! Codex Provider implementation using rig
//!
//! Implements the LlmProvider trait for Codex (ChatGPT backend API) communication.
//! Uses OAuth authentication via ~/.codex/auth.json or macOS keychain.

use crate::{CompletionResponse, LlmProvider, StopReason};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::completion::CompletionRequestBuilder;
use rig::providers::openai;
use tracing::warn;

pub mod codex_auth;

/// Default Codex model
const DEFAULT_MODEL: &str = "gpt-5.1-codex";

/// GPT-5.1 Codex context window size (272K tokens)
pub const CONTEXT_WINDOW: usize = 272_000;

/// GPT-5.1 Codex max output tokens (assumption: same as GPT-4)
const MAX_OUTPUT_TOKENS: usize = 4096;

/// Codex Provider for ChatGPT Backend API (using rig with OAuth)
#[derive(Clone)]
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
}

impl std::fmt::Debug for CodexProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexProvider")
            .field("model", &self.model_name)
            .finish()
    }
}

impl CodexProvider {
    /// Create a new CodexProvider using credentials from ~/.codex/auth.json
    ///
    /// Reads credentials from:
    /// 1. macOS keychain (if on macOS)
    /// 2. ~/.codex/auth.json file
    /// 3. $CODEX_HOME/auth.json if CODEX_HOME is set
    ///
    /// If cached OPENAI_API_KEY exists in auth.json, uses it directly.
    /// Otherwise, performs OAuth refresh and token exchange.
    ///
    /// Model can be overridden via CODEX_MODEL environment variable.
    pub fn new() -> Result<Self> {
        // Get API key from Codex auth (synchronous version using reqwest blocking)
        let api_key = codex_auth::get_codex_api_key_sync()?;

        // Allow model override via CODEX_MODEL env var
        let model_name = std::env::var("CODEX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new CodexProvider with an explicit API key and model
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Build rig completions client (standard OpenAI Chat Completions API)
        // Note: We can optionally use custom base URL for ChatGPT backend:
        // .base_url("https://chatgpt.com/backend-api/codex")
        // But for now, prefer standard OpenAI API via token exchange
        let rig_client = openai::CompletionsClient::builder()
            .api_key(api_key)
            .build()
            .map_err(|e| anyhow!("Failed to build Codex client: {e}"))?;

        // Create completion model using the client
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
        })
    }

    /// Get the configured rig openai::CompletionsClient
    pub fn client(&self) -> &openai::CompletionsClient {
        &self.rig_client
    }

    /// Create a rig Agent with all 8 tools configured for this provider
    ///
    /// # Arguments
    /// * `preamble` - Optional system prompt/preamble for the agent
    pub fn create_rig_agent(
        &self,
        preamble: Option<&str>,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, LsTool, ReadTool, WriteTool,
        };
        use rig::client::CompletionClient;

        // Build agent with all 8 tools using rig's builder pattern
        let mut agent_builder = self
            .rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new())
            .tool(WriteTool::new())
            .tool(EditTool::new())
            .tool(BashTool::new())
            .tool(GrepTool::new())
            .tool(GlobTool::new())
            .tool(LsTool::new())
            .tool(AstGrepTool::new());

        // Set preamble if provided
        if let Some(p) = preamble {
            agent_builder = agent_builder.preamble(p);
        }

        agent_builder.build()
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
                    // Multi-turn conversation history (including assistant messages) is handled by rig
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
                    for r in reasoning.reasoning {
                        content_parts.push(ContentPart::Text { text: r });
                    }
                }
                rig::completion::AssistantContent::Image(_) => {
                    return Err(anyhow!("Assistant images not supported"));
                }
            }
        }

        // Map OpenAI's finish_reason to our StopReason enum
        let stop_reason = match response.raw_response.choices.first() {
            Some(choice) => match choice.finish_reason.as_str() {
                "tool_calls" => StopReason::ToolUse,
                "length" => StopReason::MaxTokens,
                "stop" | "end_turn" => StopReason::EndTurn,
                other => {
                    warn!(finish_reason = %other, "Unknown finish_reason from Codex API");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No choices in Codex response");
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
impl LlmProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
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
        false // Codex does not support prompt caching
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
