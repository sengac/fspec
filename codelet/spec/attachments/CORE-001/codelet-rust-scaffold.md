# Codelet Rust Port - Project Scaffold

## Executive Summary

This document outlines the complete scaffold structure for porting codelet from TypeScript to Rust. The scaffold is organized around the five bounded contexts identified in the Foundation Event Storm:

1. **CLI Interface** - Command parsing, configuration, entry points
2. **Provider Management** - Multi-provider LLM abstraction via Rig.rs
3. **Tool Execution** - File operations, code search, bash execution
4. **Context Management** - Token tracking, compaction, prompt caching
5. **Agent Execution** - Main runner loop, message history, streaming

---

## Source Project Analysis

### Codelet TypeScript Structure

```
~/projects/codelet/src/
├── index.ts              # CLI entry point
├── cli.ts                # Commander.js CLI factory
├── cli-utils.ts          # Version reading utilities
├── startup-card.ts       # Welcome display
├── agent/
│   ├── runner.ts         # Main agent loop (1,259 lines)
│   ├── tools.ts          # Tool implementations (858 lines)
│   ├── tool-definitions.ts
│   ├── provider-manager.ts (551 lines)
│   ├── claude-provider.ts
│   ├── openai-provider.ts
│   ├── codex-provider.ts
│   ├── gemini-provider.ts
│   ├── context.ts
│   ├── compaction.ts
│   ├── anchor-point-compaction.ts
│   ├── token-tracker.ts
│   ├── token-state-manager.ts
│   ├── system-prompt.ts
│   ├── system-prompt-state.ts
│   ├── input-manager.ts
│   ├── model-limits.ts
│   ├── llm-summary-provider.ts
│   └── ripgrep.ts
├── commands/
│   └── agent.ts
├── components/           # Ink/React UI (not porting)
└── utils/
    ├── logger.ts
    ├── prompt-cache.ts
    ├── debug-capture.ts
    ├── system-reminder-parser.ts
    └── bash-truncation-handler.ts
```

---

## Rust Project Scaffold

### Directory Structure

```
/home/rquast/projects/codelet/
├── Cargo.toml                    # Workspace manifest
├── src/
│   ├── main.rs                   # Entry point
│   ├── lib.rs                    # Library root
│   │
│   ├── cli/                      # CLI Interface bounded context
│   │   ├── mod.rs
│   │   ├── commands.rs           # Subcommand definitions
│   │   ├── config.rs             # Configuration loading
│   │   ├── config_override.rs    # -c key=value system
│   │   └── startup.rs            # Welcome card display
│   │
│   ├── providers/                # Provider Management bounded context
│   │   ├── mod.rs                # Provider trait + manager
│   │   ├── anthropic.rs          # Claude via Rig.rs
│   │   ├── openai.rs             # GPT-4 via Rig.rs
│   │   ├── google.rs             # Gemini via Rig.rs
│   │   ├── model_limits.rs       # Context window database
│   │   └── credentials.rs        # API key detection
│   │
│   ├── tools/                    # Tool Execution bounded context
│   │   ├── mod.rs                # Tool trait + registry
│   │   ├── definition.rs         # Tool schemas for LLM
│   │   ├── bash.rs               # Command execution
│   │   ├── read.rs               # File reading
│   │   ├── write.rs              # File writing
│   │   ├── edit.rs               # File editing
│   │   ├── grep.rs               # Ripgrep integration
│   │   ├── glob.rs               # File pattern matching
│   │   ├── ls.rs                 # Directory listing
│   │   └── ast_grep.rs           # AST-based search (optional)
│   │
│   ├── context/                  # Context Management bounded context
│   │   ├── mod.rs
│   │   ├── token_tracker.rs      # Token counting + cache metrics
│   │   ├── token_state.rs        # Warning levels + state management
│   │   ├── compaction.rs         # Compaction trigger logic
│   │   ├── anchor_point.rs       # Anchor point detection algorithm
│   │   ├── prompt_cache.rs       # Anthropic cache boundary tracking
│   │   ├── project_context.rs    # CLAUDE.md/AGENTS.md discovery
│   │   └── system_reminder.rs    # Deduplication system
│   │
│   ├── agent/                    # Agent Execution bounded context
│   │   ├── mod.rs
│   │   ├── runner.rs             # Main execution loop
│   │   ├── message.rs            # Message types + history
│   │   ├── streaming.rs          # Response stream handling
│   │   ├── tool_executor.rs      # Tool call → result pipeline
│   │   ├── input_manager.rs      # Terminal input + interrupts
│   │   └── system_prompt.rs      # Provider-aware prompts
│   │
│   └── utils/                    # Shared utilities
│       ├── mod.rs
│       ├── logger.rs             # Tracing setup
│       └── truncation.rs         # Output truncation helpers
│
├── tests/                        # Integration tests
│   ├── cli_test.rs
│   ├── tools_test.rs
│   └── integration_test.rs
│
├── benches/                      # Benchmarks
│   └── token_tracking.rs
│
└── spec/                         # fspec specifications
    ├── features/
    └── ...
```

---

## Module Specifications

### 1. CLI Interface (`src/cli/`)

**Purpose:** Parse command-line arguments, load configuration, dispatch to agent.

**Key Types:**
```rust
// src/cli/mod.rs
pub struct Cli {
    pub config_overrides: CliConfigOverrides,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub verbose: bool,
    pub quiet: bool,
    pub output: OutputFormat,
    pub command: Option<Commands>,
}

pub enum Commands {
    Exec(ExecArgs),
    Completion(CompletionArgs),
    Config(ConfigArgs),
    Resume(ResumeArgs),
}

pub enum OutputFormat {
    Text,
    Json,
}
```

**Key Functions:**
```rust
pub async fn run() -> Result<()>;
pub fn parse_args() -> Cli;
fn generate_completions(shell: Shell);
```

**Rust Crates:**
- `clap` v4 with derive macros
- `clap_complete` for shell completions

---

### 2. Provider Management (`src/providers/`)

**Purpose:** Abstract multiple LLM providers behind a unified trait.

**Key Types:**
```rust
// src/providers/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Google,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;

    async fn complete(&self, messages: &[Message]) -> Result<CompletionStream>;
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionStream>;

    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;
}

pub struct ProviderManager {
    active_provider: ProviderType,
    anthropic: Option<AnthropicProvider>,
    openai: Option<OpenAIProvider>,
    google: Option<GoogleProvider>,
}

impl ProviderManager {
    pub async fn new() -> Result<Self>;
    pub fn current(&self) -> &dyn LlmProvider;
    pub async fn switch(&mut self, provider: ProviderType) -> Result<()>;
    pub fn available_providers(&self) -> Vec<ProviderType>;
}
```

**Model Limits Database:**
```rust
// src/providers/model_limits.rs
pub struct ModelLimits {
    pub context_window: usize,
    pub max_output_tokens: usize,
}

pub fn get_limits(model: &str) -> ModelLimits {
    match model {
        "claude-sonnet-4-20250514" => ModelLimits { context_window: 200_000, max_output_tokens: 16_384 },
        "gpt-4o" => ModelLimits { context_window: 128_000, max_output_tokens: 4_096 },
        "gemini-2.0-flash" => ModelLimits { context_window: 1_000_000, max_output_tokens: 8_192 },
        _ => ModelLimits { context_window: 100_000, max_output_tokens: 4_096 },
    }
}
```

**Rust Crates:**
- `rig-core` for LLM provider abstraction
- `async-trait` for async trait methods
- `reqwest` for HTTP (fallback)

---

### 3. Tool Execution (`src/tools/`)

**Purpose:** Execute file operations, code search, and shell commands.

**Key Types:**
```rust
// src/tools/mod.rs
pub const MAX_OUTPUT_LENGTH: usize = 30_000;
pub const MAX_LINE_LENGTH: usize = 2_000;
pub const DEFAULT_LINE_LIMIT: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub truncated: bool,
    pub is_error: bool,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &ToolParameters;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}
```

**Tool Implementations:**

| Tool | Input | Output | Notes |
|------|-------|--------|-------|
| Bash | `command: String, timeout: u64` | stdout/stderr | Truncate at 30KB |
| Read | `file_path, offset?, limit?` | Line-numbered content | cat -n style |
| Write | `file_path, content` | Success message | Create dirs |
| Edit | `file_path, old_string, new_string` | Success/error | Exact match required |
| Grep | `pattern, path?, glob?, context?` | Matches with line numbers | Via ripgrep |
| Glob | `pattern, path?` | File list | Respects .gitignore |
| LS | `path` | Directory listing | Type indicators |

**Rust Crates:**
- `tokio::process` for Bash
- `ignore` for gitignore-aware walking
- `which` for finding ripgrep

---

### 4. Context Management (`src/context/`)

**Purpose:** Track token usage, trigger compaction, manage prompt caching.

**Key Types:**
```rust
// src/context/token_tracker.rs
#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub request_count: u64,
}

impl TokenTracker {
    pub fn effective_tokens(&self) -> u64 {
        // Cache reads get 90% discount
        self.input_tokens - (self.cache_read_tokens * 9 / 10)
    }
}

// src/context/token_state.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    None,       // < 80%
    Approaching, // 80-90%
    Critical,   // 90-95%
    Emergency,  // > 95%
}

pub struct TokenState {
    pub current_tokens: u64,
    pub total_capacity: u64,
    pub warning_level: WarningLevel,
}

// src/context/anchor_point.rs
#[derive(Debug, Clone)]
pub struct AnchorPoint {
    pub turn_index: usize,
    pub anchor_type: AnchorType,
    pub weight: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum AnchorType {
    TaskCompletion,
    ErrorResolution,
    UserCheckpoint,
    FeatureMilestone,
}

pub fn detect_anchor_points(turns: &[ConversationTurn]) -> Vec<AnchorPoint>;
pub fn select_compaction_boundary(anchors: &[AnchorPoint], budget: usize) -> usize;
```

**Compaction Algorithm:**
1. Convert message history to conversation turns
2. Detect anchor points (task completion, error resolution, etc.)
3. Calculate token budget (context_window - 50K buffer)
4. Select turns to preserve based on anchor weights
5. Generate LLM summary of dropped turns
6. Inject summary as continuation message
7. Reset token tracker

**Rust Crates:**
- Standard library types
- `chrono` for timestamps

---

### 5. Agent Execution (`src/agent/`)

**Purpose:** Main execution loop, message handling, streaming.

**Key Types:**
```rust
// src/agent/message.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_call")]
    ToolCall { id: String, name: String, arguments: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_call_id: String, content: String, is_error: bool },
}

// src/agent/runner.rs
pub struct Runner {
    provider: ProviderManager,
    tools: ToolRegistry,
    messages: Vec<Message>,
    token_tracker: TokenTracker,
    input_manager: InputManager,
    config: Config,
}

impl Runner {
    pub async fn new(config: Config) -> Result<Self>;
    pub async fn run(&mut self) -> Result<()>;
    async fn process_prompt(&mut self, prompt: &str) -> Result<()>;
    async fn execute_tool_calls(&mut self, calls: Vec<ToolCall>) -> Vec<ToolResult>;
    fn should_compact(&self) -> bool;
    async fn perform_compaction(&mut self) -> Result<()>;
}

// src/agent/input_manager.rs
pub struct InputManager {
    state: InputState,
}

pub enum InputState {
    Idle,
    Running,
    Paused,
    Interrupted,
}

impl InputManager {
    pub fn new() -> Self;
    pub async fn read_line(&mut self, prompt: &str) -> Result<Option<String>>;
    pub fn check_interrupt(&mut self) -> bool;
}
```

**Main Loop Flow:**
```
1. Display welcome card
2. Initialize provider manager
3. Load project context (CLAUDE.md)
4. Enter REPL loop:
   a. Display prompt (e.g., "[claude] > ")
   b. Read user input
   c. Check for commands (/claude, /openai, etc.)
   d. Add user message to history
   e. Inject system reminders (token state, context)
   f. Send to LLM with tools
   g. Stream response
   h. If tool calls: execute tools, add results, continue
   i. If text response: display and loop
   j. Check compaction threshold
   k. Repeat
```

**Rust Crates:**
- `tokio` for async runtime
- `crossterm` for terminal control
- `futures` for stream handling

---

## Data Flow Diagrams

### Message Flow
```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│   User      │───▶│   Runner     │───▶│  Provider   │
│   Input     │    │  (history)   │    │  (LLM API)  │
└─────────────┘    └──────────────┘    └─────────────┘
                          │                   │
                          │                   ▼
                          │            ┌─────────────┐
                          │            │  Streaming  │
                          │            │  Response   │
                          │            └─────────────┘
                          │                   │
                          ▼                   ▼
                   ┌──────────────┐    ┌─────────────┐
                   │    Tool      │◀───│ Tool Calls  │
                   │   Registry   │    │  (parsed)   │
                   └──────────────┘    └─────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │ Tool Results │
                   │ (→ history)  │
                   └──────────────┘
```

### Token Management Flow
```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│  API Call   │───▶│   Token      │───▶│  Token      │
│  Response   │    │   Tracker    │    │   State     │
└─────────────┘    └──────────────┘    └─────────────┘
                          │                   │
                          ▼                   ▼
                   ┌──────────────┐    ┌─────────────┐
                   │  Effective   │    │  Warning    │
                   │   Tokens     │    │   Level     │
                   └──────────────┘    └─────────────┘
                          │                   │
                          ▼                   ▼
                   ┌──────────────┐    ┌─────────────┐
                   │  Compaction  │    │   System    │
                   │   Check      │    │  Reminder   │
                   └──────────────┘    └─────────────┘
```

---

## Dependency Mapping

| TypeScript Package | Rust Equivalent | Notes |
|--------------------|-----------------|-------|
| `ai` (Vercel AI SDK) | `rig-core` | Unified LLM abstraction |
| `@ai-sdk/anthropic` | `rig-core` anthropic feature | Built into Rig |
| `@ai-sdk/openai` | `rig-core` openai feature | Built into Rig |
| `@ai-sdk/google` | `rig-core` google feature | Built into Rig |
| `commander` | `clap` v4 | CLI parsing |
| `zod` | `serde` + custom validation | Schema validation |
| `chalk` | `owo-colors` | Terminal colors |
| `winston` | `tracing` + `tracing-appender` | Logging with rotation |
| `ink` / `react` | `ratatui` (optional) | TUI framework |
| `dotenv` | `dotenv` | Env loading |

---

## Implementation Phases

### Phase 1: Core Infrastructure ✓
- [x] Project initialization with Cargo.toml
- [x] Basic CLI structure with clap
- [x] Configuration loading
- [x] Logging setup with tracing

### Phase 2: Tool Execution
- [ ] Tool trait and registry
- [ ] Bash tool with timeout
- [ ] Read tool with line numbers
- [ ] Write tool with directory creation
- [ ] Edit tool with exact matching
- [ ] Grep tool via ripgrep
- [ ] Glob tool with gitignore

### Phase 3: Provider Management
- [ ] Provider trait definition
- [ ] Rig.rs integration
- [ ] Anthropic provider
- [ ] OpenAI provider
- [ ] Google provider
- [ ] Provider switching

### Phase 4: Context Management
- [ ] Token tracker
- [ ] Token state with warning levels
- [ ] Project context discovery
- [ ] System reminder handling
- [ ] Anchor point detection
- [ ] Compaction algorithm

### Phase 5: Agent Execution
- [ ] Message types
- [ ] Runner main loop
- [ ] Streaming response handling
- [ ] Tool execution pipeline
- [ ] Input manager with interrupts
- [ ] REPL interface

### Phase 6: Polish & Testing
- [ ] Integration tests
- [ ] Error handling refinement
- [ ] Performance optimization
- [ ] Documentation

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | Claude API key | - |
| `OPENAI_API_KEY` | OpenAI/GPT API key | - |
| `GOOGLE_GENERATIVE_AI_API_KEY` | Gemini API key | - |
| `CODELET_PROVIDER` | Default provider | anthropic |
| `CODELET_MODEL` | Default model | (provider default) |
| `CODELET_LOG_LEVEL` | Log level | info |
| `CODELET_HOME` | Config directory | ~/.config/codelet |

---

## Testing Strategy

### Unit Tests
- Token tracker calculations
- Anchor point detection
- Compaction boundary selection
- Tool output truncation
- Config override parsing

### Integration Tests
- CLI argument parsing
- Tool execution (with temp files)
- Provider initialization (mocked)
- Full agent loop (mocked LLM)

### Property-Based Tests
- Token estimation accuracy
- Compaction preserves anchor points
- Message serialization roundtrip

---

## Notes

1. **No UI Components**: Unlike codelet which uses Ink/React, codelet will use simple terminal output initially. TUI can be added later with ratatui.

2. **Rig.rs Abstraction**: Using Rig.rs provides a unified interface to multiple providers, similar to Vercel AI SDK.

3. **Streaming**: Response streaming will use Rust's async streams via `futures::Stream`.

4. **Tool Schemas**: Tools will be defined with Rust structs that serialize to JSON Schema for the LLM.

5. **Error Handling**: Using `anyhow` for application errors, `thiserror` for library errors.

6. **Async Runtime**: Tokio is the async runtime, matching Rig.rs requirements.
