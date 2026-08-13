# AST Research: Interactive TUI Integration Points

**Work Unit**: CLI-002  
**Date**: 2025-12-02  
**Purpose**: Identify integration points for interactive TUI in existing codelet codebase

---

## Research Method

Used `fspec research --tool=ast` to analyze existing Rust modules:
- `src/cli/mod.rs` - CLI entry point
- `src/agent/rig_agent.rs` - Agent execution
- `src/providers/manager.rs` - Provider management

---

## Findings: src/cli/mod.rs

### Current Structure
```rust
pub struct Cli {
    prompt: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    verbose: bool,
    command: Option<Commands>,
}

pub enum Commands {
    Exec { prompt: String },
    Completion { shell: Shell },
    Config { path: bool },
}

pub async fn run() -> Result<()>
async fn run_agent(prompt: &str, provider_name: Option<&str>) -> Result<()>
async fn run_agent_stream<M>(agent: RigAgent<M>, prompt: &str) -> Result<()>
```

### Integration Point for Interactive Mode

**Current code** (line 81-89):
```rust
None => {
    if let Some(prompt) = cli.prompt {
        run_agent(&prompt, cli.provider.as_deref()).await
    } else {
        println!("Interactive mode not yet implemented");
        Ok(())
    }
}
```

**Change needed**: Replace placeholder with call to new interactive module:
```rust
None => {
    if let Some(prompt) = cli.prompt {
        run_agent(&prompt, cli.provider.as_deref()).await
    } else {
        // NEW: Run interactive TUI mode
        interactive::run_interactive_mode(cli.provider.as_deref()).await
    }
}
```

---

## Findings: src/agent/rig_agent.rs

### Available Methods
```rust
impl<M: CompletionModel> RigAgent<M> {
    pub fn new(agent: Agent<M>, max_depth: usize) -> Self
    pub fn with_default_depth(agent: Agent<M>) -> Self
    pub fn max_depth(&self) -> usize
    
    // Non-streaming (for simple prompts)
    pub async fn prompt(&self, prompt: &str) -> Result<String>
    
    // Streaming (for interactive mode - THIS IS WHAT WE NEED)
    pub async fn prompt_streaming(&self, prompt: &str) 
        -> impl Stream<Item = Result<MultiTurnStreamItem<M::StreamingResponse>>>
}
```

### Integration Pattern

**For interactive TUI**, use `prompt_streaming()`:
```rust
let agent = RigAgent::with_default_depth(rig_agent);
let mut stream = agent.prompt_streaming(user_input).await;

// Coordinate with terminal events using tokio::select!
loop {
    select! {
        Some(chunk) = stream.next() => {
            if !is_interrupted.load(Ordering::Relaxed) {
                process_chunk(chunk);
            } else {
                break; // Interrupt agent
            }
        }
        Some(event) = tui_events.next() => {
            handle_terminal_event(event);
        }
    }
}
```

**MultiTurnStreamItem types** (from existing code):
- `StreamAssistantItem(Text)` - LLM text output
- `StreamAssistantItem(ToolCall)` - Tool execution
- `StreamUserItem` - Tool results sent back
- `FinalResponse` - Conversation complete

---

## Findings: src/providers/manager.rs

### Current Methods
```rust
impl ProviderManager {
    pub fn new() -> Result<Self>
    pub fn with_provider(name: &str) -> Result<Self>
    pub fn current_provider_name(&self) -> &str
    
    // Provider getters
    pub fn get_claude(&self) -> Result<ClaudeProvider>
    pub fn get_openai(&self) -> Result<OpenAIProvider>
    pub fn get_codex(&self) -> Result<CodexProvider>
    pub fn get_gemini(&self) -> Result<GeminiProvider>
}
```

### Missing Methods (Need to Add)

For startup card display:
```rust
impl ProviderManager {
    // NEW: Check if any provider is configured
    pub fn has_any_provider(&self) -> bool {
        self.has_claude() || self.has_openai() || 
        self.has_gemini() || self.has_codex()
    }
    
    // NEW: List all available providers for startup card
    pub fn list_available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.has_claude() { providers.push("Claude (/claude)".to_string()); }
        if self.has_openai() { providers.push("OpenAI (/openai)".to_string()); }
        if self.has_gemini() { providers.push("Gemini (/gemini)".to_string()); }
        if self.has_codex() { providers.push("Codex (/codex)".to_string()); }
        providers
    }
    
    // NEW: Switch provider mid-session
    pub fn switch_provider(&mut self, name: &str) -> Result<()> {
        // Validate provider exists, update internal state
        // Return error if provider not configured
    }
}
```

---

## Integration Architecture

### New Module Structure

```
src/
├── cli/
│   ├── mod.rs              (existing - modify)
│   └── interactive.rs      (NEW - create)
├── agent/
│   ├── mod.rs
│   └── rig_agent.rs        (existing - no changes)
├── providers/
│   ├── mod.rs
│   └── manager.rs          (existing - extend with new methods)
└── tui/                    (NEW - create bounded context)
    ├── mod.rs
    ├── terminal.rs         (terminal state management)
    ├── events.rs           (TuiEvent enum, event stream)
    ├── status.rs           (status display widget)
    └── input_queue.rs      (input queue management)
```

### Module Responsibilities

**src/cli/interactive.rs** (public interface):
```rust
pub async fn run_interactive_mode(provider: Option<&str>) -> Result<()> {
    // 1. Initialize TUI (src/tui)
    // 2. Display startup card
    // 3. Run main event loop (tokio::select!)
    // 4. Clean up on exit
}
```

**src/tui/** (internal implementation):
- `terminal.rs`: Raw mode, panic hook, keyboard enhancements
- `events.rs`: Unified event stream (terminal + draw requests)
- `status.rs`: Status display rendering
- `input_queue.rs`: Async input queue (mpsc channel)

---

## Dependencies to Add

```toml
[dependencies]
# Existing
rig-core = "..."
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# NEW for TUI
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
tokio-stream = "0.1"
async-stream = "0.3"
```

---

## Testing Strategy

### Integration Points to Test

1. **CLI dispatch** (`src/cli/mod.rs`):
   - Test: Running `codelet` without args calls `interactive::run_interactive_mode()`
   - Test: Provider flag passed correctly to interactive mode

2. **Provider switching** (`src/providers/manager.rs`):
   - Test: `has_any_provider()` returns true when credentials configured
   - Test: `list_available_providers()` returns correct list
   - Test: `switch_provider()` updates active provider

3. **Agent streaming** (integration with `RigAgent`):
   - Test: `prompt_streaming()` yields MultiTurnStreamItem events
   - Test: Breaking stream mid-execution preserves partial results
   - Test: Tool calls complete atomically

4. **Terminal state** (`src/tui/terminal.rs`):
   - Test: Raw mode enabled on start, disabled on exit
   - Test: Panic hook restores terminal state
   - Test: Keyboard enhancements gracefully degrade

5. **Event coordination** (`src/tui/events.rs`):
   - Test: ESC key during streaming sets interrupt flag
   - Test: Input queue persists across interruption
   - Test: Status updates don't block streaming

---

## Conclusion

The existing codebase is **well-structured for adding interactive TUI**:

✅ **Minimal changes needed**:
- Modify `src/cli/mod.rs` (1 line change + new import)
- Extend `src/providers/manager.rs` (3 new methods)
- Create new `src/cli/interactive.rs` + `src/tui/` modules

✅ **Clean boundaries**:
- TUI is isolated bounded context
- Reuses existing `RigAgent` and `ProviderManager`
- No changes to agent execution or provider logic

✅ **ACDD-friendly**:
- Each scenario maps to testable function
- Integration points are narrow and well-defined
- Panic hook ensures terminal restoration in all cases

**Next step**: Write tests for 7 scenarios in feature file, starting with simplest (startup card).
