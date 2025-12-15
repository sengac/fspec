# OpenAI Provider Research - PROV-003

## Executive Summary

Implement a standard OpenAI provider for codelet using the official OpenAI API (api.openai.com), similar to the existing ClaudeProvider. This is the foundation for multi-provider support and is intentionally kept simple - using only OPENAI_API_KEY authentication without Codex-specific OAuth complexity.

**Complexity**: 5-8 story points
**Priority**: High (foundation for multi-provider architecture)
**Dependencies**: None (builds on existing ClaudeProvider pattern)

---

## Background: Multi-Provider Architecture in Codelet

### Original TypeScript Implementation Analysis

The codelet TypeScript version has three provider implementations:

1. **claude-provider.ts** - Anthropic Claude via @ai-sdk/anthropic
2. **codex-provider.ts** - OpenAI Codex via ChatGPT backend API with OAuth
3. **gemini-provider.ts** - Google Gemini via @ai-sdk/google
4. **provider-manager.ts** - Coordinates provider switching and credentials

### Key Discovery: Codex vs Standard OpenAI

**Critical distinction found in codelet source:**

The TypeScript codelet uses **codex-provider.ts**, which is NOT a standard OpenAI provider:

```typescript
// From codelet/src/agent/codex-provider.ts
const CODEX_BACKEND_API = 'https://chatgpt.com/backend-api/codex';
const CODEX_MODEL = 'gpt-5.1-codex';

// Uses OAuth with token refresh
async function refreshTokens(refreshToken: string): Promise<RefreshResponse>
```

This is a complex integration that:
- Uses ChatGPT backend API (NOT standard OpenAI API)
- Requires OAuth authentication with token refresh
- Reads from ~/.codex/auth.json or macOS keychain
- Uses special headers (conversation_id, session_id, chatgpt-account-id)
- Accesses gpt-5.1-codex model (Codex-specific)

**Estimated complexity for full Codex integration: 13+ story points (TOO LARGE)**

---

## Recommendation: Start with Standard OpenAI Provider

### Why Split This Into Two Work Units?

**PROV-003 (This Story): OpenAI Provider (Standard API)** - 5-8 points
- Use standard OpenAI API (api.openai.com)
- OPENAI_API_KEY authentication only
- Support gpt-4, gpt-4-turbo, gpt-3.5-turbo models
- Follow ClaudeProvider pattern exactly

**Future PROV-004: Codex Auth Integration** - 5-8 points (future story)
- Add Codex-specific OAuth flow
- Read from ~/.codex/auth.json
- Token refresh logic
- macOS keychain integration
- ChatGPT backend API support

**Benefits of This Approach:**
1. ✅ Keeps PROV-003 under 8 story points
2. ✅ Proves multi-provider architecture works with simple case first
3. ✅ Delivers value immediately (OpenAI standard API users)
4. ✅ Clean separation of concerns
5. ✅ Easier to test and validate

---

## Architecture Analysis

### Current ClaudeProvider Implementation

Location: `/home/rquast/projects/codelet/src/providers/claude.rs`

**Key features:**
```rust
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    context_window: usize,
    max_output_tokens: usize,
}

impl LlmProvider for ClaudeProvider {
    fn complete(&self, messages: &[Message]) -> Result<String>;
    fn complete_with_tools(&self, messages: &[Message], tools: Vec<Tool>) -> Result<(String, Vec<ToolCall>)>;
    fn supports_streaming(&self) -> bool { false }
    fn supports_caching(&self) -> bool { true }
}
```

**Authentication:**
- Reads ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN from env
- OAuth has special headers and beta features
- Standard API uses x-api-key header

### Proposed OpenAIProvider Structure

**File**: `src/providers/openai.rs`

```rust
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    context_window: usize,
    max_output_tokens: usize,
}

impl OpenAIProvider {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY not set")?;

        Ok(Self {
            api_key,
            model: "gpt-4-turbo".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
        })
    }
}

impl LlmProvider for OpenAIProvider {
    // Implement complete() and complete_with_tools()
    // Same pattern as ClaudeProvider
}
```

---

## OpenAI API Specification

### Endpoint

**Base URL**: `https://api.openai.com/v1`
**Chat Completions**: `POST /v1/chat/completions`

### Authentication

**Header**: `Authorization: Bearer {OPENAI_API_KEY}`

### Request Format

```json
{
  "model": "gpt-4-turbo",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "read_file",
        "description": "Read a file",
        "parameters": {
          "type": "object",
          "properties": {
            "file_path": {"type": "string"}
          },
          "required": ["file_path"]
        }
      }
    }
  ],
  "temperature": 1.0,
  "max_tokens": 4096
}
```

### Response Format

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4-turbo",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you?",
        "tool_calls": [
          {
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "read_file",
              "arguments": "{\"file_path\": \"/path/to/file\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

### Tool Calling Format Differences

| Provider | Tool Call ID | Arguments Format | Tool Result Format |
|----------|--------------|------------------|---------------------|
| Claude | `toolu_abc123` | JSON object in `input` field | `tool_result` content block |
| OpenAI | `call_abc123` | JSON string in `arguments` field | `tool` message with `tool_call_id` |

**OpenAI tool result message:**
```json
{
  "role": "tool",
  "content": "File contents here",
  "tool_call_id": "call_abc123"
}
```

---

## Model Support

### Recommended Default Model

**gpt-4-turbo** (gpt-4-turbo-2024-04-09)
- Context window: 128,000 tokens
- Max output: 4,096 tokens
- Cost: $10/1M input tokens, $30/1M output tokens
- Best balance of performance and cost

### Other Supported Models

| Model | Context Window | Max Output | Cost (per 1M tokens) |
|-------|----------------|------------|---------------------|
| gpt-4-turbo | 128K | 4K | $10/$30 |
| gpt-4 | 8K | 8K | $30/$60 |
| gpt-4-32k | 32K | 32K | $60/$120 |
| gpt-3.5-turbo | 16K | 4K | $0.50/$1.50 |

**Note**: gpt-5.1-codex is NOT available via standard OpenAI API (requires Codex backend).

---

## Implementation Plan

### Phase 1: Core Provider (PROV-003 - This Story)

**Files to create:**
1. `src/providers/openai.rs` - OpenAIProvider implementation
2. `tests/openai_provider_test.rs` - Integration tests

**Files to modify:**
1. `src/providers/mod.rs` - Export OpenAIProvider
2. `src/main.rs` - Add OpenAI initialization logic

**Implementation steps:**

1. **Create OpenAIProvider struct** following ClaudeProvider pattern
2. **Implement LlmProvider trait**:
   - `complete()` - Basic chat completions
   - `complete_with_tools()` - Tool calling support
   - `supports_streaming()` - Return false (future enhancement)
   - `supports_caching()` - Return false (OpenAI doesn't have prompt caching)
3. **HTTP client setup** using reqwest (already in dependencies)
4. **Error handling** - Map OpenAI errors to our error types
5. **Tool format conversion** - OpenAI uses different format than Claude
6. **Token tracking** - Parse usage from response

### Phase 2: Codex Integration (Future PROV-004)

Not part of this story - separate work unit after PROV-003 is complete.

---

## Key Differences from ClaudeProvider

### 1. Tool Call Format

**Claude:**
```json
{
  "type": "tool_use",
  "id": "toolu_abc123",
  "name": "read_file",
  "input": {"file_path": "/path/to/file"}
}
```

**OpenAI:**
```json
{
  "id": "call_abc123",
  "type": "function",
  "function": {
    "name": "read_file",
    "arguments": "{\"file_path\": \"/path/to/file\"}"
  }
}
```

**Conversion needed:**
- Parse `arguments` string to JSON object
- Extract function name from nested structure
- Generate tool result message with `tool_call_id`

### 2. Message Format

**Claude** uses content blocks:
```json
{
  "role": "assistant",
  "content": [
    {"type": "text", "text": "I'll read the file."},
    {"type": "tool_use", "id": "toolu_123", "name": "read_file", "input": {...}}
  ]
}
```

**OpenAI** uses separate fields:
```json
{
  "role": "assistant",
  "content": "I'll read the file.",
  "tool_calls": [
    {"id": "call_123", "type": "function", "function": {"name": "read_file", "arguments": "{...}"}}
  ]
}
```

### 3. Tool Result Injection

**Claude** expects `tool_result` content block in user message:
```json
{
  "role": "user",
  "content": [
    {"type": "tool_result", "tool_use_id": "toolu_123", "content": "..."}
  ]
}
```

**OpenAI** expects separate `tool` role message:
```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "..."
}
```

### 4. No Prompt Caching

OpenAI doesn't have prompt caching like Claude. The `supports_caching()` method should return `false`.

---

## Testing Strategy

### Integration Tests

**File**: `tests/openai_provider_test.rs`

**Test scenarios:**

1. **Provider initialization**
   - ✅ Succeeds with OPENAI_API_KEY set
   - ❌ Fails with clear error when OPENAI_API_KEY missing

2. **Basic completion**
   - ✅ Send prompt, receive text response
   - ✅ Parse token usage correctly
   - ✅ Handle API errors gracefully

3. **Tool calling**
   - ✅ Send prompt with tools, receive tool call
   - ✅ Parse tool call arguments correctly
   - ✅ Handle multiple tool calls in one response
   - ✅ Inject tool results in correct format
   - ✅ Continue conversation after tool execution

4. **Error handling**
   - ❌ Invalid API key (401 error)
   - ❌ Rate limit (429 error)
   - ❌ Model not found (404 error)
   - ❌ Malformed request (400 error)

5. **Model limits**
   - ✅ Context window respected
   - ✅ Max output tokens respected

### Mock vs Real API Testing

**Integration tests** should use real OpenAI API:
- Set OPENAI_API_KEY in test environment
- Use gpt-3.5-turbo (cheaper) for tests
- Keep prompts minimal to reduce cost

**Unit tests** (future) could mock HTTP responses.

---

## Comparison with Codelet TypeScript Implementation

### What We're NOT Porting (Yet)

From `codelet/src/agent/codex-provider.ts`:

❌ **OAuth token refresh**
```typescript
async function refreshTokens(refreshToken: string): Promise<RefreshResponse>
```

❌ **Codex auth.json reading**
```typescript
const auth = readCodexAuth();
if (!auth?.tokens?.refresh_token) return null;
```

❌ **ChatGPT backend API**
```typescript
const CODEX_BACKEND_API = 'https://chatgpt.com/backend-api/codex';
```

❌ **macOS keychain integration**
```typescript
function getKeychainCredentials(): CodexAuthJson | null
```

❌ **Custom headers**
```typescript
headers: {
  'User-Agent': USER_AGENT,
  originator: 'codex_cli_rs',
  conversation_id: session.conversationId,
  session_id: session.conversationId,
  'chatgpt-account-id': session.accountId,
}
```

### What We ARE Porting (PROV-003)

✅ **Basic OpenAI provider structure**
- LlmProvider trait implementation
- HTTP client for api.openai.com
- OPENAI_API_KEY authentication

✅ **Tool calling support**
- Convert tools to OpenAI format
- Parse tool calls from response
- Inject tool results correctly

✅ **Token tracking**
- Parse usage from response
- Track input/output tokens

✅ **Error handling**
- Map OpenAI errors to our error types
- Handle rate limits, auth errors, etc.

---

## Story Point Estimation

### Complexity Breakdown

| Task | Complexity | Points |
|------|------------|--------|
| Create OpenAIProvider struct | Simple | 1 |
| HTTP client setup (reqwest) | Simple (already used in Claude) | 1 |
| Implement complete() | Medium | 2 |
| Implement complete_with_tools() | Medium-High (format conversion) | 3 |
| Tool format conversion logic | Medium | 2 |
| Error handling | Simple | 1 |
| Integration tests | Medium | 2 |
| Documentation | Simple | 1 |

**Total: ~13 points without optimization**

### Optimization: Reuse Claude Patterns

By following ClaudeProvider implementation closely:
- HTTP client setup is identical (reqwest)
- Error handling can be shared
- Tool registry already exists
- Message types are already defined

**Adjusted Total: 5-8 points**

### Risk Factors

**Low risk:**
- ✅ Well-documented OpenAI API
- ✅ Similar to existing ClaudeProvider
- ✅ No OAuth complexity
- ✅ Standard HTTP requests

**Medium risk:**
- ⚠️ Tool format conversion (different from Claude)
- ⚠️ Testing costs (real API calls required)

---

## Dependencies

### Rust Crates

**Already in Cargo.toml:**
- ✅ reqwest (HTTP client)
- ✅ serde, serde_json (JSON serialization)
- ✅ anyhow, thiserror (error handling)
- ✅ async-trait (async traits)
- ✅ tokio (async runtime)

**No new dependencies needed!**

### Environment Variables

**Required:**
- `OPENAI_API_KEY` - OpenAI API key from platform.openai.com

**Optional (for testing):**
- `OPENAI_MODEL` - Override default model (default: gpt-4-turbo)

---

## Acceptance Criteria (Draft)

These will be formalized in Example Mapping during specifying phase:

### Must Have

1. ✅ OpenAIProvider implements LlmProvider trait
2. ✅ Reads OPENAI_API_KEY from environment
3. ✅ complete() sends request to api.openai.com and returns response
4. ✅ complete_with_tools() supports tool calling with proper format conversion
5. ✅ Parses token usage from response
6. ✅ Tool calls are correctly parsed and executed
7. ✅ Tool results are injected in correct OpenAI format
8. ✅ Comprehensive integration tests pass
9. ✅ Error handling for auth failures, rate limits, malformed requests

### Should Have

10. ✅ Support gpt-4-turbo (default), gpt-4, gpt-3.5-turbo models
11. ✅ Configurable via environment variable (OPENAI_MODEL)
12. ✅ Clear error messages when OPENAI_API_KEY missing

### Won't Have (Future PROV-004)

13. ❌ Codex OAuth authentication
14. ❌ Reading from ~/.codex/auth.json
15. ❌ macOS keychain integration
16. ❌ ChatGPT backend API support
17. ❌ Provider switching infrastructure (ProviderManager)

---

## Next Steps

1. **Move to specifying state**: `fspec update-work-unit-status PROV-003 specifying`
2. **Conduct Example Mapping**:
   - Define user story (role/action/benefit)
   - Add business rules (blue cards)
   - Add concrete examples (green cards)
   - Ask clarifying questions (red cards)
3. **Generate scenarios**: `fspec generate-scenarios PROV-003`
4. **Move to testing**: Write integration tests following feature scenarios
5. **Move to implementing**: Implement OpenAIProvider
6. **Move to validating**: Run tests, verify quality
7. **Mark done**: Complete PROV-003

---

## References

- **OpenAI API Docs**: https://platform.openai.com/docs/api-reference/chat
- **OpenAI Tool Calling**: https://platform.openai.com/docs/guides/function-calling
- **Codelet TypeScript Source**: `/home/rquast/projects/codelet/src/agent/`
- **ClaudeProvider Implementation**: `/home/rquast/projects/codelet/src/providers/claude.rs`
- **Foundation Event Storm**: `/home/rquast/projects/codelet/spec/foundation.json`

---

## Conclusion

**PROV-003 is the right next feature because:**

1. ✅ **Small enough** (5-8 points after reusing Claude patterns)
2. ✅ **Foundational** (proves multi-provider architecture)
3. ✅ **Within Event Storm constraints** (Provider Management bounded context)
4. ✅ **Clear requirements** (well-documented OpenAI API)
5. ✅ **Low risk** (similar to existing ClaudeProvider)
6. ✅ **Immediate value** (enables OpenAI users)
7. ✅ **Sets up future work** (Codex integration, ProviderManager, Gemini)

**Not porting full Codex complexity yet** - that's intentional. We're building the foundation first, then adding OAuth/Codex in PROV-004 after PROV-003 proves the multi-provider pattern works.
