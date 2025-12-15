# PROV-003 Implementation Plan

## TL;DR

Implement standard OpenAI provider (api.openai.com) using OPENAI_API_KEY authentication. Follow ClaudeProvider pattern exactly. Intentionally simple - NO Codex OAuth complexity yet.

**Estimated: 5-8 story points**

---

## What We're Building

A second LLM provider for codelet that:
- Uses standard OpenAI API (https://api.openai.com/v1)
- Reads OPENAI_API_KEY from environment
- Supports tool calling with format conversion
- Default model: gpt-4-turbo (128K context, 4K output)
- Follows existing LlmProvider trait

---

## What We're NOT Building (Yet)

❌ Codex OAuth authentication (future PROV-004)
❌ Reading ~/.codex/auth.json (future PROV-004)
❌ ChatGPT backend API (future PROV-004)
❌ Provider switching UI (future - ProviderManager)
❌ macOS keychain integration (future PROV-004)

**Why split this?** The full Codex integration is 13+ points. By doing standard OpenAI first:
1. Stay under 8 point constraint
2. Prove multi-provider architecture works
3. Deliver immediate value (standard OpenAI users)
4. Reduce complexity and risk

---

## File Structure

```
src/providers/
├── mod.rs              # Export OpenAIProvider (modify)
├── claude.rs           # Existing ClaudeProvider (reference)
└── openai.rs           # New OpenAIProvider (create)

tests/
└── openai_provider_test.rs  # Integration tests (create)
```

---

## Implementation Steps

### 1. Create OpenAIProvider Struct (src/providers/openai.rs)

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
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4-turbo".to_string());

        Ok(Self {
            api_key,
            model,
            context_window: 128_000,
            max_output_tokens: 4_096,
        })
    }
}
```

### 2. Implement LlmProvider Trait

```rust
impl LlmProvider for OpenAIProvider {
    fn complete(&self, messages: &[Message]) -> Result<String> {
        // HTTP POST to api.openai.com/v1/chat/completions
        // Convert messages to OpenAI format
        // Parse response and extract content
    }

    fn complete_with_tools(&self, messages: &[Message], tools: Vec<Tool>)
        -> Result<(String, Vec<ToolCall>)> {
        // Same as complete() but include tools array
        // Parse tool_calls from response
        // Convert OpenAI format to our ToolCall type
    }

    fn supports_streaming(&self) -> bool { false }
    fn supports_caching(&self) -> bool { false }
}
```

### 3. Tool Format Conversion

**Challenge**: OpenAI and Claude use different tool formats.

**OpenAI tool call:**
```json
{
  "id": "call_abc123",
  "type": "function",
  "function": {
    "name": "read_file",
    "arguments": "{\"file_path\": \"/path\"}"
  }
}
```

**Our ToolCall struct:**
```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,  // Parsed JSON object
}
```

**Conversion needed:**
- Extract `function.name`
- Parse `function.arguments` string to JSON
- Store as ToolCall

### 4. Tool Result Injection

**OpenAI expects `tool` role message:**
```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "File contents here"
}
```

**Implementation:**
```rust
// In complete_with_tools(), after executing tools:
for (tool_call, result) in tool_calls.iter().zip(results.iter()) {
    messages.push(Message {
        role: "tool".to_string(),
        content: result.clone(),
        tool_call_id: Some(tool_call.id.clone()),
    });
}
```

### 5. HTTP Request Structure

```rust
#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    temperature: f32,
    max_tokens: usize,
}

let client = reqwest::Client::new();
let response = client
    .post("https://api.openai.com/v1/chat/completions")
    .header("Authorization", format!("Bearer {}", self.api_key))
    .header("Content-Type", "application/json")
    .json(&request)
    .send()
    .await?;
```

### 6. Error Handling

Map OpenAI errors to our error types:
- 401 Unauthorized → "Invalid OPENAI_API_KEY"
- 429 Rate Limit → "Rate limit exceeded, retry after X seconds"
- 404 Not Found → "Model not found: {model}"
- 400 Bad Request → "Malformed request: {error_message}"

### 7. Integration Tests

**tests/openai_provider_test.rs:**

```rust
#[tokio::test]
async fn test_openai_provider_initialization() {
    std::env::set_var("OPENAI_API_KEY", "test-key");
    let provider = OpenAIProvider::new();
    assert!(provider.is_ok());
}

#[tokio::test]
async fn test_basic_completion() {
    // Requires real OPENAI_API_KEY in env
    let provider = OpenAIProvider::new().unwrap();
    let messages = vec![
        Message::user("Hello!")
    ];
    let response = provider.complete(&messages).unwrap();
    assert!(!response.is_empty());
}

#[tokio::test]
async fn test_tool_calling() {
    let provider = OpenAIProvider::new().unwrap();
    let messages = vec![
        Message::user("Read /tmp/test.txt")
    ];
    let tools = vec![
        Tool {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            parameters: /* JSON schema */
        }
    ];
    let (response, tool_calls) = provider.complete_with_tools(&messages, tools).unwrap();
    assert!(!tool_calls.is_empty());
    assert_eq!(tool_calls[0].name, "read_file");
}
```

---

## Testing Strategy

### Environment Setup

```bash
# Required for integration tests
export OPENAI_API_KEY="sk-..."

# Optional: use cheaper model for tests
export OPENAI_MODEL="gpt-3.5-turbo"
```

### Test Coverage

1. ✅ Provider initialization (with/without API key)
2. ✅ Basic completion (simple prompt → response)
3. ✅ Tool calling (tool definition → tool call parsing)
4. ✅ Tool result injection (format conversion)
5. ✅ Multi-turn conversation
6. ✅ Error handling (401, 429, 400, 404)
7. ✅ Token usage tracking

### Cost Management

Integration tests will make real API calls. To minimize cost:
- Use gpt-3.5-turbo for tests ($0.50/$1.50 per 1M tokens)
- Keep test prompts minimal
- Consider CI/CD environment variable gating

---

## Key Architectural Decisions

### Decision 1: Standard API Only (No Codex Yet)

**Rationale:**
- Codex OAuth is complex (13+ points alone)
- Standard OpenAI API is well-documented and stable
- Delivers immediate value to OpenAI standard users
- Proves multi-provider architecture works first

**Trade-off:** Codex users must wait for PROV-004

### Decision 2: Follow ClaudeProvider Pattern Exactly

**Rationale:**
- Proven architecture
- Reuses HTTP client setup
- Similar error handling
- Consistent with codebase patterns

**Benefit:** Reduces implementation risk and complexity

### Decision 3: No Streaming Support Initially

**Rationale:**
- ClaudeProvider doesn't support streaming yet
- Adds significant complexity
- Not required for MVP functionality

**Future:** Add streaming in separate work unit after basic providers work

### Decision 4: Real API Integration Tests

**Rationale:**
- Mocking HTTP responses is complex for tool calling
- Real API calls catch integration issues
- Cost is low for minimal test suite

**Trade-off:** Tests require OPENAI_API_KEY and incur small cost

---

## Success Criteria

### Technical

1. ✅ OpenAIProvider implements LlmProvider trait
2. ✅ All integration tests pass
3. ✅ cargo clippy -- -D warnings passes
4. ✅ cargo fmt --check passes
5. ✅ Tool calling works end-to-end (agent → tool → result → response)

### Functional

6. ✅ User can set OPENAI_API_KEY and use OpenAI models
7. ✅ Agent can execute tools via OpenAI provider
8. ✅ Token usage is tracked correctly
9. ✅ Clear error messages for missing API key or auth failures

### Quality

10. ✅ Code follows Rust best practices
11. ✅ Error handling is comprehensive
12. ✅ Documentation is clear
13. ✅ No clippy warnings

---

## Risk Mitigation

### Risk: Tool Format Conversion Bugs

**Likelihood:** Medium
**Impact:** High (breaks tool calling)

**Mitigation:**
- Write comprehensive tests for format conversion
- Compare OpenAI API docs carefully
- Test with all core tools (Read, Write, Edit, Bash, Grep)

### Risk: API Rate Limits During Testing

**Likelihood:** Low (small test suite)
**Impact:** Low (can retry)

**Mitigation:**
- Use gpt-3.5-turbo for tests (higher rate limits)
- Add exponential backoff for 429 errors
- Consider test environment variable gating

### Risk: Breaking Existing ClaudeProvider

**Likelihood:** Low (separate file)
**Impact:** High (breaks existing functionality)

**Mitigation:**
- Don't modify claude.rs
- Only modify providers/mod.rs to export new provider
- Run all existing tests before committing

---

## Timeline Estimate

Based on 5-8 story points and typical velocity:

| Phase | Tasks | Time |
|-------|-------|------|
| Specifying | Example Mapping, generate scenarios | 1 hour |
| Testing | Write integration tests | 2 hours |
| Implementing | Create OpenAIProvider, tool format conversion | 3-4 hours |
| Validating | Run tests, fix issues, quality checks | 1 hour |

**Total: 7-8 hours of focused work**

---

## Next Actions

1. ✅ Research complete (this document)
2. ⏭️ Move to specifying: `fspec update-work-unit-status PROV-003 specifying`
3. ⏭️ Conduct Example Mapping (define acceptance criteria)
4. ⏭️ Generate scenarios: `fspec generate-scenarios PROV-003`
5. ⏭️ Write tests (move to testing state)
6. ⏭️ Implement (move to implementing state)
7. ⏭️ Validate (move to validating state)
8. ⏭️ Mark done

---

## References

- **Full Research**: See `openai-provider-research.md` for detailed API spec and analysis
- **OpenAI API**: https://platform.openai.com/docs/api-reference/chat
- **ClaudeProvider**: `/home/rquast/projects/codelet/src/providers/claude.rs`
- **Event Storm**: Provider Management bounded context, OpenAIProvider aggregate
