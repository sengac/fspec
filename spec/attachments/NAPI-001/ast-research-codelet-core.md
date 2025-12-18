# AST Research: Codelet Core Components

## Session (codelet/cli/src/session/mod.rs)

Key functions to wrap:
- `new(provider_name: Option<&str>)` - Creates session with optional provider
- `current_provider_name(&self)` - Returns current provider name
- `switch_provider(&mut self, provider_name: &str)` - Switches provider, clears context
- `provider_manager(&self)` - Returns reference to ProviderManager
- `inject_context_reminders(&mut self)` - Injects CLAUDE.md and environment info
- `compact_messages<F, Fut>(&mut self, llm_prompt: F)` - Compacts context

Key fields:
- `messages: Vec<rig::message::Message>` - Conversation history
- `turns: Vec<ConversationTurn>` - Turn tracking for compaction
- `token_tracker: TokenTracker` - Token usage tracking

## ProviderManager (codelet/providers/src/manager.rs)

Key functions:
- `new()` - Detects credentials, selects default provider (Claude > Gemini > Codex > OpenAI)
- `with_provider(provider_name: &str)` - Creates with specific provider
- `current_provider_name(&self)` - Returns current provider name
- `list_available_providers(&self)` - Lists available providers with credentials
- `switch_provider(&mut self, provider_name: &str)` - Switches to different provider
- `context_window(&self)` - Returns context window size
- `has_any_provider(&self)` - Checks if any provider is configured

Provider types: Claude, OpenAI, Codex, Gemini

## RigAgent (codelet/core/src/rig_agent.rs)

Key functions:
- `new(agent: Agent<M>, max_depth: usize)` - Creates agent wrapper
- `prompt(&self, prompt: &str)` - Non-streaming prompt
- `prompt_streaming(&self, prompt: &str)` - Streaming without history
- `prompt_streaming_with_history(&self, prompt: &str, history: &mut [Message])` - Streaming with history
- `prompt_streaming_with_history_and_hook<P>(&self, prompt: &str, history: &mut [Message], hook: P)` - Full streaming with compaction hook

Stream item types:
- `StreamedAssistantContent::Text` - Text chunks
- `StreamedAssistantContent::ToolCall` - Tool call info
- `StreamedUserContent::ToolResult` - Tool execution result
- `FinalResponse` - Completion with token usage

## NAPI-RS Integration Strategy

1. Create `CodeletSession` class that wraps `codelet_cli::session::Session`
2. Expose getters: `currentProviderName`, `availableProviders`, `tokenTracker`, `messages`
3. Expose methods: `switchProvider(name)`, `clearHistory()`
4. Create `prompt(input, callback)` async function using `ThreadsafeFunction<StreamChunk>`
5. Stream chunks: text, tool_call, tool_result, done, error
6. Use existing agent building from `codelet_cli::interactive::agent_runner`
