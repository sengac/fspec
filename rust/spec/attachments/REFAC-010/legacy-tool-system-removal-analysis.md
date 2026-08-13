# Legacy Tool System Removal Analysis

## Executive Summary

The codelet Rust port contains a dual tool implementation that emerged from a partially completed migration to the `rig` framework. The production CLI exclusively uses `RigAgent` with `rig::tool::Tool`, making the legacy `Tool` trait, `ToolRegistry`, and `Runner` vestigial code that only exists to support tests.

**Recommendation**: Complete the migration by removing the legacy system entirely.

---

## Current Architecture

### Two Parallel Execution Paths

```
┌─────────────────────────────────────────────────────────────────┐
│                     PRODUCTION PATH (Active)                     │
├─────────────────────────────────────────────────────────────────┤
│  CLI (interactive.rs, lib.rs)                                   │
│       ↓                                                         │
│  ClaudeProvider::create_rig_agent()                             │
│       ↓                                                         │
│  rig::agent::Agent with .tool(ReadTool::new())                  │
│       ↓                                                         │
│  RigAgent::prompt_streaming()                                   │
│       ↓                                                         │
│  rig's internal tool handling                                   │
│       ↓                                                         │
│  impl rig::tool::Tool for ReadTool  →  call()                   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   LEGACY PATH (Tests Only)                       │
├─────────────────────────────────────────────────────────────────┤
│  Tests (bash_tool_test.rs, core_file_tools_test.rs, etc.)       │
│       ↓                                                         │
│  ToolRegistry::default()                                        │
│       ↓                                                         │
│  Box<dyn Tool> (custom trait)                                   │
│       ↓                                                         │
│  Runner::execute_tool() or ToolRegistry::execute()              │
│       ↓                                                         │
│  impl Tool for ReadTool  →  execute()                           │
└─────────────────────────────────────────────────────────────────┘
```

### Evidence: CLI Uses Only RigAgent

From `cli/src/interactive.rs`:
```rust
use codelet_core::RigAgent;
// ...
let agent = RigAgent::with_default_depth(rig_agent);
```

From `cli/src/lib.rs`:
```rust
use codelet_core::RigAgent;
// ...
let agent = RigAgent::with_default_depth(rig_agent);
```

**The `Runner` struct is never instantiated in CLI code.**

---

## Code Duplication Analysis

Each tool file contains TWO implementations with nearly identical logic:

### Example: `tools/src/read.rs`

**Custom trait implementation (lines 66-153)** - VESTIGIAL:
```rust
#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str { "Read" }
    fn description(&self) -> &str { "Read file contents..." }
    fn parameters(&self) -> &ToolParameters { &self.parameters }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
        let path = require_absolute_path(file_path)?;
        require_file_exists(path)?;
        let content = read_file_contents(path)?;
        // ... 70 lines of processing logic ...
    }
}
```

**Rig trait implementation (lines 181-250)** - ACTIVE:
```rust
impl rig::tool::Tool for ReadTool {
    const NAME: &'static str = "read";
    type Error = ReadError;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition { ... }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = require_absolute_path(&args.file_path)?;
        require_file_exists(path)?;
        let content = read_file_contents(path)?;
        // ... 50 lines of SAME processing logic ...
    }
}
```

**The execution logic is duplicated.** Any bug fix or behavior change must be applied to both implementations.

---

## Components to Remove

### 1. Custom `Tool` Trait (`tools/src/lib.rs:95-108`)

```rust
// DELETE THIS
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &ToolParameters;
    async fn execute(&self, args: Value) -> Result<ToolOutput>;
}
```

### 2. `ToolRegistry` (`tools/src/lib.rs:111-187`)

```rust
// DELETE THIS
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self { ... }
    pub fn with_core_tools() -> Self { ... }
    pub fn register(&mut self, tool: Box<dyn Tool>) { ... }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> { ... }
    pub fn execute(&self, name: &str, args: Value) -> Result<ToolOutput> { ... }
    pub fn list(&self) -> Vec<&str> { ... }
    pub fn definitions(&self) -> Vec<ToolDefinition> { ... }
}
```

### 3. `Runner` (`core/src/lib.rs:18-170`)

```rust
// DELETE THIS
pub struct Runner {
    messages: Vec<Message>,
    tools: ToolRegistry,
    provider: Option<Box<dyn LlmProvider>>,
}

impl Runner {
    pub fn new() -> Self { ... }
    pub fn with_tools(tools: ToolRegistry) -> Self { ... }
    pub fn with_provider(provider: Box<dyn LlmProvider>) -> Self { ... }
    pub fn execute_tool(&self, name: &str, args: Value) -> Result<ToolOutput> { ... }
    pub async fn run(&mut self, user_input: &str) -> Result<Vec<Message>> { ... }
}
```

### 4. `ToolParameters` (`tools/src/lib.rs:30-50`)

```rust
// DELETE THIS - rig uses schemars::JsonSchema instead
pub struct ToolParameters {
    pub schema_type: String,
    pub properties: serde_json::Map<String, Value>,
    pub required: Vec<String>,
}
```

### 5. Custom `impl Tool for XxxTool` in Each Tool File

Remove from all 7 tool files:
- `tools/src/read.rs` (lines 66-153)
- `tools/src/write.rs`
- `tools/src/edit.rs`
- `tools/src/bash.rs`
- `tools/src/grep.rs`
- `tools/src/glob.rs`
- `tools/src/astgrep.rs`

---

## Files to Modify

### High Impact (Core Changes)

| File | Action | Lines Affected |
|------|--------|----------------|
| `tools/src/lib.rs` | Remove `Tool` trait, `ToolRegistry`, `ToolParameters` | ~100 lines |
| `core/src/lib.rs` | Remove `Runner`, keep only `RigAgent` re-export | ~150 lines |
| `tools/src/read.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/write.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/edit.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/bash.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/grep.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/glob.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |
| `tools/src/astgrep.rs` | Remove `impl Tool`, keep `impl rig::tool::Tool` | ~90 lines |

### Test Files to Update

| File | Action |
|------|--------|
| `tests/bash_tool_test.rs` | Replace `ToolRegistry` usage with direct tool testing |
| `tests/grep_glob_tools_test.rs` | Replace `ToolRegistry` usage with direct tool testing |
| `tests/core_file_tools_test.rs` | Replace `ToolRegistry` usage with direct tool testing |
| `tests/astgrep_tool_test.rs` | Replace `ToolRegistry` usage with direct tool testing |
| `tests/agent_runner_test.rs` | Remove or migrate to `RigAgent` testing |
| `tests/agent_multi_turn_test.rs` | Update to use `RigAgent` |
| `tests/rig_agent_multi_turn_test.rs` | Remove `ToolRegistry` references |
| `tests/integration_wiring_test.rs` | Update tool registration tests |
| `tests/project_scaffold_test.rs` | Update module export tests |

### Examples to Update

| File | Action |
|------|--------|
| `examples/test_all_tools.rs` | Rewrite to test via rig's trait |

---

## Implementation Steps

### Phase 1: Prepare (Estimated: 1 hour)

1. **Create branch**: `git checkout -b refac/remove-legacy-tool-system`
2. **Run existing tests**: `cargo test` - ensure baseline passes
3. **Document current test count**: Record number of passing tests

### Phase 2: Remove Legacy Code (Estimated: 2 hours)

1. **Remove from `tools/src/lib.rs`**:
   - Delete `Tool` trait (lines 95-108)
   - Delete `ToolParameters` struct (lines 30-50)
   - Delete `ToolRegistry` struct and impl (lines 111-187)
   - Keep `ToolDefinition` and `ToolOutput` (used by other code)

2. **Remove from `core/src/lib.rs`**:
   - Delete `Runner` struct and impl (lines 18-170)
   - Keep `RigAgent` re-export
   - Keep `compaction` module

3. **Remove custom `impl Tool` from each tool file**:
   - `read.rs`: Delete lines 66-153, keep 181-250
   - `write.rs`: Delete custom impl, keep rig impl
   - `edit.rs`: Delete custom impl, keep rig impl
   - `bash.rs`: Delete custom impl, keep rig impl
   - `grep.rs`: Delete custom impl, keep rig impl
   - `glob.rs`: Delete custom impl, keep rig impl
   - `astgrep.rs`: Delete custom impl, keep rig impl

4. **Remove `parameters` field from tool structs**:
   - Each tool struct has `parameters: ToolParameters` - remove it
   - Rig uses `schemars::JsonSchema` derive on Args structs instead

### Phase 3: Update Tests (Estimated: 2-3 hours)

1. **Strategy for tool tests**:
   ```rust
   // OLD: Using ToolRegistry
   let registry = ToolRegistry::default();
   let result = registry.execute("Read", args).await?;

   // NEW: Direct tool testing via rig trait
   use rig::tool::Tool;
   let tool = ReadTool::new();
   let args = ReadArgs { file_path: "...".into(), offset: None, limit: None };
   let result = tool.call(args).await?;
   ```

2. **Update each test file**:
   - Replace `ToolRegistry::default()` with direct tool instantiation
   - Replace `registry.execute()` with `tool.call()`
   - Update assertions for new return types (`String` vs `ToolOutput`)

3. **Remove ToolRegistry-specific tests**:
   - Tests like "ToolRegistry includes all core tools" become obsolete
   - Replace with tests verifying tools work via rig trait

### Phase 4: Verify (Estimated: 1 hour)

1. **Run all tests**: `cargo test`
2. **Run clippy**: `cargo clippy -- -D warnings`
3. **Run fmt check**: `cargo fmt --check`
4. **Manual CLI testing**: Verify interactive mode works

### Phase 5: Cleanup (Estimated: 30 minutes)

1. **Remove unused imports** in all modified files
2. **Update documentation** if any references legacy system
3. **Update `rust.md`** to reflect completed migration

---

## Expected Outcomes

### Lines of Code Removed

| Component | Approximate Lines |
|-----------|-------------------|
| `Tool` trait | 15 |
| `ToolParameters` | 25 |
| `ToolRegistry` | 80 |
| `Runner` | 150 |
| Custom `impl Tool` (7 tools × ~90) | 630 |
| **Total** | **~900 lines** |

### Benefits

1. **No code duplication**: Single implementation per tool
2. **Tests match production**: Tests use same code path as CLI
3. **Reduced maintenance**: Changes only need to be made once
4. **Clearer architecture**: One framework, one execution path
5. **Smaller binary**: Less dead code compiled

### Risks

1. **Test breakage**: Many tests use `ToolRegistry` - requires rewriting
2. **Hidden dependencies**: Some code might reference legacy types unexpectedly
3. **Behavior differences**: Subtle differences between `execute()` and `call()` return types

### Mitigation

- Run tests frequently during refactoring
- Use `cargo check` to find compilation errors early
- Review each tool's test coverage after migration

---

## Verification Checklist

- [ ] All tool files have only `impl rig::tool::Tool`
- [ ] `tools/src/lib.rs` exports only `ToolDefinition`, `ToolOutput`, and individual tools
- [ ] `core/src/lib.rs` exports only `RigAgent`, `compaction`, and common types
- [ ] No references to `ToolRegistry` in non-test code
- [ ] No references to custom `Tool` trait in non-test code
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] CLI interactive mode works manually tested

---

## Appendix: Grep Evidence

### ToolRegistry Usage (Production vs Tests)

```
# Production code (CLI) - NO USAGE
cli/src/*.rs: 0 references to ToolRegistry

# Test code - ALL USAGE
tests/*.rs: 30+ references to ToolRegistry
```

### Runner Usage (Production vs Tests)

```
# Production code (CLI) - NO USAGE
cli/src/*.rs: 0 references to Runner (only RigAgent)

# Test code - SOME USAGE
tests/agent_runner_test.rs: References Runner for legacy tests
```

This confirms the legacy system is only maintained for backward compatibility with tests, not for production functionality.
