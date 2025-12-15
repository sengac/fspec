# AST Research: Logging Implementation Points

## AST Analysis Results

### 1. Current Logging Initialization (src/main.rs)
**Lines 12-17**:
```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    )
    .init();
```

**Analysis**: Simple fmt() setup, no file output, no JSON, no rotation.

### 2. eprintln! Usage in src/providers/claude.rs
**Line 314**:
```rust
eprintln!("Unknown stop_reason from Anthropic API: {}", other);
```

**Context**: Inside match expression for response.raw_response.stop_reason
**Function**: Part of Anthropic API response handling
**Replacement Target**: `warn!(stop_reason = %other, "Unknown stop_reason from Anthropic API");`

### 3. eprintln! Usage in src/cli/mod.rs
**Line 124**:
```rust
eprintln!("\nError: {}", e);
```

**Context**: Error handling in run_agent() function
**Function**: Displays agent execution errors to user
**Replacement Target**: `error!(error = %e, "Agent execution failed");`

### 4. println! Usage in src/cli/mod.rs (PRESERVE)
**Line 71-76**: Config path display (user-facing output)
**Line 85**: "Interactive mode not yet implemented" (user-facing message)
**Line 130**: Final newline (formatting)

**Analysis**: These are intentional user output, not diagnostics. KEEP as println!

### 5. RigAgent Methods (src/agent/rig_agent.rs)
**Lines 55-64**: `RigAgent::prompt()` method
- Entry point for non-streaming execution
- Needs: `info!(prompt = %prompt, "Starting agent execution");`
- Needs: `info!(response_length = response.len(), "Agent execution completed");`

**Lines 74-87**: `RigAgent::prompt_streaming()` method
- Entry point for streaming execution
- Needs: `info!(prompt = %prompt, "Starting streaming agent execution");`
- Needs: `info!("Streaming agent execution completed");`

### 6. Tool Execute() Methods
All tools follow async trait pattern with `async fn execute(&self, args: Value) -> Result<ToolOutput>`:

1. **src/tools/read.rs:78** - ReadTool::execute()
2. **src/tools/write.rs:?** - WriteTool::execute()
3. **src/tools/bash.rs:74** - BashTool::execute()
4. **src/tools/grep.rs:?** - GrepTool::execute()
5. **src/tools/glob.rs:?** - GlobTool::execute()
6. **src/tools/edit.rs:?** - EditTool::execute()
7. **src/tools/astgrep.rs:?** - AstGrepTool::execute()

**Pattern**: Add `debug!(tool = "ToolName", input = ?args, "Executing tool");` at method start

## File Rotation Research (tracing-appender)

From Cargo.toml line 58:
```toml
tracing-appender = "0.2"  # Daily file rotation
```

**API**: `tracing_appender::rolling::daily(directory, file_name_prefix)`
- Creates files named: `{prefix}.{YYYY-MM-DD}`
- Example: `codelet.2024-12-02`
- Automatic rotation at midnight UTC
- Built-in cleanup mechanism (keeps N files)

## JSON Format Research (tracing-subscriber)

From Cargo.toml line 57:
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

**API**: `fmt::layer().json()`
- Outputs structured JSON logs
- Format: `{"timestamp":"...","level":"INFO","target":"codelet::module","fields":{...},"message":"..."}`
- Machine-parseable for analysis tools

## codelet Equivalence Mapping

Based on analysis of ~/projects/codelet/src/utils/logger.ts:

| codelet Pattern | codelet Pattern |
|-----------------|-------------------|
| `import { logger } from '../utils/logger'` | `use tracing::{info, debug, error, warn};` |
| `logger.error('msg', { key: val })` | `error!(key = val, "msg");` |
| `logger.info('Agent started')` | `info!("Agent started");` |
| `logger.debug('Tool', { toolName, input })` | `debug!(tool = %toolName, input = ?input, "Executing tool");` |
| `~/.codelet/logs/app-2024-12-02.log` | `~/.codelet/logs/codelet-2024-12-02` |
| Winston DailyRotateFile | tracing_appender::rolling::daily |

## Conclusion

All necessary points identified for logging implementation:
- ✅ Logging module location: src/logging/mod.rs (new file)
- ✅ 2 eprintln! replacements (claude.rs:314, cli.rs:124)
- ✅ 4 println! preservations (cli.rs:71,76,85,130)
- ✅ 2 agent methods needing logging (RigAgent::prompt, RigAgent::prompt_streaming)
- ✅ 7 tool execute() methods needing debug logging
- ✅ Dependencies already present in Cargo.toml
- ✅ Equivalence with codelet Winston pattern established
