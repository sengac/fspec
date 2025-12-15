//! Gemini Provider implementation using rig
//!
//! Implements the LlmProvider trait for Google Gemini API communication.
//! Uses rig::providers::gemini for HTTP communication.

use crate::{CompletionResponse, LlmProvider, StopReason};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
use codelet_tools::ToolDefinition as OurToolDefinition;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::providers::gemini;
use tracing::warn;

/// Default Gemini model
const DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

/// Gemini 2.0 Flash context window size
pub const CONTEXT_WINDOW: usize = 1_000_000;

/// Gemini 2.0 Flash max output tokens
const MAX_OUTPUT_TOKENS: usize = 8192;

/// Gemini Provider for Google Generative AI API (using rig)
#[derive(Clone, Debug)]
pub struct GeminiProvider {
    completion_model: gemini::completion::CompletionModel,
    rig_client: gemini::Client,
    model_name: String,
}

impl GeminiProvider {
    /// Create a new GeminiProvider using API key from environment
    ///
    /// Checks for API key in GOOGLE_GENERATIVE_AI_API_KEY environment variable.
    /// Model can be overridden via GEMINI_MODEL environment variable.
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GOOGLE_GENERATIVE_AI_API_KEY")
            .map_err(|_| anyhow!("GOOGLE_GENERATIVE_AI_API_KEY environment variable not set"))?;

        // Allow model override via GEMINI_MODEL env var
        let model_name =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self::from_api_key(&api_key, &model_name)
    }

    /// Create a new GeminiProvider with an explicit API key and model
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Build rig gemini client
        let rig_client = gemini::Client::new(api_key)?;

        // Create completion model using the client
        let completion_model = rig_client.completion_model(model);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
        })
    }

    /// Get the configured rig gemini::Client
    pub fn client(&self) -> &gemini::Client {
        &self.rig_client
    }

    /// Create a rig Agent with all 7 tools configured for this provider
    pub fn create_rig_agent(&self) -> rig::agent::Agent<gemini::completion::CompletionModel> {
        use codelet_tools::{
            AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, ReadTool, WriteTool,
        };

        // Build agent with all 7 tools using rig's builder pattern
        self.rig_client
            .agent(&self.model_name)
            .max_tokens(MAX_OUTPUT_TOKENS as u64)
            .tool(ReadTool::new())
            .tool(WriteTool::new())
            .tool(EditTool::new())
            .tool(BashTool::new())
            .tool(GrepTool::new())
            .tool(GlobTool::new())
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
                    // Multi-turn conversation history handled by rig
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
        response: rig::completion::CompletionResponse<
            gemini::completion::gemini_api_types::GenerateContentResponse,
        >,
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

        // Map Gemini's finish_reason to our StopReason enum
        use gemini::completion::gemini_api_types::FinishReason;
        let stop_reason = match response.raw_response.candidates.first() {
            Some(candidate) => match &candidate.finish_reason {
                Some(FinishReason::Stop) => StopReason::EndTurn,
                Some(FinishReason::MaxTokens) => StopReason::MaxTokens,
                Some(FinishReason::Safety) => StopReason::EndTurn, // Treat safety stops as end turn
                Some(FinishReason::Recitation) => StopReason::EndTurn, // Treat recitation stops as end turn
                Some(FinishReason::FinishReasonUnspecified) => StopReason::EndTurn,
                Some(other) => {
                    warn!(finish_reason = ?other, "Unknown finish_reason from Gemini API");
                    StopReason::EndTurn
                }
                None => {
                    warn!("No finish_reason in Gemini candidate");
                    StopReason::EndTurn
                }
            },
            None => {
                warn!("No candidates in Gemini response");
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
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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
        false // Gemini does not support prompt caching yet
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
