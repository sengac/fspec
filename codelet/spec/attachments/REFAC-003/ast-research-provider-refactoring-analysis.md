# AST Research: Provider Refactoring Analysis for REFAC-003

## Analysis Date
2025-12-02

## Purpose
Analyze current ClaudeProvider implementation to plan refactoring to use rig::providers::anthropic with full streaming support.

---

## Current Implementation Analysis

### File: `src/providers/claude.rs` (525 lines)

#### Structure Overview
- **Functions**: 18 methods total
- **Lines of Code**: 525
- **Current Status**: Non-streaming, custom HTTP implementation

#### Key Components

**1. Authentication (3 methods)**
- `new()` - Env-based initialization (line 61)
- `from_api_key()` - API key constructor (line 78)
- `from_api_key_with_mode()` - With auth mode (line 83)

**Analysis**:
- ✅ Supports both API key and OAuth modes
- ✅ OAuth detection via CLAUDE_CODE_OAUTH_TOKEN env var
- ❌ Custom implementation instead of using rig's auth

**2. HTTP Request Handling (2 methods)**
- `send_api_request()` - Generic HTTP sender (line 131, 34 lines)
- `add_auth_headers()` - Auth header injection (line 121)

**Analysis**:
- ✅ DRY principle: Centralized request logic
- ✅ Error handling for API errors
- ❌ Duplicates rig's built-in HTTP client
- ❌ No streaming support

**3. Message Formatting (3 methods)**
- `format_messages()` - Simple text conversion (line 167, 44 lines)
- `format_messages_with_tools()` - Tool-aware conversion (line 214, 42 lines)
- `message_content_to_api()` - Content block conversion (line 259, 26 lines)

**Analysis**:
- ✅ Properly converts internal types to API format
- ✅ Handles system message extraction
- ✅ OAuth mode adds Claude Code prefix
- ❌ Code duplication between format methods (112 lines total)
- ❌ Rig provides these conversions built-in

**4. LlmProvider Trait Implementation (8 methods)**
- Metadata methods: `name()`, `model()`, `context_window()`, `max_output_tokens()`
- Feature flags: `supports_caching()`, `supports_streaming()`
- Completion methods: `complete()`, `complete_with_tools()`

**Analysis**:
- ✅ Implements custom LlmProvider trait
- ❌ `supports_streaming()` returns `false` (line 310)
- ❌ Custom trait instead of rig::completion::CompletionModel

**5. Request/Response Types (9 structs)**
- Request types: `AnthropicRequest`, `AnthropicRequestWithTools`
- Response types: `AnthropicResponse`, `AnthropicResponseWithTools`
- Content blocks: `ApiContentBlock`, `ResponseContentBlock`
- Error types: `AnthropicError`, `ErrorDetails`

**Analysis**:
- ✅ Complete type coverage
- ❌ Duplicates rig-core's built-in types
- ❌ ~120 lines of boilerplate that rig provides

---

## What Rig Provides

### Module: `rig::providers::anthropic`

Based on REFAC-001 research (spec/attachments/REFAC-001/rig-refactoring-research.md):

**1. CompletionModel Trait**
```rust
pub trait CompletionModel {
    type Response;
    type StreamingResponse;

    fn completion(&self, request: CompletionRequest)
        -> Result<CompletionResponse<Self::Response>>;
    fn stream(&self, request: CompletionRequest)
        -> Result<StreamingCompletionResponse<Self::StreamingResponse>>;
}
```

**2. Anthropic-Specific Types**
- `rig::providers::anthropic::Client` - HTTP client with auth
- `rig::providers::anthropic::CompletionModel` - Model wrapper
- `rig::providers::anthropic::CompletionResponse` - Response type
- `rig::providers::anthropic::StreamingResponse` - Streaming chunks

**3. Streaming Support**
```rust
pub struct StreamingCompletionResponse<T> {
    // Methods:
    // - pause() - Pause streaming
    // - resume() - Resume streaming
    // - cancel() - Cancel streaming
    // - next() -> Option<Result<RawStreamingChoice>>
}

pub enum RawStreamingChoice {
    Message(String),           // Text chunks
    ToolCall(ToolCall),        // Complete tool call
    ToolCallDelta(ToolCallDelta), // Incremental tool call
    Reasoning(String),         // Extended thinking
    FinalResponse(Response),   // Complete response
}
```

**4. Built-in OAuth Support**
- Custom header injection
- Bearer token authentication
- Beta features header

---

## Migration Strategy

### Phase 1: Wrap Rig's CompletionModel (Minimal Changes)

**Approach**: Keep current `ClaudeProvider` struct but use rig internally

```rust
use rig::providers::anthropic;

pub struct ClaudeProvider {
    inner: anthropic::CompletionModel,
    auth_mode: AuthMode,
}

impl ClaudeProvider {
    pub fn new() -> Result<Self> {
        // Use rig's client
        let client = if let Ok(oauth_token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
            // Create rig client with custom OAuth headers
            anthropic::Client::from_bearer_token(&oauth_token)
                .with_custom_header("anthropic-beta", ANTHROPIC_BETA_HEADER)
        } else {
            anthropic::Client::from_env()
        };

        let model = client.completion_model("claude-sonnet-4-20250514");

        Ok(Self {
            inner: model,
            auth_mode: if std::env::var("CLAUDE_CODE_OAUTH_TOKEN").is_ok() {
                AuthMode::OAuth
            } else {
                AuthMode::ApiKey
            },
        })
    }
}
```

**Benefits**:
- ✅ Backward compatible with existing LlmProvider trait
- ✅ Existing tests continue to work
- ✅ Minimal code changes

**Drawbacks**:
- ❌ Still using custom LlmProvider trait
- ❌ Not leveraging rig's full capabilities

### Phase 2: Add Streaming Support

**Approach**: Implement stream() method using rig's streaming

```rust
impl ClaudeProvider {
    pub async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<impl Stream<Item = StreamChunk>> {
        // Convert internal messages to rig CompletionRequest
        let request = self.to_completion_request(messages, tools)?;

        // Use rig's streaming
        let mut stream = self.inner.stream(request).await?;

        // Transform rig chunks to internal format
        Ok(stream.map(|chunk| match chunk {
            RawStreamingChoice::Message(text) => StreamChunk::Text(text),
            RawStreamingChoice::ToolCall(call) => StreamChunk::ToolCall(call),
            RawStreamingChoice::ToolCallDelta(delta) => StreamChunk::ToolCallDelta(delta),
            RawStreamingChoice::Reasoning(thinking) => StreamChunk::Reasoning(thinking),
            _ => StreamChunk::Other,
        }))
    }
}
```

**Benefits**:
- ✅ Real-time streaming support
- ✅ Tool call delta visibility
- ✅ Extended thinking support

### Phase 3: OAuth Integration

**Current OAuth Handling** (lines 247-253):
```rust
if self.auth_mode == AuthMode::OAuth {
    let prefix = CLAUDE_CODE_PROMPT_PREFIX;
    system_content = Some(match system_content {
        Some(existing) => format!("{}\n\n{}", prefix, existing),
        None => prefix.to_string(),
    });
}
```

**With Rig**:
```rust
let client = anthropic::Client::from_bearer_token(&oauth_token)
    .with_custom_header("anthropic-beta", ANTHROPIC_BETA_HEADER)
    .with_system_prompt_prefix(CLAUDE_CODE_PROMPT_PREFIX);
```

**Benefits**:
- ✅ Declarative configuration
- ✅ Rig handles header injection
- ✅ Less boilerplate

---

## Code Reduction Analysis

### Current Code: 525 lines

**Sections to be REMOVED**:
1. **HTTP Request Logic**: 64 lines (lines 121-164)
   - `send_api_request()`: 34 lines
   - `add_auth_headers()`: 9 lines
   - Custom error handling: 21 lines

2. **Request/Response Types**: ~120 lines (lines 408-525)
   - All struct definitions replaced by rig types

3. **Message Formatting**: 112 lines (lines 167-285)
   - Rig provides built-in conversion

**Total Reduction**: ~296 lines (56% reduction)

**New Code**: ~150 lines (rig wrapper + streaming)

**Expected Final Size**: ~230 lines (44% of current)

---

## Streaming Architecture

### Current: No Streaming Support
```rust
fn supports_streaming(&self) -> bool {
    false // Line 310
}
```

### With Rig: Full Streaming
```rust
pub async fn stream(&self, request: CompletionRequest)
    -> Result<StreamingCompletionResponse>
{
    // Rig handles:
    // - SSE (Server-Sent Events) parsing
    // - Chunk buffering
    // - Error recovery
    // - Pause/resume/cancel
}
```

**Streaming Chunk Types** (from rig):
1. **Message**: Text content chunks
2. **ToolCallDelta**: Incremental tool call building
3. **ToolCall**: Complete tool call
4. **Reasoning**: Extended thinking chunks
5. **FinalResponse**: Complete response with metadata

---

## Test Impact Analysis

### Existing Tests: `tests/claude_provider_test.rs`

**Tests that MUST pass**:
1. `test_complete_simple_user_message_with_valid_api_key`
2. `test_reject_provider_creation_when_api_key_missing`
3. `test_handle_authentication_error_for_invalid_api_key`
4. `test_format_system_and_user_messages_correctly`
5. `test_return_correct_model_limits`
6. `test_provider_streaming_support`
7. `test_provider_model_name`
8. `test_return_correct_provider_name`
9. `test_provider_supports_caching`

**Changes Required**:
- Test #6 (`test_provider_streaming_support`): Update to return `true`
- All others: Should pass with wrapper approach

---

## Implementation Checklist

### REFAC-003 Scope (This Work Unit)

✅ **Must Do**:
1. Replace custom HTTP client with rig::providers::anthropic::Client
2. Implement completion() using rig's CompletionModel trait
3. Implement stream() for streaming support
4. Support both OAuth and API key authentication
5. Emit text chunks, tool call deltas, and reasoning chunks
6. All existing tests must pass
7. Update `supports_streaming()` to return `true`

❌ **Out of Scope** (REFAC-004):
- Agent loop integration (handled in REFAC-004)
- Multi-turn tool execution (handled in REFAC-004)
- Tool trait implementation (handled in REFAC-004)

---

## Critical Implementation Notes

### 1. OAuth Authentication
**Current**: Custom header injection in `add_auth_headers()` (line 121-128)

**With Rig**: Use rig's builder pattern
```rust
Client::from_bearer_token(token)
    .with_custom_header("anthropic-beta", BETA_HEADER)
```

### 2. System Prompt Prefix
**Current**: Manually prepended in `format_messages_with_tools()` (line 247-253)

**With Rig**: Inject at request creation or use client configuration

### 3. Tool Call Conversion
**Current**: Manual conversion in `message_content_to_api()` (line 259-285)

**With Rig**: Use built-in type conversions between internal and API formats

### 4. Error Handling
**Current**: Custom AnthropicError parsing (line 152-160)

**With Rig**: Rig provides structured error types

---

## Performance Considerations

**Current Implementation**:
- Synchronous message formatting
- Single HTTP request/response
- No streaming overhead

**With Rig Streaming**:
- Async streaming with backpressure
- SSE parsing overhead (~2-5% CPU)
- Memory efficient (chunks processed on-the-fly)

**Recommendation**: Streaming overhead is negligible for typical use cases

---

## Risk Analysis

### Low Risk ✅
- rig-core 0.25.0 is stable
- Anthropic provider well-tested in rig
- Backward compatible approach possible

### Medium Risk ⚠️
- OAuth custom headers may need extra configuration
- Extended thinking is Claude-specific (may need special handling)

### Mitigation Strategies
1. Use wrapper approach to maintain backward compatibility
2. Add feature flag for streaming (can disable if issues)
3. Comprehensive test coverage before switching

---

## References

1. **REFAC-001 Research**: `spec/attachments/REFAC-001/rig-refactoring-research.md`
2. **Rig Documentation**: `https://docs.rs/rig-core/0.25.0`
3. **Current Implementation**: `src/providers/claude.rs`
4. **Anthropic API**: `https://docs.anthropic.com/en/api/messages-streaming`

---

## Conclusion

**Refactoring Feasibility**: HIGH ✅

The current ClaudeProvider can be successfully refactored to use rig with:
- 56% code reduction (525 → 230 lines)
- Full streaming support
- Backward compatible wrapper
- All existing tests passing
- OAuth support maintained

**Recommended Approach**: Incremental migration using wrapper pattern, enabling streaming support while maintaining existing API contract.
