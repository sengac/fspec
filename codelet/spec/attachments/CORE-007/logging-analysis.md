# Logging System Analysis for CORE-007

## Current State Analysis

### Existing Logging Setup
- **Location**: src/main.rs:12-17
- **Current Implementation**:
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(
          tracing_subscriber::EnvFilter::from_default_env()
              .add_directive(tracing::Level::INFO.into()),
      )
      .init();
  ```
- **Issues**:
  - No file-based logging (only stderr output)
  - No rotation configured
  - No JSON formatting for machine parsing
  - No structured metadata support

### Code Locations Requiring Updates

#### 1. eprintln! Replacements
- **src/providers/claude.rs:314**
  - Current: `eprintln!("Unknown stop_reason from Anthropic API: {}", other);`
  - Target: `warn!(stop_reason = %other, "Unknown stop_reason from Anthropic API");`

- **src/cli/mod.rs:124**
  - Current: `eprintln!("\nError: {}", e);`
  - Target: `error!(error = %e, "Agent execution failed");`

#### 2. User-Facing println! (PRESERVE)
- **src/cli/mod.rs:71** - Config path display (KEEP)
- **src/cli/mod.rs:76** - Config directory error (KEEP)
- **src/cli/mod.rs:85** - "Interactive mode not yet implemented" (KEEP)
- **src/cli/mod.rs:130** - Final newline (KEEP)

#### 3. Agent Execution Logging
- **src/agent/rig_agent.rs:55-64** - RigAgent::prompt()
  - Add: `info!(prompt = %prompt, "Starting agent execution");` at start
  - Add: `info!(response_length = response.len(), "Agent execution completed");` at end

- **src/agent/rig_agent.rs:73-87** - RigAgent::prompt_streaming()
  - Add: `info!(prompt = %prompt, "Starting streaming agent execution");` at start
  - Add: `info!("Streaming agent execution completed");` at end

#### 4. Tool Execution Logging
All tools need `debug!(tool = "ToolName", input = ?args, "Executing tool");` at start of execute():

- src/tools/read.rs - ReadTool::execute()
- src/tools/write.rs - WriteTool::execute()
- src/tools/bash.rs - BashTool::execute()
- src/tools/grep.rs - GrepTool::execute()
- src/tools/glob.rs - GlobTool::execute()
- src/tools/edit.rs - EditTool::execute()
- src/tools/astgrep.rs - AstGrepTool::execute()

## Implementation Plan

### 1. Create Logging Module
**File**: src/logging/mod.rs
```rust
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling;
use std::path::PathBuf;

pub fn init_logging(verbose: bool) -> anyhow::Result<()> {
    let log_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".codelet")
        .join("logs");

    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(log_dir, "codelet");

    let env_filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into())
    };

    tracing_subscriber::registry()
        .with(fmt::layer().json().with_writer(file_appender))
        .with(env_filter)
        .init();

    Ok(())
}
```

### 2. Update main.rs
Replace lines 12-17 with:
```rust
codelet::logging::init_logging(false)?;
```

### 3. Update CLI to support --verbose
Add verbose flag handling to init_logging() call based on CLI args.

## Codelet Winston Equivalents

| Codelet (Winston) | codelet (Tracing) |
|-------------------|---------------------|
| `logger.error(msg, { metadata })` | `error!(key = value, "msg")` |
| `logger.warn(msg, { metadata })` | `warn!(key = value, "msg")` |
| `logger.info(msg, { metadata })` | `info!(key = value, "msg")` |
| `logger.debug(msg, { metadata })` | `debug!(key = value, "msg")` |
| `~/.codelet/logs/app-%DATE%.log` | `~/.codelet/logs/codelet-YYYY-MM-DD` |
| DailyRotateFile | `rolling::daily()` |
| JSON format | `fmt::layer().json()` |
| DEBUG env var | `RUST_LOG=debug` |
| --debug flag | `--verbose` flag |

## Dependencies
All required dependencies already in Cargo.toml:
- ✅ tracing = "0.1"
- ✅ tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
- ✅ tracing-appender = "0.2"
