# AST Research: Debug Capture Infrastructure Analysis

## Work Unit: CLI-022
## Date: 2025-12-04

## Summary

This research analyzed the codelet codebase to identify existing infrastructure that should be reused for implementing the `/debug` feature (one-for-one port from codelet).

---

## 1. Logging Infrastructure (src/logging/mod.rs)

### Key Functions
- `init_logging(verbose: bool)` - Initializes tracing with JSON formatting

### Reusable Patterns
- Uses `dirs::home_dir()` for cross-platform path: `~/.codelet/logs/`
- Directory creation: `std::fs::create_dir_all(&log_dir)`
- Daily file rotation with `tracing_appender::rolling::daily()`
- JSON formatting via `fmt::layer().json()`

### For /debug Implementation
- Reuse same `dirs::home_dir()` pattern for `~/.codelet/debug/`
- Use same `create_dir_all` pattern for directory creation
- Follow same tracing integration for log.entry events

---

## 2. Interactive CLI (src/cli/interactive.rs)

### Key Functions
- `run_interactive_mode()` - Main entry point (line 40)
- `repl_loop()` - Main REPL loop (line 74)
- `run_agent_with_interruption()` - Agent execution (line 144)
- `run_agent_stream_with_interruption()` - Stream handling (line 467)

### Command Handling Pattern (line 96)
```rust
if input.starts_with('/') {
    let provider = input.trim_start_matches('/');
    match session.switch_provider(provider) {
        Ok(()) => { ... }
        Err(e) => { ... }
    }
    continue;
}
```

### For /debug Implementation
- Add `/debug` command handling in repl_loop similar to provider switch
- Insert before provider switch check (line ~96)
- Pattern: `if input == "/debug" { handle_debug_command(); continue; }`

### Stream Events Available for Capture
- `MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text)` - text chunks
- `MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall)` - tool calls
- `MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult)` - tool results
- `MultiTurnStreamItem::FinalResponse` - completion with token usage

---

## 3. Provider Manager (src/providers/manager.rs)

### Key Functions
- `current_provider_name()` - Returns current provider as &str (line 124)
- `context_window()` - Returns provider's context window size (line 219)
- `get_prompt_prefix()` - Returns `[provider] > ` prefix (line 211)
- `switch_provider()` - Switches between providers (line 188)

### For /debug Implementation
- Use `session.provider_manager().current_provider_name()` for session metadata
- Use `session.provider_manager().context_window()` for context window info

---

## 4. Session Management (src/session/mod.rs)

### Key Structures
- `Session` - Main session state container
- `TokenTracker` - Tracks input/output/cache tokens

### Key Functions
- `new()` - Creates new session (line 56)
- `current_provider_name()` - Delegates to provider_manager (line 78)
- `switch_provider()` - Clears context and switches (line 91)
- `provider_manager()` / `provider_manager_mut()` - Accessors (lines 110, 115)
- `add_system_reminder()` - Adds system reminders (line 145)
- `compact_messages()` - Context compaction (line 172)
- `inject_context_reminders()` - Injects CLAUDE.md + env info (line 240)

### For /debug Implementation
- Access `session.messages` for message history
- Access `session.turns` for conversation turns
- Access `session.token_tracker` for token tracking
- Add debug capture state to Session or use separate singleton

---

## 5. Compaction Module (src/agent/compaction.rs)

### Key Structures
- `TokenTracker` - input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens
- `ConversationTurn` - user_message, tool_calls, tool_results, assistant_response, tokens, timestamp
- `ToolCall` - tool, id, input
- `ToolResult` - success, output
- `CompactionMetrics` - original_tokens, compacted_tokens, compression_ratio

### Key Functions
- `effective_tokens()` - Calculates effective tokens with cache discount (line 54)
- `total_tokens()` - Returns total input + output tokens (line 61)

### For /debug Implementation
- Capture `ConversationTurn` data for turn tracking
- Use token tracking for session statistics
- Capture compaction events when `compact_messages()` is called

---

## Architecture Recommendations

### 1. New Module: src/debug_capture.rs

Create singleton manager mirroring codelet's debug-capture.ts:

```rust
pub struct DebugCaptureManager {
    enabled: bool,
    session_id: Option<String>,
    session_file: Option<PathBuf>,
    sequence: u32,
    turn_id: u32,
    start_time: Option<Instant>,
    event_count: u32,
    error_count: u32,
    warning_count: u32,
    session_metadata: SessionMetadata,
}
```

### 2. Command Integration (src/cli/interactive.rs)

Add before provider switch check in repl_loop:

```rust
// Handle /debug command
if input == "/debug" {
    let result = handle_debug_command();
    if result.enabled {
        // Set session metadata
        let manager = get_debug_capture_manager();
        manager.set_session_metadata(SessionMetadata {
            provider: Some(session.current_provider_name().to_string()),
            model: Some(get_model_name()),
            context_window: Some(session.provider_manager().context_window()),
            ..Default::default()
        });
    }
    println!("{}", result.message);
    continue;
}
```

### 3. Event Capture Points

| Event Type | Location | Integration Point |
|------------|----------|-------------------|
| session.start | debug_capture.rs | When /debug enables capture |
| session.end | debug_capture.rs | When /debug disables capture |
| api.request | Need to add wrapper | Before LLM API call |
| api.response.* | interactive.rs:467 | In stream handler |
| tool.call | interactive.rs:262 | In handle_tool_call |
| tool.result | interactive.rs:337 | In handle_tool_result |
| log.entry | logging/mod.rs | Add debug capture layer |
| user.input | interactive.rs:483 | When user message added |
| token.update | interactive.rs:553 | In token tracking |
| context.update | session/mod.rs | In add_system_reminder |
| compaction.triggered | session/mod.rs:172 | In compact_messages |
| provider.switch | interactive.rs:96 | In provider switch |
| command.executed | To be added | After command handling |

### 4. File I/O Patterns

Reuse from logging/mod.rs:
- `dirs::home_dir()` for cross-platform paths
- `std::fs::create_dir_all()` for directory creation
- File permissions via `std::fs::set_permissions()` with mode 0o700

### 5. Credential Redaction

Implement in debug_capture.rs:
```rust
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "anthropic-api-key",
    "openai-api-key",
    "api-key",
];

fn sanitize_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers.iter()
        .map(|(k, v)| {
            if SENSITIVE_HEADERS.contains(&k.to_lowercase().as_str()) {
                (k.clone(), "[REDACTED]".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}
```

---

## DRY/SOLID Compliance Checklist

- [ ] Reuse `dirs::home_dir()` from logging module
- [ ] Reuse `create_dir_all` pattern from logging module
- [ ] Reuse `Session` access patterns for messages/turns/tokens
- [ ] Reuse `ProviderManager` for provider/model metadata
- [ ] Follow command handling pattern from provider switch
- [ ] Follow event handling pattern from stream processor
- [ ] Integrate with existing tracing infrastructure
