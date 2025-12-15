//! Agent Execution bounded context
//!
//! Main runner loop, message history, streaming.

pub mod compaction;
pub mod rig_agent;

use anyhow::{anyhow, Result};
use codelet_providers::LlmProvider;
use codelet_tools::{ToolOutput, ToolRegistry};
pub use rig_agent::{RigAgent, DEFAULT_MAX_DEPTH};
use serde_json::Value;

// Re-export common types for convenience
pub use codelet_common::{ContentPart, Message, MessageContent, MessageRole};

/// Main agent runner - orchestrates LLM communication and tool execution
pub struct Runner {
    /// Message history
    messages: Vec<Message>,
    /// Tool registry
    tools: ToolRegistry,
    /// LLM provider for completions
    provider: Option<Box<dyn LlmProvider>>,
}

impl Runner {
    /// Create a new runner with default tools
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tools: ToolRegistry::default(),
            provider: None,
        }
    }

    /// Create a runner with a custom tool registry
    pub fn with_tools(tools: ToolRegistry) -> Self {
        Self {
            messages: Vec::new(),
            tools,
            provider: None,
        }
    }

    /// Create a runner with an LLM provider
    pub fn with_provider(provider: Box<dyn LlmProvider>) -> Self {
        Self {
            messages: Vec::new(),
            tools: ToolRegistry::default(),
            provider: Some(provider),
        }
    }

    /// Get message history
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Add a message to history
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get the tool registry
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Get mutable tool registry for registration
    pub fn tools_mut(&mut self) -> &mut ToolRegistry {
        &mut self.tools
    }

    /// Execute a tool by name
    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<ToolOutput> {
        self.tools.execute(name, args).await
    }

    /// List available tools
    pub fn available_tools(&self) -> Vec<&str> {
        self.tools.list()
    }

    /// Run the agent loop with user input
    ///
    /// This method orchestrates the conversation with the LLM provider:
    /// 1. Adds user message to history
    /// 2. Calls provider.complete() with current messages
    /// 3. Parses tool_use blocks from response
    /// 4. Executes tools and injects results
    /// 5. Continues loop until no tool calls (end_turn)
    pub async fn run(&mut self, user_input: &str) -> Result<Vec<Message>> {
        let Some(ref provider) = self.provider else {
            return Err(anyhow!("No provider configured"));
        };

        // Add user message
        self.messages.push(Message::user(user_input));

        // Agent loop
        loop {
            // Get tool definitions
            let tool_defs = self.tools.definitions();

            // Call LLM with tool definitions
            let response = provider
                .complete_with_tools(&self.messages, &tool_defs)
                .await?;

            // Add assistant response to history
            self.messages.push(Message {
                role: MessageRole::Assistant,
                content: response.content.clone(),
            });

            // Extract tool calls from response
            let tool_calls: Vec<(String, String, Value)> = match &response.content {
                MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|part| {
                        if let ContentPart::ToolUse { id, name, input } = part {
                            Some((id.clone(), name.clone(), input.clone()))
                        } else {
                            None
                        }
                    })
                    .collect(),
                MessageContent::Text(_) => Vec::new(),
            };

            // If no tool calls, exit loop
            if tool_calls.is_empty() {
                break;
            }

            // Execute tools and inject results
            for (tool_id, tool_name, input) in tool_calls {
                let result = self.tools.execute(&tool_name, input).await;

                let tool_result = match result {
                    Ok(output) => ContentPart::ToolResult {
                        tool_use_id: tool_id,
                        content: output.content,
                        is_error: output.is_error,
                    },
                    Err(e) => ContentPart::ToolResult {
                        tool_use_id: tool_id,
                        content: e.to_string(),
                        is_error: true,
                    },
                };

                // Add tool result as user message
                self.messages.push(Message {
                    role: MessageRole::User,
                    content: MessageContent::Parts(vec![tool_result]),
                });
            }
        }

        Ok(self.messages.clone())
    }
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}
