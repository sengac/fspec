# Testing Guide

This document describes how we write and run tests in fspec. It covers our testing
philosophy, the Rust test framework, fixture infrastructure, and the concrete
patterns every contributor should follow.

---

## Table of Contents

1. [Philosophy](#philosophy)
2. [Running Tests](#running-tests)
3. [⚠️ Workspace Testing Rules — Read Before Running](#workspace-testing-rules--read-before-running)
4. [Test Pyramid](#test-pyramid)
5. [Test Helpers & Fixtures](#test-helpers--fixtures)
6. [ACDD Test Structure Convention](#acdd-test-structure-convention)
7. [Property-Based Testing (proptest)](#property-based-testing-proptest)
8. [Snapshot Testing (insta)](#snapshot-testing-insta)
9. [CLI Integration Testing (assert_cmd)](#cli-integration-testing-assert_cmd)
10. [Async Testing (tokio::test)](#async-testing-tokiotest)
11. [Troubleshooting](#troubleshooting)

---

## Philosophy

Our tests follow four guiding principles:

| Principle | In Practice |
|-----------|-------------|
| **Integration over mocks** | Prefer real filesystems, real stores, and real Tokio tasks. Mock only at system boundaries (network, external APIs). |
| **SOLID / DRY / Composable** | Every fixture module has a single domain. Base fixtures compose into richer ones. No copy-paste setup code. |
| **Redirect, don't intercept** | Control *inputs* (temp directories, `std::env::set_var`) rather than replacing *code paths*. |
| **ACDD compliance** | Every test file links back to a `.feature` file. |

> **"Don't mock what you can redirect."**
>
> By controlling filesystem contents and environment variables instead of
> intercepting module internals, our tests exercise the same code paths that run
> in production.

---

## Running Tests

| Command | What It Does |
|---------|-------------|
| `cargo test -p <crate>` | Run all tests for a specific crate |
| `cargo test -p <crate> --test <name>` | Run a specific integration test file |
| `cargo test -p <crate> -- <filter>` | Run tests matching a name filter |
| `cargo test --profile ci-test -p <crate>` | Run with the CI-optimized profile |

> **All tests run against compiled Rust, not source files.** `cargo test` compiles
> and runs in one step.

---

## ⚠️ Workspace Testing Rules — Read Before Running

> **NEVER run `cargo test --workspace` (or a bare `cargo test`) from `codelet/`.**
>
> **Incident 2026-07-10:** a plain `cargo test --workspace` compiled all 944
> integration-test binaries in the workspace with full DWARF debug info —
> **1.4–2 GB per binary**, because every test binary statically links the full
> crate graph (arrow, datafusion, lance, tantivy). `target/debug/deps` grew to
> **299 GB** and the machine crashed mid-link.

### Safe invocation patterns

| Goal | Command |
|------|---------|
| One crate's test target (preferred) | `cargo test -p <crate> --test <name>` |
| One whole crate | `cargo test -p <crate>` |
| Several crates | `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p <a> -p <b> -j 12 --no-fail-fast` |
| CI-style bounded run | `cargo test --profile ci-test -p <crate>` |

Rules of thumb:

1. **Always scope with `-p`** — never let cargo expand to the whole workspace.
2. **Drop debug info for broad runs** with
   `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` (both are needed:
   deps build under the `dev` profile, test binaries under `test`).
3. **Tee output to a file** (`cargo test ... 2>&1 | tee /tmp/test-out.txt`)
   and read the file — never re-run an expensive suite just to see a
   different slice of the output.
4. The disk-bloat background, the `ci-test` profile, and
   `incremental = false` on `[profile.test]` are documented in
   `codelet/Cargo.toml` (RPC-043 + the 2026-07-10 incident note).
5. These rules are enforced at runtime by `~/.fspec/blocklist.json`
   (`cargo-test-workspace-block` blocks `--workspace`/`--all`;
   `cargo-test-unscoped-prompt` prompts on a bare `cargo test`).

---

## Test Pyramid

```
┌──────────────────────────────────────────────────┐
│ E2E PTY Tests  (portable-pty + vt100)             │  Real PTY, real spawned
│ fspec binary scrollback parity                    │  fspec process, VT100 parser
├──────────────────────────────────────────────────┤
│ Integration Tests  (tests/*.rs)                   │  Real FS, real Tokio,
│ Real session creation, real RPC roundtrips         │  real file locking
├──────────────────────────────────────────────────┤
│ Property Tests  (proptest)                        │  Arbitrary inputs,
│ Parsing, serialization, edge cases               │  shrinking to minimal failures
├──────────────────────────────────────────────────┤
│ Snapshot Tests  (insta)                           │  YAML/json output snapshots,
│ Config serialization, CLI output shapes            │  reviewed diffs
├──────────────────────────────────────────────────┤
│ Unit Tests  (#[cfg(test)] modules)               │  Pure logic, builder fixtures,
│ Fast, isolated, typed                             │  in-memory data structures
├──────────────────────────────────────────────────┤
│ Shared Helpers  (codelet/test-helpers/)           │  Composable, lifecycle-managed,
│ Temp dirs, fixture builders                       │  auto-cleanup
└──────────────────────────────────────────────────┘
```

---

## Test Helpers & Fixtures

All test helpers live in `codelet/test-helpers/` and are consumed under
`[dev-dependencies]` by workspace crates.

### Temp Directories

```rust
use codelet_test_helpers::setup_temp_directory;

#[tokio::test]
async fn test_work_unit_creation() {
    let tmp_dir = setup_temp_directory();

    // Use tmp_dir.path() as project root
    let result = create_story(&CreateStoryArgs {
        prefix: "AUTH",
        title: "Login",
        project_root: tmp_dir.path(),
    }).await;

    assert!(result.is_ok());
}
// tmp_dir dropped at end of scope — directory cleaned up automatically
```

### Builder Fixtures

All fixture data follows the **builder pattern with `Partial<T>` override spreads**:

```rust
// Base builder — sensible defaults
pub fn create_test_model_info(overrides: &str) -> ModelInfo {
    let base = serde_json::json!({
        "id": "test-model",
        "name": "Test Model",
        "contextWindow": 128000
    });
    // Merge overrides...
}
```

### Fixture File Index

| File | Domain | Key Exports |
|------|--------|-------------|
| `temp_dir.rs` | OS temp dirs | `setup_temp_directory`, `TempDirectory` |
| `work_unit_fixtures.rs` | Work units | `create_test_work_unit`, `create_test_environment` |
| `foundation_fixtures.rs` | Foundation schema | `create_minimal_foundation`, `create_complete_foundation` |
| `session_fixtures.rs` | Sessions | `create_test_session_params`, `mock_provider_manager` |

---

## ACDD Test Structure Convention

Every test file **must** follow these conventions:

### 1. Feature File Doc Comment

```rust
/// Feature: spec/features/user-authentication.feature
///
/// This test validates the acceptance criteria defined in the feature file.
/// Scenarios map directly to Gherkin scenarios.
```

### 2. Test Names Mirror Gherkin Scenarios

```rust
/// Feature: spec/features/create-story.feature
#[tokio::test]
async fn scenario_create_story_with_valid_args() {
    // Given: clean temp directory
    let tmp_dir = setup_temp_directory();

    // When: create story command is called
    let result = create_story(&CreateStoryArgs {
        prefix: "AUTH",
        title: "User Login",
        project_root: tmp_dir.path(),
    }).await;

    // Then: work unit is created
    assert!(result.is_ok());
}
```

### 3. Step Comments in Tests

```rust
#[tokio::test]
async fn scenario_login_with_valid_credentials() {
    // Given I am on the login page
    let tmp_dir = setup_temp_directory();

    // When I enter valid credentials
    let result = authenticate(&tmp_dir, "user@example.com", "password123").await;

    // Then I should see the dashboard
    assert!(result.is_ok());
}
```

---

## Property-Based Testing (proptest)

Use `proptest` for parsing, serialization, and any logic where the input space
is large enough that example-based testing is insufficient.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn model_string_always_round_trips(input in "[a-z]+/[a-z]+-[0-9]+") {
        let parsed = parse_model_string(&input).unwrap();
        let reconstructed = format!("{}/{}", parsed.registry_provider, parsed.model_part);
        prop_assert_eq!(reconstructed, input);
    }
}
```

### When to Use proptest

- **Model string parsing** — any text parsing with multiple valid formats
- **JSON serialization** — round-trip tests for complex data structures
- **Path handling** — cross-platform path manipulation
- **Regex patterns** — verify patterns match expected inputs and reject unexpected ones

---

## Snapshot Testing (insta)

Use `insta` for tests where the output is a complex JSON/YAML structure that
would be tedious to assert field-by-field.

```rust
use insta::assert_yaml_snapshot;

#[test]
fn test_work_unit_export() {
    let work_units = create_test_work_units();
    let exported = export_work_units(&work_units);
    assert_yaml_snapshot!(exported);
}
```

### Snapshot Review Workflow

1. Run tests — new snapshots are saved as `.snap.new` files
2. Review the diff: `cargo insta review`
3. Accept or reject changes
4. Commit accepted snapshots alongside the code

---

## CLI Integration Testing (assert_cmd)

Test the `fspec` binary end-to-end using `assert_cmd` and `predicates`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn fspec_version_exits_zero() {
    Command::cargo_bin("fspec")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fspec"));
}

#[test]
fn fspec_unknown_command_exits_nonzero() {
    Command::cargo_bin("fspec")
        .unwrap()
        .arg("nonexistent-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}
```

---

## Async Testing (tokio::test)

All async tests use `#[tokio::test]`. Tests that mutate process-global state
must use `#[serial_test::serial]` to prevent race conditions:

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_blocklist_init() {
    // Mutates process-global BLOCKLIST_PROJECT_ROOT
    let result = build_service(&args).await;
    assert!(result.is_ok());
}
```

### Tokio Test Patterns

```rust
// Testing channels and concurrency
#[tokio::test]
async fn test_message_flow() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    tx.send("hello").await.unwrap();
    let msg = rx.recv().await.unwrap();
    assert_eq!(msg, "hello");
}

// Testing timeouts
#[tokio::test]
async fn test_hook_timeout() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        slow_hook(),
    ).await;

    assert!(result.is_err()); // Timeout occurred
}
```

---

## Forbidden Dependency Tests

The workspace enforces that certain crates must not depend on others (e.g.,
`codelet-fspec` must not depend on `codelet-napi`). These are verified by
integration tests in `tests/no_napi_dependency.rs`:

```rust
// This test fails to compile if codelet-fspec pulls in codelet-napi
#[test]
fn no_napi_dependency() {
    // The mere fact that this compiles proves the dependency is absent.
    // If codelet-fspec depended on codelet-napi, the workspace resolver
    // would include it in the dependency graph.
}
```

---

## Troubleshooting

### Tests hang or timeout

- Check for unawaited tasks — all `tokio::spawn` must be joined or explicitly ignored
- Ensure `tempfile` directories are dropped (use RAII, not manual cleanup)
- Use `#[serial]` for tests that mutate global state

### proptest failures

- Use `-- --nocapture` to see shrinking output: `cargo test -p <crate> --test <name> -- --nocapture`
- The shrinking output shows the minimal failing input

### Snapshot tests fail

- Run `cargo insta review` to see diffs and accept/reject changes
- Snapshots are stored in `snapshots/` directories alongside test files

### "Too many open files" error

- Reduce `-j` parallelism: `cargo test -p <crate> -j 4`
- Check for leaked file handles in test cleanup

### Serial test contention

- If tests share global state (e.g., `BLOCKLIST_PROJECT_ROOT`), always use `#[serial]`
- Consider restructuring to avoid shared mutable state
