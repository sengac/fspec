# AST Research: Handler Patterns for inject_summary Tool

## Research Goal
Identify the established patterns for Rig Tool implementations with per-session handler registries in codelet-tools, to ensure CMPCT-008 follows them exactly.

## 1. Handler Type Aliases (Arc<dyn Fn + Send + Sync>)

All per-session handlers follow this pattern:

```
codelet/tools/src/fspec_handler.rs:69
  pub type FspecHandler = Arc<dyn Fn(FspecRequest) -> FspecResult + Send + Sync>;

codelet/tools/src/session_search/handler.rs:30
  pub type SessionSearchHandler = Arc<dyn Fn(SessionSearchAction, Uuid) -> SessionSearchResult + Send + Sync>;

codelet/tools/src/bridge_handler.rs:44
  pub type BridgeHandler = Arc<dyn Fn(BridgeRequest) -> BridgeResult + Send + Sync>;

codelet/tools/src/tool_pause.rs:64
  pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>;
```

**Pattern for inject_summary:**
```rust
pub type InjectSummaryHandler = Arc<dyn Fn(Uuid, String) -> InjectSummaryResult + Send + Sync>;
```

## 2. Global Handler Registries (Lazy<RwLock<HashMap<Uuid, Handler>>>)

All per-session registries use this exact structure:

```
codelet/tools/src/fspec_handler.rs:73
  static FSPEC_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, FspecHandler>>> =
      once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

codelet/tools/src/session_search/handler.rs:34
  static SESSION_SEARCH_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, SessionSearchHandler>>> =
      once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

codelet/tools/src/bridge_handler.rs:62
  static BRIDGE_SESSION_CONTEXTS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, Arc<BridgeSessionContext>>>> =
      once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));
```

## 3. Tool Trait Implementations

All tools follow the Rig Tool trait pattern:

```
codelet/tools/src/fspec.rs:64
  impl Tool for FspecTool { ... }

codelet/tools/src/session_search/mod.rs:61
  impl Tool for SessionSearchTool { ... }
```

Key fields on Tool trait:
- `const NAME: &'static str`
- `type Error = ToolError`
- `type Args = <Args struct>`
- `type Output = String`
- `definition()` returns `rig::completion::ToolDefinition`
- `call()` dispatches to handler or returns error

## 4. Session-Scoped Tool Construction

SessionSearchTool stores session_id for handler lookup:

```
codelet/tools/src/session_search/mod.rs:47-59
  pub struct SessionSearchTool { session_id: Uuid }
  impl SessionSearchTool {
      pub fn new(session_id: Uuid) -> Self { Self { session_id } }
  }
```

## 5. Registry API (set/has/execute/clear)

Standard 4-function API:
- `set_<handler>(session_id, Option<Handler>)` — insert or remove
- `has_<handler>(session_id) -> bool` — check presence
- `execute_<action>(session_id, args) -> Result` — dispatch to handler
- `clear_all_<handlers>()` — for testing

## 6. lib.rs Re-exports

SessionSearch re-exports:
```
codelet/tools/src/lib.rs:115-119
  pub use session_search::{
      SessionSearchTool, SessionSearchHandler,
      set_session_search_handler, has_session_search_handler,
      clear_all_session_search_handlers,
  };
```

FspecHandler re-exports:
```
codelet/tools/src/lib.rs:94-100
  pub use fspec_handler::{
      execute_fspec_command_for_session,
      has_fspec_handler_for_session,
      set_fspec_handler_for_session,
      clear_all_fspec_handlers,
      FspecHandler, FspecRequest as FspecHandlerRequest, FspecResult as FspecHandlerResult,
  };
```

## Conclusion

The inject_summary tool should follow the **SessionSearchTool pattern** most closely:
- Single file `inject_summary.rs` containing tool struct, args, result, handler type, registry, API functions
- Tool constructed with `session_id: Uuid`
- Handler dispatched via stored `session_id`
- Standard 4-function registry API
- Re-exported from `lib.rs`
