# Agent Development Guidelines for fspec

This document provides guidelines for AI assistants working on the **fspec codebase**. This is about DEVELOPING fspec itself, not using it.

---

## Project Overview

**fspec** is a pure-Rust CLI tool for AI agents to manage Gherkin-based feature specifications and project work units using Acceptance Criteria Driven Development (ACDD).

- **Repository**: https://github.com/sengac/fspec
- **License**: MIT
- **Language**: Rust (Cargo workspace in `codelet/`)

For complete project context:
- **Project foundation**: [spec/FOUNDATION.md](spec/FOUNDATION.md)
- **Workflow**: run `fspec bootstrap` for complete details

---

## MANDATORY CODING STANDARDS - ZERO TOLERANCE

**ALL CODE MUST PASS QUALITY CHECKS BEFORE COMMITTING**

### CRITICAL DO NOT VIOLATIONS - CODE WILL BE REJECTED

#### Rust Violations:

- ❌ **NEVER** use `unwrap()` — use `expect("reason")` or proper error handling
- ❌ **NEVER** use `anyhow::Error` in public APIs — use `thiserror` derive macros
- ❌ **NEVER** use `println!` / `eprintln!` in production code — use `tracing` macros
- ❌ **NEVER** use `panic!()` — return errors instead
- ❌ **NEVER** use `unsafe` blocks — workspace denies `unsafe_code`
- ❌ **NEVER** use `dbg!()` in production code — workspace warns on `dbg_macro`
- ❌ **NEVER** use `todo!()` — workspace warns on `todo`

#### Clippy Violations (All Deny):

- ❌ **NEVER** use `expect_used` — use `expect("descriptive reason")`
- ❌ **NEVER** use `unwrap_used` — use `expect("descriptive reason")`
- ❌ **NEVER** use `panic` — return errors via `?`
- ❌ **NEVER** write manual implementations when std provides idiomatic alternatives:
  - `manual_clamp`, `manual_filter`, `manual_find`, `manual_flatten`
  - `manual_map`, `manual_memcpy`, `manual_non_exhaust`
  - `manual_ok_or`, `manual_range_contains`, `manual_retain`
  - `manual_strip`, `manual_try_fold`, `manual_unwrap_or`
- ❌ **NEVER** leave redundant clones, needless borrows, or unnecessary lazy evaluations

#### Error Handling (Required):

```rust
// ✅ CORRECT — thiserror derive + anyhow for internal
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FspecCoreError {
    #[error("work unit {0} not found")]
    WorkUnitNotFound(String),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

// ✅ CORRECT — propagate with ?
pub async fn run(args: &str) -> Result<String, FspecCoreError> {
    let data = parse_args(args)?;
    let result = process(data)?;
    Ok(result)
}

// ❌ WRONG — unwrap/expect without reason
let result = parse_args(args).unwrap();
```

#### Logging (Required):

```rust
// ✅ CORRECT — tracing macros
use tracing::{info, warn, error, debug};

info!("Processing work unit {}", id);
warn!("Deprecated command used: {}", cmd);
error!("Failed to load config: {err:?}");

// ❌ WRONG — println in production code
println!("Processing work unit {}", id);
```

---

## MANDATORY IMPLEMENTATION PATTERNS

### Two Front Doors, One Source of Truth

Every command in `codelet-fspec-core` exposes a single entry point:

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

This function is called by **both**:
1. The LLM-facing dispatcher (agent tool calls)
2. The standalone CLI (shell subcommands)

**NEVER duplicate business logic** between these two entry points.

### Async Runtime

- ✅ **ALWAYS** use `tokio` for async operations
- ✅ **ALWAYS** get runtime handle via `tokio::runtime::Handle::current()`
- ❌ **NEVER** create new `tokio::runtime::Builder` / `Runtime::new` instances

### File Organization

- **Keep files under 300 lines** — refactor when approaching this limit
- When a file exceeds 300 lines, stop and refactor BEFORE continuing
- Ask for approval before major refactoring

---

## Testing Requirements

> **📖 For the complete testing guide — runners, fixtures, patterns, and examples — see [TESTING.md](TESTING.md).**

### Critical Rules

- **Use Rust's built-in test framework** — `#[test]`, `#[tokio::test]`
- **Use `proptest` for property-based tests** — especially parsing and serialization logic
- **Use `insta` for snapshot tests** — YAML snapshots for complex output verification
- **Use `serial_test` for tests that mutate global state** — process-level singletons
- Write meaningful tests that verify actual functionality
- No trivial tests like `assert!(true)`
- **Test Coverage:** All new code must have corresponding unit tests

### Test Philosophy: Integration First, Mocks Last

- **Prefer integration tests** that use real filesystems (temp dirs), real stores
- **Redirect, don't intercept** — control inputs via temp directories rather than mocking code paths
- **Reuse shared helpers** from `codelet/test-helpers/` — never duplicate filesystem setup logic

### Test File Requirements

- ✅ **ALWAYS** create `tests/*.rs` integration test files or inline `#[cfg(test)]` modules
- ✅ **ALWAYS** run tests through `cargo test -p <crate> --test <name>`
- ✅ **ALWAYS** use helpers from `codelet/test-helpers/` for temp dirs and fixtures

### Test Naming Convention

```rust
// tests/rpc080_agent_loop_persistence.rs

/// Feature: spec/features/agent-loop-persistence.feature
#[tokio::test]
async fn scenario_agent_loop_saves_turns_to_disk() {
    // Given: fresh temp directory
    let tmp_dir = setup_temp_directory();

    // When: agent loop processes a turn
    let session = create_session(&tmp_dir).await;
    session.send_input("hello").await;

    // Then: turn is persisted to disk
    let turns = load_turns(&tmp_dir).await;
    assert!(turns.len() >= 1);
}
```

---

## Technology Stack

### Build System

- **Cargo**: Rust package manager and build system
- **Cargo workspace**: 25 crates in `codelet/`
- **rustup**: Rust toolchain manager

### Key Technologies

- **CLI Framework**: clap v4 (derive macros)
- **Terminal UI**: ratatui (TUI), crossterm (terminal events)
- **Async Runtime**: tokio
- **LLM Integration**: rig-core (patched via `patches/rig-core`)
- **RPC Framework**: tarpc (in-process + WebSocket transports)
- **HTTP Server**: axum (attachment viewer)
- **Gherkin Parser**: `gherkin` crate (pure Rust)
- **JSON Schema**: `jsonschema` crate
- **Logging**: tracing + tracing-subscriber + tracing-appender
- **Testing**: proptest, insta, serial_test, assert_cmd
- **Profiling**: pprof-rs (sampling profiler)

---

## Development Methodology: Acceptance Criteria Driven Development (ACDD)

This project uses **Acceptance Criteria Driven Development** where:

1. **Specifications come first** — We define acceptance criteria in Gherkin format (see spec/features/*.feature)
2. **Tests come second** — We write tests that directly map to scenarios BEFORE any code
3. **Code comes last** — We implement just enough code to make the tests pass

### CRITICAL RULES:

- **NEVER write production code without a failing test first**
- **Each Gherkin scenario must have corresponding tests**
- **Tests must map 1:1 to scenarios in feature files**
- **Feature files define acceptance criteria, NOT implementation details**

---

## Development Workflow

### 1. Before Making Changes

- Read the acceptance criteria in relevant .feature files (spec/features/*.feature)
- Check `spec/FOUNDATION.md` for project requirements
- Review `spec/TAGS.md` for available tags

### 2. When Writing Code (ACDD Process)

**CRITICAL**: Follow this exact order:

1. **Write feature file FIRST** in spec/features/ directory
   - Define acceptance criteria in Gherkin format
   - Include architecture notes in doc strings
   - Add proper tags (@phase, @component, @feature-group)
   - Format with `fspec format`
   - Validate with `fspec validate`

2. **Write tests SECOND** before any implementation
   - Map each scenario to test cases
   - Run tests and ensure they fail for the right reasons
   - Use descriptive test names matching scenarios

3. **Implement code LAST** to make tests pass
   - Write minimal code to pass tests
   - Refactor while keeping tests green
   - Follow existing patterns in the codebase

4. **Verify implementation**
   - Run `cargo check -p <crate>` to ensure Rust compiles
   - Run `cargo test -p <crate>` to ensure all tests pass
   - Run `fspec validate` to verify feature files
   - Run `fspec validate-tags` to verify tags are registered

### 3. Quality Check Integration

Run quality checks before committing:

```bash
cargo check -p codelet-fspec       # Check compilation
cargo clippy -p codelet-fspec      # Check clippy lints
cargo test -p codelet-fspec        # Run tests
```

**Code that violates workspace lints will be rejected by the compiler.**

---

## Common Build Commands

```bash
# Check compilation
cargo check -p codelet-fspec

# Run clippy
cargo clippy -p codelet-fspec

# Run tests for a specific crate
cargo test -p codelet-fspec

# Run a specific test file
cargo test -p codelet-fspec --test no_napi_dependency

# Build release binary
cargo build --profile release-slim -p codelet-fspec

# Run the binary directly
cargo run -p codelet-fspec -- --help

# Format code
cargo fmt -p codelet-fspec
```

---

## Workspace Testing Guidelines

### ⚠️ NEVER run unscoped `cargo test` or `cargo test --workspace`

A plain `cargo test --workspace` compiled all 944 integration-test binaries with full DWARF debug info (1.4–2 GB PER BINARY). The machine crashed mid-link.

**Safe invocation patterns:**

```bash
# 1. Scope by package + target (preferred):
cargo test -p codelet-fspec --test no_napi_dependency

# 2. Broader runs with explicit packages:
cargo test -p codelet-fspec -p codelet-fspec-core -j 12 --no-fail-fast

# 3. Use the dedicated profile:
cargo test --profile ci-test -p codelet-fspec
```

---

## Important Reminders

1. **Quality over Speed**: Take time to write proper types and error handling
2. **Ask Before Major Changes**: Propose refactoring before implementing
3. **Maintain Specifications**: Update feature files as code evolves
4. **Cross-Platform**: Always consider Windows path/shell differences
5. **No Shortcuts**: Fix issues properly, don't use `unwrap()` or disable linters
6. **No Unsafe**: The workspace denies `unsafe_code`

---

## When You Get Stuck

1. Check existing patterns in the codebase
2. Refer to `spec/FOUNDATION.md` for project goals
3. Run `fspec bootstrap` for fspec usage and workflow
4. Run tests to verify changes
5. Check feature files for acceptance criteria

---

## Contributing

When contributing to fspec:

1. Follow ACDD: Feature file → Tests → Implementation
2. Ensure all tests pass
3. Update relevant documentation
4. Follow the established patterns
5. Keep commits focused and descriptive
6. Update specifications when behavior changes
7. Register new tags using `fspec register-tag`

Remember: The goal is to create a CLI tool that helps AI agents manage Gherkin specifications and project work units using ACDD. Every line of code should contribute to this goal.
