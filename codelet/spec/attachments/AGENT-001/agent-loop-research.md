# AGENT-001: Basic Agent Execution Loop - Research Document

## Executive Summary

This document outlines the implementation plan for the core agent execution loop in codelet, based on analysis of the codelet TypeScript implementation (runner.ts) and the current codelet Rust codebase.

## Gap Analysis

### Current State (codelet)

**src/agent/mod.rs (151 lines):**
- ✅ Message structs (MessageRole, MessageContent, ContentPart) - DONE
- ✅ Basic Runner struct with messages and tools - DONE
- ✅ Tool execution method `execute_tool(name, args)` - DONE
- ❌ **NO agent loop** - Runner is passive message container
- ❌ **NO LlmProvider integration** - Runner doesn't take/use provider
- ❌ **NO tool call parsing** - doesn't parse tool_use from responses
- ❌ **NO tool result injection** - doesn't inject results back

### Target State (codelet runner.ts)

**Key features from runner.ts (1259 lines):**

1. **Agent Loop Pattern (lines 796-1217)**:
   ```typescript
   let continueLoop = true;
   while (continueLoop) {
     // Call LLM
     const result = streamText({ model, messages, tools, ... });

     // Process streaming response
     for await (const part of result.fullStream) {
       switch (part.type) {
         case 'text-delta': // output text
         case 'tool-call': // collect tool calls
         case 'tool-result': // collect results
         case 'finish': // get finish reason
       }
     }

     // Check loop continuation
     if (finishReason === 'tool-calls') {
       // Continue - tools were called
     } else {
       continueLoop = false; // Exit on end_turn/stop
     }
   }
   ```

2. **Tool Call Detection (lines 231-242)**:
   ```typescript
   function hasToolCall(msgContent: CoreMessage['content']): boolean {
     return msgContent.some(part =>
       part.type === 'tool-call'
     );
   }
   ```

3. **Tool Execution via Registry**:
   - Tools defined in `tool-definitions.ts`
   - Execution happens during streaming via Vercel AI SDK

## Implementation Plan

### Phase 1: Extend LlmProvider Trait

Add tool support to the existing `LlmProvider` trait:

```rust
// In src/providers/mod.rs
#[async_trait]
pub trait LlmProvider: Send + Sync {
    // Existing methods...

    /// Complete with tool definitions
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse>;
}

pub struct CompletionResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
}

pub enum ContentBlock {
    Text(String),
    ToolUse { id: String, name: String, input: Value },
}

pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
}
```

### Phase 2: Add ToolRegistry.definitions()

Add tool definition export to ToolRegistry:

```rust
// In src/tools/mod.rs
impl ToolRegistry {
    /// Get tool definitions for API request
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.parameters(),
            })
            .collect()
    }
}

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### Phase 3: Implement Runner.run()

Add the core agent loop to Runner:

```rust
// In src/agent/mod.rs
impl Runner {
    /// Create runner with provider
    pub fn with_provider<P: LlmProvider + 'static>(provider: P) -> Self {
        Self {
            messages: Vec::new(),
            tools: ToolRegistry::default(),
            provider: Some(Box::new(provider)),
        }
    }

    /// Run the agent loop with user input
    pub async fn run(&mut self, user_input: &str) -> Result<Vec<Message>> {
        // Add user message
        self.add_message(Message::user(user_input));

        loop {
            // Get tool definitions
            let tool_defs = self.tools.definitions();

            // Call LLM
            let response = self.provider
                .as_ref()
                .ok_or_else(|| anyhow!("No provider configured"))?
                .complete_with_tools(&self.messages, &tool_defs)
                .await?;

            // Add assistant response to history
            self.add_message(Message::from_response(&response));

            // Check for tool calls
            let tool_calls = response.content.iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } =>
                        Some((id.clone(), name.clone(), input.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>();

            if tool_calls.is_empty() {
                // No tool calls - exit loop
                break;
            }

            // Execute tools and inject results
            for (id, name, input) in tool_calls {
                let result = self.tools.execute(&name, input).await;
                let tool_result = match result {
                    Ok(output) => ContentPart::ToolResult {
                        tool_use_id: id,
                        content: output.content,
                        is_error: false,
                    },
                    Err(e) => ContentPart::ToolResult {
                        tool_use_id: id,
                        content: e.to_string(),
                        is_error: true,
                    },
                };

                // Add tool result as user message
                self.add_message(Message {
                    role: MessageRole::User,
                    content: MessageContent::Parts(vec![tool_result]),
                });
            }
        }

        Ok(self.messages.clone())
    }
}
```

### Phase 4: Update ClaudeProvider

Implement `complete_with_tools` for ClaudeProvider:

```rust
// In src/providers/claude.rs
impl LlmProvider for ClaudeProvider {
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse> {
        let request = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_OUTPUT_TOKENS,
            "messages": format_messages(messages),
            "tools": tools.iter().map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            }).collect::<Vec<_>>(),
        });

        let response = self.client
            .post(API_URL)
            .headers(...)
            .json(&request)
            .send()
            .await?;

        // Parse response into CompletionResponse
        // ...
    }
}
```

## Story Point Estimate: 8

Breakdown:
- Phase 1 (Extend LlmProvider): 1 point
- Phase 2 (ToolRegistry.definitions): 1 point
- Phase 3 (Runner.run loop): 4 points
- Phase 4 (ClaudeProvider update): 2 points

## Testing Strategy

1. **Unit Tests with Mock Provider**:
   - Create MockLlmProvider that returns controlled responses
   - Test single tool call iteration
   - Test multiple tool calls
   - Test error handling

2. **Integration Tests**:
   - Test full loop with real ClaudeProvider (requires API key)
   - Test actual tool execution with Read/Write/Bash tools

## Dependencies

- Existing: `reqwest`, `serde`, `async-trait`, `tokio`
- No new dependencies required

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API format mismatch | Medium | High | Careful testing with real API |
| Tool result format | Low | Medium | Match Anthropic spec exactly |
| Async complexity | Low | Low | Use proven tokio patterns |

## References

- Anthropic Messages API: https://docs.anthropic.com/claude/reference/messages
- codelet runner.ts: ~/projects/codelet/src/agent/runner.ts
- Current codelet: ~/projects/codelet/src/agent/mod.rs
