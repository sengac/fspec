# TOOL-017 AST Research: Handler Pattern and Tool Registration

## Handler Pattern Analysis (InjectSummaryTool Reference)

### Pattern: Per-Session Handler Registry

The established handler pattern uses four components:

1. **Type alias**: `Arc<dyn Fn(Uuid, Request) -> Result<Response, String> + Send + Sync>`
2. **Global registry**: `once_cell::sync::Lazy<RwLock<HashMap<Uuid, Handler>>>`
3. **Registry functions**: `set_handler`, `has_handler`, `execute`, `clear_all`
4. **Tool struct**: Implements `rig::tool::Tool` with `session_id` field, delegates to registry

### Key Implementation Details

- Handler is cloned (Arc clone) before releasing the RwLock read guard
- Handler closure runs without holding any lock
- Missing handler returns descriptive error (mode-gating)
- Tool call serializes result to JSON string for LLM consumption

### Registration Points

1. **lib.rs**: `pub mod` + `pub use` for all exported types and functions
2. **Providers**: `.tool(ToolName::new(session_id))` in agent builder
3. **NAPI**: Handler creation + registration before agent run, cleanup after

### Tools Following This Pattern

| Tool | Handler Type | Request | Response |
|------|-------------|---------|----------|
| InjectSummaryTool | `InjectSummaryHandler` | `(Uuid, String)` | `InjectSummaryResult` |
| SessionSearchTool | `SessionSearchHandler` | `(Uuid, SessionSearchAction, ...)` | `String` |
| FspecTool | `FspecHandler` | `(FspecRequest)` | `FspecResult` |
| DeepSearchTool | `DeepSearchHandler` | `(Uuid, String, Vec<String>, ...)` | `String` |

### HITL Tool Application

For request_user_input, follow the same pattern:
- `HitlHandler = Arc<dyn Fn(Uuid, HitlRequest) -> Result<HitlResponse, String> + Send + Sync>`
- `HITL_HANDLERS: Lazy<RwLock<HashMap<Uuid, HitlHandler>>>`
- `set_hitl_handler`, `has_hitl_handler`, `execute_hitl`, `clear_all_hitl_handlers`
- `RequestUserInputTool` implements `rig::tool::Tool`
- Handler blocks synchronously (same as pause_for_user pattern)
- NAPI creates handler that uses crossbeam/oneshot to bridge to TUI

## Existing PauseHandler Pattern (tool_pause.rs)

The pause handler is a **global singleton** (not per-session):
- `PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>`
- Simple enum kinds: Continue, Confirm, Triple
- Returns simple enum response: Resumed, Approved, Denied, Interrupted, AllowOnce, AllowSession

The HITL tool should NOT use PauseHandler because:
1. It needs structured question/answer data, not simple enum responses
2. It needs per-session isolation (PauseHandler is global)
3. The data shapes are completely different (questions array vs message string)

## Provider Registration (Verified)

All providers chain tools in their agent builder:
- `codelet/providers/src/claude.rs`
- `codelet/providers/src/openai.rs`  
- `codelet/providers/src/gemini.rs`
- `codelet/providers/src/zai.rs`
- `codelet/providers/src/codex/mod.rs`

Each uses: `.tool(ToolName::new(session_id))` pattern.
