# AST Research: DeepSearch Self-Recursion Touch Points

**Date:** 2026-03-17
**Work Unit:** RLM-002
**Method:** DeepSearch tool exploring codelet/tools/src/deep_search/ and codelet/napi/src/deep_search_handler*

---

## Findings: 5 Touch Points for Self-Recursion

### 1. DeepSearchTool struct (codelet/tools/src/deep_search/mod.rs:234-247)

```rust
#[derive(Clone, Debug)]
pub struct DeepSearchTool {
    pub session_id: Uuid,
}
```

**Change needed:** Add `depth: usize` and `max_recursion_depth: usize` fields.

### 2. DeepSearchArgs (mod.rs:60-75)

```rust
pub struct DeepSearchArgs {
    pub query: String,
    pub scope: Option<Vec<String>>,
    pub max_depth: Option<usize>,
}
```

**Change needed:** Add `max_recursion_depth: Option<usize>` field (default: 2).

### 3. DeepSearchHandler type alias (mod.rs:98-106)

```rust
pub type DeepSearchHandler = Arc<
    dyn Fn(String, Option<Vec<String>>, usize)
        -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
    + Send + Sync,
>;
```

**Change needed:** Add depth and max_recursion_depth params to the closure signature.

### 4. SUB_AGENT_TOOL constants (mod.rs:45-57)

```rust
pub const SUB_AGENT_TOOL_NAMES: [&str; 7] = [
    "Read", "Grep", "AstGrep", "Glob", "Ls", "Bash", "SessionSearch",
];
pub const SUB_AGENT_TOOL_COUNT: usize = SUB_AGENT_TOOL_NAMES.len(); // 7
```

**Change needed:** Split into BASE_TOOL_COUNT (7) and RECURSIVE_TOOL_COUNT (8), or make dynamic.

### 5. build_and_run! macro (deep_search_handler.rs:99-135)

Hardcoded 7 `.tool()` calls. Must conditionally add `.tool(DeepSearchTool::new(session_id).with_depth(depth+1))` when `depth < max_recursion_depth`.

**Compile-time assertion at line 187:** `assert!(SUB_AGENT_TOOL_COUNT == 7)` — must update.

### 6. execute_deep_search (deep_search_handler.rs:55-92)

Currently registers SessionSearch handler only. Must ALSO register a DeepSearch handler for the ephemeral UUID so the child sub-agent can call DeepSearch itself. Needs a cleanup guard (like SessionSearchCleanup).

### 7. Handler registration in session_manager.rs (lines 4297-4319)

Parent handler captures provider/model but no depth info. Must pass depth=0 and max_recursion_depth through.

---

## Summary: Minimal Change Set

1. **mod.rs:** Add depth/max_recursion_depth to DeepSearchTool, DeepSearchArgs, DeepSearchHandler
2. **mod.rs:** Update build_system_prompt() to conditionally include DeepSearch tool
3. **deep_search_handler.rs:** Register recursive DeepSearch handler in execute_deep_search()
4. **deep_search_handler.rs:** Conditionally add DeepSearchTool in build_and_run! macro
5. **session_manager.rs:** Pass depth=0 through parent handler registration
