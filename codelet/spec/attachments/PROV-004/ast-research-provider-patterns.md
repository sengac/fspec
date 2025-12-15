# AST Research: Provider Implementation Patterns

## Research Objective
Analyze existing ClaudeProvider and OpenAIProvider implementations to understand patterns for porting Codex provider.

## Claude Provider Analysis (src/providers/claude.rs)

### Key Structures
- `ClaudeProvider` struct with fields:
  - `completion_model: anthropic::completion::CompletionModel`
  - `rig_client: anthropic::Client`
  - `model_name: String`
  - `auth_mode: AuthMode` (ApiKey or OAuth)

### Authentication Modes
```rust
pub enum AuthMode {
    ApiKey,
    OAuth { system_prompt_prefix: String },
}
```

### Public Methods
1. `new() -> Result<Self>` - Initialize from ANTHROPIC_API_KEY env var
2. `from_oauth_token(token: &str) -> Result<Self>` - Initialize with OAuth token
3. `client() -> &anthropic::Client` - Get rig client
4. `create_rig_agent() -> rig::agent::Agent<...>` - Create agent with 7 tools

### LlmProvider Trait Implementation
- `name() -> &str` - Returns "claude"
- `model() -> &str` - Returns model name
- `context_window() -> usize` - Returns 200,000
- `max_output_tokens() -> usize` - Returns 8,192
- `supports_caching() -> bool` - Returns true
- `supports_streaming() -> bool` - Returns true
- `complete(&self, messages) -> Result<String>` - Non-tool completion
- `complete_with_tools(&self, messages, tools) -> Result<CompletionResponse>` - Tool-enabled completion

### Helper Methods (Private)
- `extract_text_from_content(content: &MessageContent) -> String` - Extract text from message
- `extract_prompt_data(&self, messages: &[Message]) -> (Option<String>, String)` - Parse system/user messages
- `rig_response_to_completion(...) -> Result<CompletionResponse>` - Convert rig response to our format

## OpenAI Provider Analysis (src/providers/openai.rs)

### Key Structures
- `OpenAIProvider` struct with fields:
  - `completion_model: openai::completion::CompletionModel`
  - `rig_client: openai::CompletionsClient`
  - `model_name: String`

### Public Methods
1. `new() -> Result<Self>` - Initialize from OPENAI_API_KEY env var
2. `from_api_key(api_key: &str, model: &str) -> Result<Self>` - Initialize with explicit key
3. `client() -> &openai::CompletionsClient` - Get rig client
4. `create_rig_agent() -> rig::agent::Agent<...>` - Create agent with 7 tools

### LlmProvider Trait Implementation
- `name() -> &str` - Returns "openai"
- `model() -> &str` - Returns model name
- `context_window() -> usize` - Returns 128,000
- `max_output_tokens() -> usize` - Returns 4,096
- `supports_caching() -> bool` - Returns false (OpenAI doesn't support)
- `supports_streaming() -> bool` - Returns true
- `complete(&self, messages) -> Result<String>` - Delegates to complete_with_tools with no tools
- `complete_with_tools(&self, messages, tools) -> Result<CompletionResponse>` - Tool-enabled completion

### Helper Methods (Private) - DUPLICATE with Claude
- `extract_text_from_content(content: &MessageContent) -> String` - **IDENTICAL TO CLAUDE**
- `extract_prompt_data(&self, messages: &[Message]) -> (Option<String>, String)` - **95% SAME AS CLAUDE**
- `rig_response_to_completion(...) -> Result<CompletionResponse>` - Convert rig response (provider-specific)

## Common Pattern Identified

### Provider Structure Pattern
```rust
pub struct XProvider {
    completion_model: provider::completion::CompletionModel,
    rig_client: provider::Client,  // or provider::CompletionsClient
    model_name: String,
    // Optional: auth-specific fields like AuthMode
}
```

### Constructor Pattern
```rust
impl XProvider {
    pub fn new() -> Result<Self> {
        // 1. Read credentials from env var or file
        // 2. Build rig client
        // 3. Create completion model
        // 4. Return wrapped provider
    }
}
```

### LlmProvider Trait Pattern
All providers must implement:
- Metadata: `name()`, `model()`, `context_window()`, `max_output_tokens()`
- Capabilities: `supports_caching()`, `supports_streaming()`
- Completion: `complete()`, `complete_with_tools()`

### Rig Agent Creation Pattern
```rust
pub fn create_rig_agent(&self) -> rig::agent::Agent<...> {
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
```

## Codex Provider Mapping

### Structure (Following Pattern)
```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,  // Reuse OpenAI model
    rig_client: openai::CompletionsClient,                  // OpenAI client with custom base URL
    model_name: String,                                      // gpt-5.1-codex
    // No auth_mode needed - always uses API key (from token exchange or cached)
}
```

### Constructor (OAuth-Enhanced)
```rust
impl CodexProvider {
    pub fn new() -> Result<Self> {
        // 1. Read from ~/.codex/auth.json (or macOS keychain)
        // 2. If OPENAI_API_KEY exists in auth.json, use directly
        // 3. Else: refresh tokens → exchange for API key → cache
        // 4. Build rig client (optionally with custom base URL)
        // 5. Create completion model
        // 6. Return provider
    }
}
```

### Authentication Module (New)
Create `src/providers/codex_auth.rs`:
- `read_codex_auth() -> Result<Option<CodexAuthJson>>` - Read from file/keychain
- `refresh_tokens(refresh_token: &str) -> Result<RefreshResponse>` - OAuth refresh
- `exchange_token_for_api_key(id_token: &str) -> Result<String>` - Token exchange
- `get_codex_api_key() -> Result<String>` - Full flow

### LlmProvider Implementation
- `name()` → "codex"
- `model()` → model_name (default: "gpt-5.1-codex")
- `context_window()` → 272,000
- `max_output_tokens()` → 4,096
- `supports_caching()` → false
- `supports_streaming()` → true
- `complete()` → Delegate to complete_with_tools
- `complete_with_tools()` → **REUSE OpenAI pattern** (same API format)

## Code Reuse Opportunities

### Option 1: Inherit from OpenAIProvider (Composition)
```rust
pub struct CodexProvider {
    inner: OpenAIProvider,  // Reuse OpenAI implementation
}

impl LlmProvider for CodexProvider {
    fn name(&self) -> &str { "codex" }
    fn complete(&self, messages: &[Message]) -> Result<String> {
        self.inner.complete(messages)  // Delegate
    }
    // ... other methods delegate to inner
}
```

**Pros**: Maximum code reuse, minimal duplication
**Cons**: Less flexible if Codex API diverges from OpenAI

### Option 2: Copy-Paste OpenAI Implementation (Current Approach)
```rust
pub struct CodexProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
}

// Copy complete_with_tools() from OpenAIProvider
// Only change: constructor reads from auth.json
```

**Pros**: Simple, explicit, easy to customize
**Cons**: Code duplication (extract_text_from_content, etc.)

### Recommendation: Option 2 with Shared Helpers
1. Copy OpenAIProvider structure and trait implementation
2. Extract `extract_text_from_content()` to `src/providers/mod.rs` as shared helper
3. Extract tool conversion logic to shared helper
4. Only customize: constructor (OAuth flow), metadata (name, limits)

## Dependencies Required

### Cargo.toml additions
```toml
[dependencies]
# OAuth and token management
keyring = "3.3"        # macOS keychain access
sha2 = "0.10"          # SHA-256 for keyring key computation
uuid = { version = "1.0", features = ["v4"] }  # conversation_id generation

# Already present (for auth.json parsing)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["fs"] }
```

## Implementation Checklist

- [x] AST research on ClaudeProvider
- [x] AST research on OpenAIProvider
- [x] Identify common patterns
- [x] Map Codex provider to pattern
- [x] Identify code reuse opportunities
- [x] Define dependencies
- [ ] Create codex_auth module
- [ ] Implement CodexProvider following OpenAI pattern
- [ ] Extract shared helpers to mod.rs
- [ ] Write comprehensive tests
- [ ] Link coverage

## References
- ClaudeProvider: src/providers/claude.rs (398 lines)
- OpenAIProvider: src/providers/openai.rs (298 lines)
- Codelet TypeScript: ~/projects/codelet/src/agent/codex-provider.ts
- Codelet Auth: ~/projects/codelet/src/agent/codex-auth.ts
