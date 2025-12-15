# Rig Refactoring Research - REFAC-001

## Executive Summary

**Critical Finding**: The current codelet implementation does NOT use rig-core properly and lacks streaming support. A comprehensive refactor of AGENT-001, PROV-001, and PROV-002 is required to:

1. ✅ **Use rig's unified provider abstraction** (CompletionModel trait)
2. ✅ **Implement streaming for both providers** (Claude and future OpenAI)
3. ✅ **Use rig's agent architecture** with multi-turn tool calling
4. ✅ **Leverage rig's tool system** for proper tool definitions and execution
5. ✅ **Support both streaming and non-streaming modes** in the agent loop

**Estimated Complexity**: 13-21 story points (Large Epic)
**Recommendation**: Break into multiple work units (REFAC-001a, REFAC-001b, REFAC-001c)

---

## Table of Contents

1. [What is Rig?](#what-is-rig)
2. [Current codelet Implementation Analysis](#current-codelet-implementation-analysis)
3. [Rig Architecture Deep Dive](#rig-architecture-deep-dive)
4. [Comparison: Current vs Rig](#comparison-current-vs-rig)
5. [What Needs to be Refactored](#what-needs-to-be-refactored)
6. [Migration Path](#migration-path)
7. [Breaking Changes](#breaking-changes)
8. [Implementation Plan](#implementation-plan)
9. [Testing Strategy](#testing-strategy)
10. [Risk Assessment](#risk-assessment)

---

## 1. What is Rig?

**Rig** (rig-core 0.25.0) is a Rust library for building scalable, modular, and ergonomic LLM-powered applications.

### Key Features

- ✅ **Unified provider abstraction** - 20+ LLM providers under one interface
- ✅ **Native streaming support** - Both text and tool calling stream seamlessly
- ✅ **Multi-turn tool calling** - Automatic tool execution loops with depth control
- ✅ **Agent workflows** - High-level agent abstraction with RAG, tools, and dynamic context
- ✅ **Type-safe tool definitions** - Derive macros for tool generation from Rust types
- ✅ **Agentic workflows** - Handles multi-turn streaming and prompting automatically
- ✅ **GenAI Semantic Convention compliance** - Full OpenTelemetry integration
- ✅ **WASM compatibility** - Core library works in browser environments

### Supported Providers (Built-in)

- Anthropic (Claude)
- OpenAI (GPT-4, GPT-3.5)
- Azure OpenAI
- Google Gemini
- Cohere
- Groq
- xAI (Grok)
- Perplexity
- Together AI
- Deepseek
- Mistral
- Ollama (local models)
- And 8 more...

### Architecture Principles

1. **CompletionModel trait** - Generic interface for all providers
2. **Agent** - High-level abstraction over models with tools, context, and streaming
3. **Tool trait** - Type-safe tool definitions with automatic JSON schema generation
4. **Streaming-first** - All operations support streaming by default
5. **Provider-agnostic** - Switch providers without changing application code

---

## 2. Current codelet Implementation Analysis

### What We Have Now

**File**: `src/providers/mod.rs` (lines 1-100)

```rust
pub trait LlmProvider: Send + Sync {
    fn complete(&self, messages: &[Message]) -> Result<String>;
    fn complete_with_tools(&self, messages: &[Message], tools: Vec<Tool>)
        -> Result<(String, Vec<ToolCall>)>;
    fn supports_streaming(&self) -> bool;
    fn supports_caching(&self) -> bool;
}
```

**Problems**:

1. ❌ **No actual streaming implementation** - `supports_streaming()` returns `false`
2. ❌ **Blocking API** - `complete()` is synchronous, not async
3. ❌ **No rig integration** - Custom trait instead of using rig's `CompletionModel`
4. ❌ **Manual tool execution** - Agent loop manually parses and executes tools
5. ❌ **Provider-specific logic** - Each provider has custom message formatting
6. ❌ **No multi-turn depth control** - Agent loop has no configurable depth limit
7. ❌ **No telemetry** - No OpenTelemetry support

### ClaudeProvider (PROV-001)

**File**: `src/providers/claude.rs` (lines 1-312)

**Current Implementation**:
```rust
impl LlmProvider for ClaudeProvider {
    fn complete(&self, messages: &[Message]) -> Result<String> {
        // Synchronous HTTP request
        let response = self.http_client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request_body)
            .send()?;  // Blocking!
    }

    fn supports_streaming(&self) -> bool {
        false  // TODO: Implement streaming
    }
}
```

**Issues**:
- ❌ No streaming support (returns false)
- ❌ Synchronous API (blocking)
- ❌ Manual message formatting (not using rig's conversion)
- ❌ No tool call delta support
- ❌ No extended thinking support

### Agent Runner (AGENT-001)

**File**: `src/agent/mod.rs` (lines 91-250)

**Current Implementation**:
```rust
pub async fn run(&mut self, user_input: String) -> Result<Vec<Message>> {
    loop {
        let response = self.provider.complete_with_tools(&self.messages, tools)?;

        // Parse tool calls manually
        let tool_calls = parse_tool_calls(&response)?;

        if tool_calls.is_empty() {
            return Ok(self.messages);
        }

        // Execute tools manually
        for tool_call in tool_calls {
            let result = self.tools.execute(&tool_call.name, tool_call.args)?;
            self.messages.push(Message::user(result));
        }
    }
}
```

**Issues**:
- ❌ No streaming support
- ❌ No depth control for tool calling loops
- ❌ Manual tool call parsing
- ❌ No concurrent tool execution
- ❌ No pause/cancel support during streaming
- ❌ No hooks for monitoring tool execution

---

## 3. Rig Architecture Deep Dive

### 3.1 CompletionModel Trait

**Location**: `/tmp/rig/rig-core/src/completion/request.rs:109-145`

```rust
pub trait CompletionModel: Clone + WasmCompatSend + WasmCompatSync {
    /// Provider-specific response type (e.g., OpenAI vs Claude format)
    type Response: WasmCompatSend + WasmCompatSync + Serialize + DeserializeOwned;

    /// Provider-specific streaming response
    type StreamingResponse: Clone + Unpin + WasmCompatSend + WasmCompatSync
        + Serialize + DeserializeOwned + GetTokenUsage;

    /// The client type that creates this model
    type Client;

    /// Create a model from a client
    fn make(client: &Self::Client, model: impl Into<String>) -> Self;

    /// Non-streaming completion
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionResponse<Self::Response>, CompletionError>>;

    /// Streaming completion
    fn stream(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>>;
}
```

**Key Design Decisions**:

1. **Associated types** - Each provider defines its own response types
2. **Async by default** - All methods return futures
3. **Generic CompletionRequest** - Unified request format across providers
4. **Streaming via separate method** - Clean separation of streaming vs non-streaming

### 3.2 StreamingCompletionResponse

**Location**: `/tmp/rig/rig-core/src/streaming.rs:87-142`

```rust
pub struct StreamingCompletionResponse<R> {
    inner: Abortable<StreamingResult<R>>,
    abort_handle: AbortHandle,
    pause_control: PauseControl,
    text: String,                      // Accumulated text
    reasoning: String,                 // Accumulated reasoning (extended thinking)
    tool_calls: Vec<ToolCall>,        // Collected tool calls
    choice: OneOrMany<AssistantContent>,
    response: Option<R>,              // Final response with usage
}

impl<R> StreamingCompletionResponse<R> {
    pub fn pause(&self);
    pub fn resume(&self);
    pub fn cancel(&self);
    pub fn is_paused(&self) -> bool;
    pub fn is_done(&self) -> bool;
    pub fn text(&self) -> &str;
    pub fn reasoning(&self) -> &str;
    pub fn tool_calls(&self) -> &[ToolCall];
}
```

**Capabilities**:
- ✅ Pause/resume streaming
- ✅ Cancel streaming mid-flight
- ✅ Access accumulated state at any time
- ✅ Separate reasoning from text (extended thinking)
- ✅ Tool call deltas aggregated automatically

### 3.3 Agent Architecture

**Location**: `/tmp/rig/rig-core/src/agent/completion.rs:32-96`

```rust
pub struct Agent<M: CompletionModel> {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Arc<M>,
    pub preamble: Option<String>,           // System prompt
    pub static_context: Vec<Document>,      // Always-included docs
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub additional_params: Option<serde_json::Value>,
    pub tool_server_handle: ToolServerHandle,  // Manages all tools
    pub dynamic_context: DynamicContextStore,   // RAG vector stores
    pub tool_choice: Option<ToolChoice>,
}

impl<M: CompletionModel> Agent<M> {
    /// Builder pattern
    pub fn builder(model: M) -> AgentBuilder<M>;

    /// Non-streaming prompt
    pub async fn prompt(&self, prompt: impl Into<String>) -> Result<String>;

    /// Streaming prompt
    pub async fn stream_prompt(&self, prompt: impl Into<String>)
        -> Result<StreamingCompletionResponse<M::StreamingResponse>>;

    /// Multi-turn with depth control
    pub fn prompt_with_depth(&self, prompt: impl Into<String>, depth: usize)
        -> PromptRequest<'_, M, ()>;
}
```

**Multi-Turn Tool Calling**:

**Location**: `/tmp/rig/rig-core/src/agent/prompt_request/mod.rs:41-79`

```rust
pub struct PromptRequest<'a, M, H> {
    prompt: Message,
    chat_history: Option<&'a mut Vec<Message>>,
    max_depth: usize,              // Max tool-calling rounds
    agent: &'a Agent<M>,
    hook: Option<H>,               // Event hooks
}

impl<M: CompletionModel, H> PromptRequest<'_, M, H> {
    pub fn multi_turn(self, depth: usize) -> Self {
        Self { max_depth: depth, ..self }
    }

    pub async fn send(self) -> Result<String> {
        let mut depth = 0;
        loop {
            let resp = self.agent.completion(/* ... */).await?.send().await?;

            // Partition response into tool calls and text
            let (tool_calls, texts): (Vec<_>, Vec<_>) = resp.choice.iter()
                .partition(|c| matches!(c, AssistantContent::ToolCall(_)));

            if tool_calls.is_empty() || depth >= self.max_depth {
                return Ok(texts.join("\n"));
            }

            // Execute tools in parallel
            let results = stream::iter(tool_calls)
                .then(|tc| self.agent.tool_server_handle.call_tool(/* ... */))
                .collect().await;

            // Add results to history
            self.chat_history.push(Message::User { content: results });
            depth += 1;
        }
    }
}
```

**Streaming Multi-Turn**:

**Location**: `/tmp/rig/rig-core/src/agent/prompt_request/streaming.rs:152-298`

```rust
pub async fn send(self) -> Result<impl Stream<Item = Result<MultiTurnStreamItem<M::StreamingResponse>>>> {
    let stream = stream! {
        let mut depth = 0;
        loop {
            // Stream the LLM response
            let mut resp_stream = self.agent.stream(/* ... */).await?;

            while let Some(item) = resp_stream.next().await {
                match item {
                    StreamedAssistantContent::Text(t) =>
                        yield Ok(MultiTurnStreamItem::StreamAssistantItem(item)),
                    StreamedAssistantContent::ToolCall(tc) => {
                        yield Ok(MultiTurnStreamItem::StreamAssistantItem(item));
                        // Tool call will be executed after stream completes
                    }
                    _ => yield Ok(MultiTurnStreamItem::StreamAssistantItem(item)),
                }
            }

            let final_resp = resp_stream.await?;
            yield Ok(MultiTurnStreamItem::FinalResponse(final_resp.response));

            // Check for tool calls
            let tool_calls = final_resp.tool_calls();
            if tool_calls.is_empty() || depth >= self.max_depth {
                break;
            }

            // Execute tools and stream results
            for tool_call in tool_calls {
                let result = self.agent.tool_server_handle.call_tool(/* ... */).await?;
                yield Ok(MultiTurnStreamItem::StreamUserItem(
                    StreamedUserContent::ToolResult(result)
                ));
            }

            depth += 1;
        }
    };

    Ok(Box::pin(stream))
}
```

### 3.4 Tool System

**Location**: `/tmp/rig/rig-core/src/tool/mod.rs:32-94`

```rust
pub trait Tool: Sized + WasmCompatSend + WasmCompatSync {
    const NAME: &'static str;

    type Error: std::error::Error + Send + Sync + 'static;
    type Args: for<'a> Deserialize<'a> + Send + Sync;
    type Output: Serialize;

    /// Return tool definition for LLM
    fn definition(&self, prompt: String)
        -> impl Future<Output = ToolDefinition> + Send;

    /// Execute the tool
    fn call(&self, args: Self::Args)
        -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
```

**Example: ReadTool**

```rust
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ReadArgs {
    file_path: String,
}

struct ReadTool;

impl Tool for ReadTool {
    const NAME: &'static str = "read_file";
    type Error = std::io::Error;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file"
                    }
                },
                "required": ["file_path"]
            })
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tokio::fs::read_to_string(&args.file_path).await
    }
}
```

**ToolSet for Dynamic Dispatch**:

```rust
let mut toolset = ToolSet::new();
toolset.add_tool(ReadTool);
toolset.add_tool(WriteTool);
toolset.add_tool(BashTool);

// Agent can now use any tool dynamically
let agent = client.agent("gpt-4")
    .tools(&toolset)
    .build();
```

### 3.5 Provider Implementations

#### Anthropic Provider

**Location**: `/tmp/rig/rig-core/src/providers/anthropic/streaming.rs:212-404`

**Key Features**:
- ✅ SSE streaming with distinct event types
- ✅ Extended thinking support (`ContentDelta::ThinkingDelta`)
- ✅ Tool call deltas accumulated per content block
- ✅ Prompt caching headers
- ✅ PDF document support

**Streaming Flow**:
```rust
while let Some(event) = sse_stream.next().await {
    match event {
        StreamingEvent::ContentBlockStart { content_block: ToolUse { id, name } } => {
            current_tool_call = Some(ToolCallState { id, name, input_json: "" });
        }
        StreamingEvent::ContentBlockDelta { delta: InputJsonDelta { partial_json } } => {
            current_tool_call.input_json.push_str(partial_json);
            yield Ok(RawStreamingChoice::ToolCallDelta {
                id: current_tool_call.id.clone(),
                delta: partial_json
            });
        }
        StreamingEvent::ContentBlockStop => {
            if let Some(tc) = current_tool_call.take() {
                let args = serde_json::from_str(&tc.input_json)?;
                yield Ok(RawStreamingChoice::ToolCall {
                    id: tc.id, name: tc.name, arguments: args, call_id: None
                });
            }
        }
        // ... handle text, thinking, and final response
    }
}
```

#### OpenAI Provider

**Location**: `/tmp/rig/rig-core/src/providers/openai/completion/streaming.rs:143-287`

**Key Features**:
- ✅ SSE streaming with delta accumulation
- ✅ Tool calls indexed by position
- ✅ Incremental JSON parsing
- ✅ Audio input support
- ✅ Refusal detection

**Streaming Flow**:
```rust
let mut tool_calls: HashMap<usize, ToolCall> = HashMap::new();

while let Some(event) = sse_stream.next().await {
    for tool_call in &delta.tool_calls {
        let existing = tool_calls.entry(tool_call.index).or_insert_with(default);

        if let Some(id) = &tool_call.id { existing.id = id.clone(); }
        if let Some(name) = &tool_call.function.name { existing.function.name = name.clone(); }

        if let Some(chunk) = &tool_call.function.arguments {
            existing.function.arguments.push_str(chunk);

            // Try parsing when it looks complete
            if existing.function.arguments.starts_with('{') && existing.function.arguments.ends_with('}') {
                match serde_json::from_str(&existing.function.arguments) {
                    Ok(json) => existing.function.arguments = json,
                    Err(_) => {} // Keep accumulating
                }
            }

            yield Ok(RawStreamingChoice::ToolCallDelta { id: existing.id.clone(), delta: chunk });
        }
    }

    if finish_reason == FinishReason::ToolCalls {
        for (_, tc) in tool_calls.drain() {
            yield Ok(RawStreamingChoice::ToolCall {
                id: tc.id, name: tc.function.name, arguments: tc.function.arguments, call_id: None
            });
        }
    }
}
```

---

## 4. Comparison: Current vs Rig

| Feature | Current codelet | Rig |
|---------|-------------------|-----|
| **Streaming** | ❌ Not implemented (returns false) | ✅ Full streaming with pause/resume/cancel |
| **API Style** | ❌ Synchronous (blocking) | ✅ Async/await throughout |
| **Provider Abstraction** | ⚠️ Custom `LlmProvider` trait | ✅ Generic `CompletionModel` trait with 20+ providers |
| **Tool Calling** | ⚠️ Manual parsing in agent loop | ✅ Automatic multi-turn with depth control |
| **Tool Definitions** | ⚠️ Manual JSON schema creation | ✅ Type-safe trait with derive macros |
| **Concurrent Tools** | ❌ Sequential execution | ✅ Parallel execution via streams |
| **Message Format** | ⚠️ Custom types per provider | ✅ Unified `message::Message` type |
| **Extended Thinking** | ❌ Not supported | ✅ Native support (Claude) |
| **RAG Support** | ❌ Not implemented | ✅ Built-in vector store integration |
| **Telemetry** | ❌ No OpenTelemetry | ✅ Full GenAI Semantic Convention |
| **WASM** | ❌ Not supported | ✅ Full WASM compatibility |
| **Hooks** | ❌ No event hooks | ✅ Custom hooks for monitoring |

---

## 5. What Needs to be Refactored

### 5.1 AGENT-001: Basic Agent Execution Loop

**Current File**: `src/agent/mod.rs`

**Required Changes**:

1. **Replace Runner with rig::agent::Agent**
   ```rust
   // OLD
   pub struct Runner {
       messages: Vec<Message>,
       tools: ToolRegistry,
       provider: Option<Box<dyn LlmProvider>>,
   }

   // NEW
   pub struct Runner<M: CompletionModel> {
       agent: rig::agent::Agent<M>,
   }
   ```

2. **Add streaming support**
   ```rust
   impl<M: CompletionModel> Runner<M> {
       pub async fn run(&mut self, input: String) -> Result<String> {
           self.agent.prompt(input).await
       }

       pub async fn run_streaming(&mut self, input: String)
           -> Result<StreamingCompletionResponse<M::StreamingResponse>>
       {
           self.agent.stream_prompt(input).await
       }
   }
   ```

3. **Multi-turn tool calling with depth control**
   ```rust
   pub async fn run_multi_turn(&mut self, input: String, max_depth: usize)
       -> Result<String>
   {
       self.agent.prompt_with_depth(input, max_depth)
           .multi_turn(max_depth)
           .send()
           .await
   }
   ```

4. **Remove manual tool call parsing** - Rig handles this automatically

5. **Add pause/cancel support for streaming**
   ```rust
   pub fn pause_streaming(&self) {
       if let Some(stream) = &self.current_stream {
           stream.pause();
       }
   }
   ```

### 5.2 PROV-001: Anthropic Claude Provider

**Current File**: `src/providers/claude.rs`

**Required Changes**:

1. **Implement rig::completion::CompletionModel**
   ```rust
   use rig::completion::{CompletionModel, CompletionRequest, CompletionResponse};
   use rig::streaming::StreamingCompletionResponse;

   pub type ClaudeProvider = rig::providers::anthropic::CompletionModel;

   // Or wrap it if we need custom behavior:
   pub struct ClaudeProviderWrapper {
       inner: rig::providers::anthropic::CompletionModel,
   }

   impl CompletionModel for ClaudeProviderWrapper {
       type Response = rig::providers::anthropic::CompletionResponse;
       type StreamingResponse = rig::providers::anthropic::StreamingCompletionResponse;
       type Client = rig::providers::anthropic::Client;

       fn make(client: &Self::Client, model: impl Into<String>) -> Self {
           Self {
               inner: rig::providers::anthropic::CompletionModel::make(client, model)
           }
       }

       async fn completion(&self, req: CompletionRequest)
           -> Result<CompletionResponse<Self::Response>, CompletionError>
       {
           self.inner.completion(req).await
       }

       async fn stream(&self, req: CompletionRequest)
           -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
       {
           self.inner.stream(req).await
       }
   }
   ```

2. **Remove custom HTTP client code** - Rig handles this

3. **Remove custom message formatting** - Use rig's conversions

4. **Add OAuth support** (if not in rig)
   ```rust
   impl rig::providers::anthropic::ClientBuilder {
       pub fn with_oauth_token(mut self, token: String) -> Self {
           self.headers.insert("authorization", format!("Bearer {}", token));
           self
       }
   }
   ```

5. **Use rig's streaming infrastructure**
   - No need to implement SSE parsing
   - No need to handle tool call deltas
   - Extended thinking comes for free

### 5.3 PROV-002: Claude Code OAuth Authentication

**Current File**: `src/providers/claude.rs` (OAuth section)

**Required Changes**:

1. **Extend rig's Anthropic client**
   ```rust
   pub fn create_claude_oauth_provider() -> Result<ClaudeProvider> {
       let token = std::env::var("CLAUDE_CODE_OAUTH_TOKEN")?;

       let client = rig::providers::anthropic::ClientBuilder::default()
           .header("authorization", format!("Bearer {}", token))
           .header("anthropic-beta", "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14")
           .build()?;

       Ok(client.agent("claude-sonnet-4-20250514").build())
   }
   ```

2. **Keep OAuth token refresh logic** (external to rig)

3. **Use rig's prompt caching headers** (already supported)

### 5.4 Tool System Refactoring

**Current File**: `src/tools/mod.rs`

**Required Changes**:

1. **Implement rig::tool::Tool for each tool**
   ```rust
   // OLD
   pub trait Tool {
       fn name(&self) -> &str;
       fn description(&self) -> &str;
       fn parameters(&self) -> serde_json::Value;
       fn execute(&self, args: serde_json::Value) -> Result<String>;
   }

   // NEW
   use rig::tool::Tool;

   #[derive(Deserialize)]
   struct ReadArgs {
       file_path: String,
       offset: Option<usize>,
       limit: Option<usize>,
   }

   struct ReadTool;

   impl Tool for ReadTool {
       const NAME: &'static str = "read_file";
       type Error = std::io::Error;
       type Args = ReadArgs;
       type Output = String;

       async fn definition(&self, _prompt: String) -> ToolDefinition {
           // Return JSON schema
       }

       async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
           // Execute tool logic
       }
   }
   ```

2. **Use ToolSet for dynamic dispatch**
   ```rust
   let mut toolset = ToolSet::new();
   toolset.add_tool(ReadTool);
   toolset.add_tool(WriteTool);
   toolset.add_tool(EditTool);
   // ... add all tools

   let agent = client.agent("claude-sonnet-4")
       .tools(&toolset)
       .build();
   ```

3. **Keep existing tool implementations** - Just wrap them in rig's Tool trait

---

## 6. Migration Path

### Phase 1: Add rig-core Dependency (REFAC-001a)

**Estimated**: 3 story points

**Tasks**:
1. Add rig-core to Cargo.toml
2. Update existing code to compile with rig types visible
3. Create compatibility shims (old API → rig types)
4. All tests still pass using old implementation

**Deliverable**: Rig is available but not yet used

### Phase 2: Refactor Providers (REFAC-001b)

**Estimated**: 8 story points

**Tasks**:
1. Refactor ClaudeProvider to use rig::providers::anthropic
2. Implement streaming for Claude
3. Add OAuth support to rig's Anthropic provider (if needed)
4. Update all provider tests to use new implementation
5. Keep old API for backwards compatibility

**Deliverable**: Claude provider uses rig with streaming

### Phase 3: Refactor Agent Loop (REFAC-001c)

**Estimated**: 8 story points

**Tasks**:
1. Replace Runner with rig::agent::Agent
2. Implement streaming agent loop
3. Add multi-turn depth control
4. Refactor tool system to use rig::tool::Tool
5. Update agent tests
6. Add streaming integration tests

**Deliverable**: Agent uses rig with streaming and multi-turn

### Phase 4: Add OpenAI Provider (PROV-003 - Unblocked)

**Estimated**: 5 story points (much easier with rig!)

**Tasks**:
1. Use rig::providers::openai::Client
2. Create OpenAIProvider wrapper if needed
3. Add to provider selection logic
4. Integration tests

**Deliverable**: OpenAI provider works out of the box

---

## 7. Breaking Changes

### Public API Changes

**Old API**:
```rust
pub trait LlmProvider {
    fn complete(&self, messages: &[Message]) -> Result<String>;
}
```

**New API**:
```rust
pub trait LlmProvider: CompletionModel {
    // Inherits completion() and stream() from CompletionModel
}
```

### Migration Guide for Users

**Old Code**:
```rust
let provider = ClaudeProvider::new()?;
let response = provider.complete(&messages)?;
```

**New Code**:
```rust
let client = rig::providers::anthropic::Client::from_env();
let agent = client.agent("claude-sonnet-4").build();
let response = agent.prompt("Hello").await?;

// Or with streaming:
let mut stream = agent.stream_prompt("Hello").await?;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk.text());
}
```

### Deprecation Path

1. **Keep old API** for 1-2 versions with deprecation warnings
2. **Add new streaming API** alongside old API
3. **Update documentation** with migration examples
4. **Remove old API** in major version bump

---

## 8. Implementation Plan

### REFAC-001a: Add Rig Dependency (3 points)

**Files to modify**:
- `Cargo.toml` - Add rig-core dependency
- `src/lib.rs` - Re-export rig types
- `src/providers/mod.rs` - Add compat layer

**Tests**:
- All existing tests still pass
- No behavior changes

**Acceptance Criteria**:
- ✅ rig-core 0.25.0 added to dependencies
- ✅ Code compiles with rig types visible
- ✅ No breaking changes to public API
- ✅ cargo test passes

---

### REFAC-001b: Refactor Claude Provider (8 points)

**Files to create/modify**:
- `src/providers/claude_rig.rs` - New rig-based implementation
- `src/providers/mod.rs` - Export both old and new
- `src/providers/claude.rs` - Mark deprecated

**Implementation**:
```rust
// src/providers/claude_rig.rs

use rig::providers::anthropic;
use rig::completion::{CompletionModel, CompletionRequest};
use rig::streaming::StreamingCompletionResponse;

pub struct ClaudeProvider {
    inner: anthropic::CompletionModel,
}

impl ClaudeProvider {
    pub fn new() -> Result<Self> {
        let client = anthropic::Client::from_env();
        let inner = client.agent("claude-sonnet-4-20250514").build();
        Ok(Self { inner })
    }

    pub fn with_oauth() -> Result<Self> {
        let token = std::env::var("CLAUDE_CODE_OAUTH_TOKEN")?;
        let client = anthropic::ClientBuilder::default()
            .header("authorization", format!("Bearer {}", token))
            .build()?;
        Ok(Self { inner: client.agent("claude-sonnet-4-20250514").build() })
    }
}

impl CompletionModel for ClaudeProvider {
    type Response = anthropic::CompletionResponse;
    type StreamingResponse = anthropic::StreamingCompletionResponse;
    type Client = anthropic::Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self {
            inner: anthropic::CompletionModel::make(client, model)
        }
    }

    async fn completion(&self, req: CompletionRequest)
        -> Result<CompletionResponse<Self::Response>, CompletionError>
    {
        self.inner.completion(req).await
    }

    async fn stream(&self, req: CompletionRequest)
        -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        self.inner.stream(req).await
    }
}
```

**Tests**:
- `tests/claude_rig_provider_test.rs`
  - Test basic completion
  - Test streaming
  - Test tool calling
  - Test extended thinking
  - Test OAuth

**Acceptance Criteria**:
- ✅ ClaudeProvider implements rig::completion::CompletionModel
- ✅ Streaming works for text responses
- ✅ Streaming works for tool calls
- ✅ OAuth authentication works
- ✅ Extended thinking is captured
- ✅ All integration tests pass

---

### REFAC-001c: Refactor Agent Loop (8 points)

**Files to create/modify**:
- `src/agent/rig_agent.rs` - New rig-based agent
- `src/agent/mod.rs` - Export both old and new
- `src/tools/rig_tools.rs` - Rig tool adapters

**Implementation**:
```rust
// src/agent/rig_agent.rs

use rig::agent::Agent;
use rig::completion::CompletionModel;
use rig::tool::ToolSet;
use rig::streaming::StreamingCompletionResponse;

pub struct Runner<M: CompletionModel> {
    agent: Agent<M>,
    toolset: ToolSet,
}

impl<M: CompletionModel> Runner<M> {
    pub fn new(model: M) -> Self {
        let mut toolset = ToolSet::new();
        toolset.add_tool(crate::tools::ReadTool);
        toolset.add_tool(crate::tools::WriteTool);
        toolset.add_tool(crate::tools::EditTool);
        toolset.add_tool(crate::tools::BashTool);
        // ... add all tools

        let agent = Agent::builder(model)
            .preamble("You are Claude Code, Anthropic's official CLI for Claude.")
            .tools(&toolset)
            .build();

        Self { agent, toolset }
    }

    /// Non-streaming prompt
    pub async fn run(&self, input: String) -> Result<String> {
        self.agent.prompt(input).await
    }

    /// Streaming prompt
    pub async fn run_streaming(&self, input: String)
        -> Result<StreamingCompletionResponse<M::StreamingResponse>>
    {
        self.agent.stream_prompt(input).await
    }

    /// Multi-turn with depth control
    pub async fn run_multi_turn(&self, input: String, max_depth: usize)
        -> Result<String>
    {
        self.agent.prompt_with_depth(input, max_depth)
            .multi_turn(max_depth)
            .send()
            .await
    }
}
```

**Tool Adapters**:
```rust
// src/tools/rig_tools.rs

use rig::tool::{Tool, ToolDefinition};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ReadArgs {
    file_path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

pub struct ReadTool;

impl Tool for ReadTool {
    const NAME: &'static str = "read_file";
    type Error = std::io::Error;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to file"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start from"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum lines to read"
                    }
                },
                "required": ["file_path"]
            })
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Reuse existing implementation
        crate::tools::read::read_file_impl(
            &args.file_path,
            args.offset,
            args.limit
        ).await
    }
}
```

**Tests**:
- `tests/rig_agent_test.rs`
  - Test basic prompt
  - Test streaming prompt
  - Test multi-turn tool calling
  - Test depth control
  - Test pause/resume streaming
  - Test tool execution

**Acceptance Criteria**:
- ✅ Runner uses rig::agent::Agent
- ✅ Non-streaming prompt works
- ✅ Streaming prompt works
- ✅ Multi-turn tool calling works with depth control
- ✅ All 7 tools work with rig's Tool trait
- ✅ Streaming can be paused/resumed/cancelled
- ✅ All integration tests pass

---

## 9. Testing Strategy

### Unit Tests

**Per-provider tests** (tests/claude_rig_provider_test.rs):
```rust
#[tokio::test]
async fn test_claude_streaming() {
    let provider = ClaudeProvider::new().unwrap();
    let mut stream = provider.stream(request).await.unwrap();

    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            StreamedAssistantContent::Text(t) => text.push_str(&t.text),
            StreamedAssistantContent::ToolCall(tc) => {
                assert_eq!(tc.name, "read_file");
            }
            _ => {}
        }
    }

    assert!(!text.is_empty());
}
```

**Agent tests** (tests/rig_agent_test.rs):
```rust
#[tokio::test]
async fn test_agent_multi_turn_streaming() {
    let client = anthropic::Client::from_env();
    let agent = Runner::new(client.agent("claude-sonnet-4").build());

    let mut stream = agent.run_streaming("Read /tmp/test.txt".to_string()).await.unwrap();

    let mut events = vec![];
    while let Some(item) = stream.next().await {
        events.push(item);
    }

    // Should have: Text, ToolCall, ToolResult, Text, FinalResponse
    assert!(events.iter().any(|e| matches!(e, MultiTurnStreamItem::StreamAssistantItem(..))));
    assert!(events.iter().any(|e| matches!(e, MultiTurnStreamItem::StreamUserItem(..))));
}
```

### Integration Tests

**End-to-end workflow** (tests/integration_streaming_test.rs):
```rust
#[tokio::test]
async fn test_streaming_with_tools_e2e() {
    // Create temp file
    let temp_file = "/tmp/rig_test.txt";
    std::fs::write(temp_file, "Hello Rig!").unwrap();

    // Create agent
    let client = anthropic::Client::from_env();
    let agent = Runner::new(client.agent("claude-sonnet-4").build());

    // Stream prompt with tool use
    let mut stream = agent.run_streaming(
        format!("Read the file {} and tell me what it says", temp_file)
    ).await.unwrap();

    let mut tool_calls = 0;
    let mut final_text = String::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tc)
            )) => {
                assert_eq!(tc.name, "read_file");
                tool_calls += 1;
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(t)
            )) => {
                final_text.push_str(&t.text);
            }
            Err(e) => panic!("Stream error: {}", e),
            _ => {}
        }
    }

    assert_eq!(tool_calls, 1);
    assert!(final_text.contains("Hello Rig"));

    std::fs::remove_file(temp_file).unwrap();
}
```

### Performance Tests

**Streaming latency**:
```rust
#[tokio::test]
async fn test_streaming_latency() {
    let start = Instant::now();
    let mut stream = agent.run_streaming("Hello").await.unwrap();

    let first_chunk_time = stream.next().await;
    let latency = start.elapsed();

    assert!(latency.as_millis() < 2000); // First chunk within 2s
}
```

---

## 10. Risk Assessment

### High Risk Areas

1. **Breaking API Changes** (High Impact)
   - **Risk**: Existing code breaks
   - **Mitigation**: Provide compatibility layer for 2 versions
   - **Rollback**: Keep old implementation alongside new

2. **Streaming State Management** (Medium Impact)
   - **Risk**: Tool call state corrupted during streaming
   - **Mitigation**: Extensive integration tests
   - **Rollback**: Disable streaming, use non-streaming fallback

3. **Provider-Specific Behavior** (Medium Impact)
   - **Risk**: OAuth or custom headers break
   - **Mitigation**: Test OAuth separately, extend rig if needed
   - **Rollback**: Use old ClaudeProvider for OAuth

### Medium Risk Areas

4. **Tool Execution Concurrency** (Low Impact)
   - **Risk**: Race conditions in parallel tool execution
   - **Mitigation**: Test with multiple concurrent tools
   - **Rollback**: Use sequential execution

5. **Memory Usage** (Low Impact)
   - **Risk**: Streaming accumulates too much state
   - **Mitigation**: Profile memory usage in tests
   - **Rollback**: Add streaming buffer limits

### Low Risk Areas

6. **Performance Regression** (Low Impact)
   - **Risk**: Rig abstractions add overhead
   - **Mitigation**: Benchmark before/after
   - **Rollback**: Optimize hot paths

---

## Conclusion

### Why Refactor to Rig?

✅ **Native streaming support** - Handles text and tool call deltas
✅ **Multi-turn tool calling** - Automatic with depth control
✅ **20+ providers** - OpenAI, Gemini, Groq, etc. all work immediately
✅ **Type-safe tools** - Derive macros prevent runtime errors
✅ **Battle-tested** - Used in production by multiple companies
✅ **Active development** - Regular updates and new providers
✅ **Clean abstractions** - Reduces boilerplate significantly

### Recommended Approach

**Split into 3 work units**:
1. **REFAC-001a** (3 points) - Add rig dependency, no behavior changes
2. **REFAC-001b** (8 points) - Refactor providers with streaming
3. **REFAC-001c** (8 points) - Refactor agent loop with multi-turn

**Total: 19 story points** (Large Epic)

**Timeline Estimate**:
- REFAC-001a: 2-3 days
- REFAC-001b: 1 week
- REFAC-001c: 1 week
- **Total: 2.5-3 weeks**

### Next Steps

1. ✅ Review this research document
2. ⏭️ Create REFAC-001a, REFAC-001b, REFAC-001c work units
3. ⏭️ Start with REFAC-001a (low risk, adds dependency)
4. ⏭️ Unblock PROV-003 after REFAC-001b completes
5. ⏭️ Add OpenAI provider using rig (much easier!)

---

## References

- **Rig Repository**: https://github.com/0xPlaygrounds/rig
- **Rig Documentation**: https://docs.rig.rs
- **Rig API Reference**: https://docs.rs/rig-core/latest/rig/
- **Rig Examples**: /tmp/rig/rig-core/examples/
- **Current codelet**: /home/rquast/projects/codelet/
- **Original codelet**: /home/rquast/projects/codelet/ (TypeScript reference)
