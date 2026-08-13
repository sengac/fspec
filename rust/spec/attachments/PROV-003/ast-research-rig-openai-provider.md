# AST Research: Rig OpenAI Provider Implementation

## Research Objective
Understand the rig-core OpenAI provider API structure to correctly implement OpenAIProvider following the ClaudeProvider pattern.

## Repository Analyzed
- **Source**: https://github.com/0xPlaygrounds/rig (cloned to /tmp/rig)
- **Version**: rig-core 0.25.0
- **Files Analyzed**:
  - `/tmp/rig/rig-core/src/providers/openai/mod.rs`
  - `/tmp/rig/rig-core/src/providers/openai/client.rs`
  - `/tmp/rig/rig-core/src/providers/openai/completion/mod.rs`

## Key Findings

### 1. Client Architecture
Rig provides TWO OpenAI client types:
- **`Client<H>`** (alias for `client::Client<OpenAIResponsesExt, H>`) - Default, uses Responses API
- **`CompletionsClient<H>`** (alias for `client::Client<OpenAICompletionsExt, H>`) - Uses Chat Completions API

**Decision**: Use `CompletionsClient` for standard OpenAI API compatibility.

### 2. CompletionModel Structure
```rust
pub struct CompletionModel<T = reqwest::Client> {
    pub(crate) client: Client<T>,
    pub model: String,
}

impl<T> CompletionModel<T> {
    pub fn new(client: Client<T>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }
}
```

**Key Insight**: CompletionModel takes a `CompletionsClient` (not `Client`) as its first parameter.

### 3. Response Structure
```rust
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: String,  // "tool_calls", "length", "stop", "end_turn"
}
```

**Key Insight**: `finish_reason` is in `choices[].finish_reason`, NOT at top level like Anthropic's `stop_reason`.

### 4. Tool Call Format Differences

**Claude/Anthropic**:
```json
{
  "tool_use": {
    "id": "toolu_123",
    "name": "Read",
    "input": {"file_path": "/tmp/test.txt"}  // JSON object
  }
}
```

**OpenAI**:
```json
{
  "tool_calls": [{
    "id": "call_123",
    "type": "function",
    "function": {
      "name": "Read",
      "arguments": "{\"file_path\":\"/tmp/test.txt\"}"  // JSON string
    }
  }]
}
```

**Implication**: Rig handles the serialization/deserialization automatically, but we need to be aware for debugging.

### 5. Client Initialization Pattern
```rust
// From client.rs:62-71
impl Provider for OpenAICompletionsExt {
    type Builder = OpenAICompletionsExtBuilder;
    const VERIFY_PATH: &'static str = "/models";

    fn build<H>(
        _: &crate::client::ClientBuilder<Self::Builder, OpenAIApiKey, H>,
    ) -> http_client::Result<Self> {
        Ok(Self)
    }
}
```

**Pattern**:
```rust
let rig_client = openai::CompletionsClient::builder()
    .api_key(api_key)
    .build()?;

let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model);
```

### 6. Agent Creation Pattern
```rust
// From codelet ClaudeProvider pattern
pub fn create_rig_agent(&self) -> rig::agent::Agent<openai::completion::CompletionModel> {
    self.rig_client
        .agent(&self.model_name)
        .max_tokens(MAX_OUTPUT_TOKENS as u64)
        .tool(ReadTool::new())
        .tool(WriteTool::new())
        // ... more tools
        .build()
}
```

## Implementation Decisions

### Type Signatures
- **Provider struct**: `OpenAIProvider { completion_model: openai::completion::CompletionModel, rig_client: openai::CompletionsClient, model_name: String }`
- **No generic parameter** on provider (rig handles reqwest::Client internally)

### API Key Handling
- Environment variable: `OPENAI_API_KEY` (required)
- Model override: `OPENAI_MODEL` (optional, defaults to `gpt-4-turbo`)
- Uses `CompletionsClient::builder().api_key().build()` pattern

### Stop Reason Mapping
```rust
match choice.finish_reason.as_str() {
    "tool_calls" => StopReason::ToolUse,
    "length" => StopReason::MaxTokens,
    "stop" | "end_turn" => StopReason::EndTurn,
    other => { warn!(...); StopReason::EndTurn }
}
```

### Caching Support
OpenAI does not support prompt caching → `supports_caching()` returns `false`

### Streaming Support
Rig provides streaming support → `supports_streaming()` returns `true`

## Comparison with ClaudeProvider

| Aspect | ClaudeProvider | OpenAIProvider |
|--------|----------------|----------------|
| Client Type | `anthropic::Client` | `openai::CompletionsClient` |
| Model Creation | `anthropic::completion::CompletionModel::new(client, model)` | `openai::completion::CompletionModel::new(client, model)` |
| Stop Reason Field | `response.raw_response.stop_reason` | `response.raw_response.choices[0].finish_reason` |
| Tool Call Format | `name` + `input` (JSON object) | `function.name` + `function.arguments` (JSON string) |
| Prompt Caching | Supported (returns true) | Not supported (returns false) |
| OAuth Support | Yes (with custom headers) | No (standard API key only) |

## Code References
- ClaudeProvider implementation: `src/providers/claude.rs`
- Rig OpenAI client: `/tmp/rig/rig-core/src/providers/openai/client.rs`
- Rig OpenAI completion: `/tmp/rig/rig-core/src/providers/openai/completion/mod.rs`

## Validation
Implementation successfully compiles with:
- `cargo check` ✓
- `cargo clippy -- -D warnings` ✓
- `cargo fmt --check` ✓
