# REFAC-013: Architectural Refactoring Plan

## Overview

This document details the refactoring work needed to address architectural issues identified in the codelet codebase. The primary goals are:

1. **Split God Objects** - Break down monolithic modules into focused, testable components
2. **Unify Error Handling** - Create composable error types across tools and providers
3. **Reduce Provider Duplication** - Extract common patterns into shared abstractions

---

## Phase 1: Split God Objects

### 1.1 Split `cli/src/interactive.rs` (845 lines)

**Current State:**
Single file handling REPL loop, response streaming, tool execution, token tracking, and compaction triggering.

**Target Structure:**
```
cli/src/interactive/
├── mod.rs                 # Public API, orchestration
├── repl_loop.rs           # Main REPL loop logic
├── stream_handler.rs      # Response streaming and rendering
├── tool_executor.rs       # Tool invocation and result handling
└── token_display.rs       # Token usage display and tracking
```

**Refactoring Steps:**

1. Create `cli/src/interactive/` directory
2. Extract `ReplLoop` struct and main loop into `repl_loop.rs`
   - `async fn run_loop(&mut self) -> Result<()>`
   - Input handling
   - Session state management
3. Extract streaming logic into `stream_handler.rs`
   - `StreamHandler` struct
   - `async fn handle_stream(&mut self, stream: impl Stream<Item=...>) -> Result<()>`
   - Text accumulation and rendering
4. Extract tool handling into `tool_executor.rs`
   - `ToolExecutor` struct
   - `async fn execute_tool(&self, call: ToolCall) -> ToolResult`
   - Tool result formatting
5. Extract token display into `token_display.rs`
   - `TokenDisplayManager` struct
   - Token usage formatting
   - Cache hit display
6. Update `mod.rs` to compose these components
7. Update imports in `cli/src/lib.rs`

**Success Criteria:**
- No file exceeds 250 lines
- Each module has single responsibility
- All existing tests pass
- Public API unchanged

---

### 1.2 Split `core/src/compaction.rs` (699 lines)

**Current State:**
Single file containing token tracking structs, conversation turn management, anchor point detection, compaction algorithm, and result types.

**Target Structure:**
```
core/src/compaction/
├── mod.rs                 # Public API, re-exports
├── model.rs               # Data types (TokenTracker, ConversationTurn, ToolCall, ToolResult)
├── anchor.rs              # Anchor point detection (AnchorType, find_anchors)
├── compactor.rs           # Compaction algorithm (ContextCompactor)
└── metrics.rs             # Result types (CompactionResult, CompactionMetrics)
```

**Refactoring Steps:**

1. Create `core/src/compaction/` directory
2. Extract data types into `model.rs`
   - `TokenTracker` struct
   - `ConversationTurn` struct
   - `ToolCall` struct
   - `ToolResult` struct
   - Related impl blocks
3. Extract anchor detection into `anchor.rs`
   - `AnchorType` enum
   - `AnchorPoint` struct
   - `fn find_anchors(turns: &[ConversationTurn]) -> Vec<AnchorPoint>`
4. Extract compaction algorithm into `compactor.rs`
   - `ContextCompactor` struct
   - `async fn compact(&self, history: &mut Vec<Message>) -> CompactionResult`
   - LLM summarization logic
5. Extract result types into `metrics.rs`
   - `CompactionResult` struct
   - `CompactionMetrics` struct
6. Update `mod.rs` with public re-exports
7. Update `core/src/lib.rs` imports

**Success Criteria:**
- No file exceeds 200 lines
- Clear separation between data, algorithm, and results
- All existing tests pass
- Public API unchanged

---

### 1.3 Split `common/src/debug_capture.rs` (716 lines)

**Current State:**
Single file with global singleton debug manager, multiple capture types, file I/O, and formatting logic.

**Target Structure:**
```
common/src/debug_capture/
├── mod.rs                 # Public API, manager access
├── manager.rs             # DebugCaptureManager struct (non-singleton)
├── capture.rs             # Capture types (ApiCapture, ToolCapture, etc.)
├── storage.rs             # File I/O for captures
└── formatting.rs          # JSON/display formatting
```

**Refactoring Steps:**

1. Create `common/src/debug_capture/` directory
2. Extract capture types into `capture.rs`
   - `ApiCapture` struct
   - `ToolCapture` struct
   - `EventCapture` enum
   - Serialization derives
3. Extract storage logic into `storage.rs`
   - `CaptureStorage` struct
   - `fn save_capture(&self, capture: &EventCapture) -> Result<()>`
   - `fn load_captures(&self) -> Result<Vec<EventCapture>>`
   - Directory management
4. Extract formatting into `formatting.rs`
   - `fn format_capture_json(capture: &EventCapture) -> String`
   - `fn format_capture_display(capture: &EventCapture) -> String`
   - Redaction logic
5. Refactor manager in `manager.rs`
   - `DebugCaptureManager` struct (injectable, not global)
   - Constructor takes `CaptureStorage`
   - Methods use composed components
6. Update `mod.rs` with optional global accessor
   - Keep `get_debug_capture_manager()` for backwards compatibility
   - Add `DebugCaptureManager::new()` for testability
7. Update `common/src/lib.rs`

**Success Criteria:**
- No file exceeds 200 lines
- Manager is injectable for testing
- Global accessor maintained for backwards compatibility
- All existing functionality preserved

---

## Phase 2: Unify Error Handling

### 2.1 Create Unified Tool Error Type

**Current State:**
Each tool defines its own error enum:
```rust
// bash.rs
pub enum BashError { ExecutionError(String), TimeoutError(u64) }

// read.rs
pub enum ReadError { FileNotFound(PathBuf), PermissionDenied(PathBuf) }

// write.rs
pub enum WriteError { FileError(std::io::Error), ValidationError(String) }
```

Cannot handle errors uniformly in caller code.

**Target State:**
```rust
// tools/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    #[error("execution failed: {message}")]
    Execution { tool: String, message: String },

    #[error("validation failed: {message}")]
    Validation { tool: String, message: String },

    #[error("timeout after {seconds}s")]
    Timeout { tool: String, seconds: u64 },

    #[error("file error on {path}: {source}")]
    File {
        tool: String,
        path: PathBuf,
        #[source] source: std::io::Error
    },

    #[error("permission denied: {path}")]
    PermissionDenied { tool: String, path: PathBuf },

    #[error("path not found: {path}")]
    NotFound { tool: String, path: PathBuf },

    #[error("output truncated: {actual} bytes exceeded {limit} limit")]
    OutputTruncated { tool: String, actual: usize, limit: usize },
}

impl ToolError {
    pub fn tool_name(&self) -> &str { /* ... */ }
    pub fn is_retryable(&self) -> bool { /* ... */ }
}
```

**Refactoring Steps:**

1. Create `tools/src/error.rs` with unified `ToolError` enum
2. Add `thiserror` derive macros
3. Implement helper methods (`tool_name()`, `is_retryable()`)
4. Update `BashTool` to return `ToolError`
5. Update `ReadTool` to return `ToolError`
6. Update `WriteTool` to return `ToolError`
7. Update `EditTool` to return `ToolError`
8. Update `GrepTool` to return `ToolError`
9. Update `GlobTool` to return `ToolError`
10. Update `AstGrepTool` to return `ToolError`
11. Update `LsTool` to return `ToolError`
12. Remove individual error enums from each tool
13. Update `tools/src/lib.rs` exports
14. Update callers in CLI to use unified error handling

**Success Criteria:**
- Single `ToolError` type for all tools
- Error messages include tool name for debugging
- `is_retryable()` allows smart retry logic
- All tests pass

---

### 2.2 Create Unified Provider Error Type

**Current State:**
Providers use `anyhow::Result` everywhere, losing type information:
```rust
// claude.rs
Err(anyhow::anyhow!("API key not found"))

// openai.rs
Err(anyhow::anyhow!("Invalid response format"))
```

**Target State:**
```rust
// providers/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("authentication failed for {provider}: {message}")]
    Authentication { provider: String, message: String },

    #[error("API error from {provider}: {status} - {message}")]
    Api { provider: String, status: u16, message: String },

    #[error("rate limited by {provider}, retry after {retry_after_secs}s")]
    RateLimited { provider: String, retry_after_secs: u64 },

    #[error("invalid response from {provider}: {message}")]
    InvalidResponse { provider: String, message: String },

    #[error("network error connecting to {provider}: {source}")]
    Network {
        provider: String,
        #[source] source: reqwest::Error
    },

    #[error("model {model} not available for {provider}")]
    ModelNotAvailable { provider: String, model: String },

    #[error("context length exceeded: {used} tokens > {limit} limit")]
    ContextLengthExceeded { provider: String, used: usize, limit: usize },
}
```

**Refactoring Steps:**

1. Create `providers/src/error.rs` with unified `ProviderError` enum
2. Update `ClaudeProvider` to return typed errors
3. Update `OpenAIProvider` to return typed errors
4. Update `GeminiProvider` to return typed errors
5. Update `CodexProvider` to return typed errors
6. Update `ProviderManager` error handling
7. Update CLI to handle `ProviderError` variants
8. Add retry logic for `RateLimited` errors

**Success Criteria:**
- Typed errors enable smart error handling
- Rate limiting can trigger automatic retry
- Authentication errors surface clearly to user
- All tests pass

---

## Phase 3: Reduce Provider Duplication

### 3.1 Extract Common Provider Patterns

**Current State:**
~40% code duplication across providers:
- Auth detection logic repeated
- Model config parsing repeated
- Token extraction patterns repeated
- Streaming handling patterns repeated

**Target State:**
```rust
// providers/src/adapter.rs

/// Common functionality shared across all providers
pub trait ProviderAdapter: Send + Sync {
    /// Provider name for error messages
    fn name(&self) -> &'static str;

    /// Detect credentials from environment
    fn detect_credentials(&self) -> Result<Credentials, ProviderError>;

    /// Parse model string into config
    fn parse_model(&self, model: &str) -> ModelConfig;

    /// Extract token counts from response
    fn extract_tokens(&self, response: &serde_json::Value) -> TokenCounts;
}

/// Default implementations for common patterns
pub trait ProviderDefaults: ProviderAdapter {
    /// Standard auth detection from env vars
    fn detect_env_credential(&self, var_name: &str) -> Option<String> {
        std::env::var(var_name).ok()
    }

    /// Standard model parsing (provider/model format)
    fn parse_model_string(&self, model: &str) -> (Option<String>, String) {
        if let Some((provider, model)) = model.split_once('/') {
            (Some(provider.to_string()), model.to_string())
        } else {
            (None, model.to_string())
        }
    }
}

// Blanket implementation
impl<T: ProviderAdapter> ProviderDefaults for T {}
```

**Refactoring Steps:**

1. Create `providers/src/adapter.rs` with shared traits
2. Create `providers/src/types.rs` for shared types:
   - `Credentials` enum
   - `ModelConfig` struct
   - `TokenCounts` struct
3. Implement `ProviderAdapter` for `ClaudeProvider`
4. Implement `ProviderAdapter` for `OpenAIProvider`
5. Implement `ProviderAdapter` for `GeminiProvider`
6. Implement `ProviderAdapter` for `CodexProvider`
7. Extract duplicated streaming logic into shared helper
8. Extract token estimation logic into shared module
9. Remove duplicated code from individual providers

**Success Criteria:**
- Common patterns extracted to traits
- Provider implementations reduced by 30%+
- Adding new provider requires less boilerplate
- All tests pass

---

## Phase 4: Additional Improvements

### 4.1 Extract Token Estimation

**Current State:**
Magic number `4` appears in multiple places:
```rust
let estimated_tokens = text.len() / 4;
```

**Target State:**
```rust
// common/src/tokens.rs
pub struct TokenEstimator {
    bytes_per_token: usize,
}

impl TokenEstimator {
    pub const DEFAULT_BYTES_PER_TOKEN: usize = 4;

    pub fn estimate(&self, text: &str) -> usize {
        text.len() / self.bytes_per_token
    }

    pub fn estimate_messages(&self, messages: &[Message]) -> usize {
        // More accurate estimation for message format
    }
}
```

### 4.2 Create Configuration Layer

**Current State:**
Configuration detection scattered across modules.

**Target State:**
```rust
// common/src/config.rs
pub struct CodeletConfig {
    pub provider: ProviderConfig,
    pub tools: ToolsConfig,
    pub context: ContextConfig,
}

impl CodeletConfig {
    pub fn from_env() -> Result<Self>;
    pub fn from_file(path: &Path) -> Result<Self>;
    pub fn merge(self, other: Self) -> Self;
}
```

---

## Implementation Order

1. **Week 1**: Phase 1.1 - Split `interactive.rs`
2. **Week 2**: Phase 1.2 - Split `compaction.rs`
3. **Week 3**: Phase 1.3 - Split `debug_capture.rs`
4. **Week 4**: Phase 2.1 - Unified `ToolError`
5. **Week 5**: Phase 2.2 - Unified `ProviderError`
6. **Week 6**: Phase 3.1 - Provider adapter patterns
7. **Week 7**: Phase 4 - Token estimation and config layer

---

## Testing Strategy

### Unit Tests
- Each new module gets dedicated unit tests
- Test error type conversions
- Test adapter default implementations

### Integration Tests
- Existing CLI integration tests must pass
- Add tests for error handling paths
- Add tests for provider fallback scenarios

### Regression Tests
- Compare output before/after refactoring
- Ensure token counts match
- Verify streaming behavior unchanged

---

## Rollback Plan

Each phase can be rolled back independently:
1. Git branches per phase
2. Feature flags for new error types (if needed)
3. Backwards-compatible public APIs

---

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Largest file | 845 lines | < 250 lines |
| Provider duplication | ~40% | < 15% |
| Error type count | 8+ enums | 2 enums |
| Test coverage | TBD | Maintain or improve |

---

## Dependencies

- `thiserror` crate (already in workspace)
- No new external dependencies required

---

## Risks

| Risk | Mitigation |
|------|------------|
| Breaking public API | Keep old type aliases for transition period |
| Test coverage gaps | Add tests before refactoring each module |
| Merge conflicts | Work on one module at a time |
| Performance regression | Benchmark streaming before/after |
