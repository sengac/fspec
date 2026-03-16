# AST Research: Handler-Delegated Tool Pattern for AgentManager

## Research Goal
Analyze the existing handler-delegated tool pattern (SessionSearchTool) to understand the implementation pattern for AgentManagerTool.

## Tool Pattern Analysis

### 1. codelet-tools/src/session_search/mod.rs — Tool Struct
```rust
#[derive(Clone, Debug)]
pub struct SessionSearchTool {
    session_id: Uuid,
}

impl SessionSearchTool {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for SessionSearchTool {
    const NAME: &'static str = "SessionSearch";
    type Error = ToolError;
    type Args = SessionSearchArgs;
    type Output = String;

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = execute_session_search(self.session_id, args.action);
        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution { ... })
    }
}
```

### 2. codelet-tools/src/session_search/handler.rs — Static Handler Registry
```rust
pub type SessionSearchHandler = Arc<dyn Fn(SessionSearchAction, Uuid) -> SessionSearchResult + Send + Sync>;

static SESSION_SEARCH_HANDLERS: Lazy<RwLock<HashMap<Uuid, SessionSearchHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn set_session_search_handler(session_id: Uuid, handler: Option<SessionSearchHandler>) { ... }
pub fn execute_session_search(session_id: Uuid, action: SessionSearchAction) -> SessionSearchResult { ... }
```

### 3. codelet-tools/src/session_search/types.rs — Serde Tagged Dispatch
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum SessionSearchAction {
    Recent { ... },
    Search { ... },
    Show { ... },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum SessionSearchResult {
    Recent { ... },
    Search { ... },
    Error { message: String },
}
```

### 4. Handler Registration in session_manager.rs agent_loop()
```rust
// Before agent run:
let session_search_handler = crate::session_search_handler::create_handler(...);
codelet_tools::set_session_search_handler(session.id, Some(session_search_handler));

// After agent run:
codelet_tools::set_session_search_handler(session.id, None);
```

### 5. Provider Registration (all 5 providers)
```rust
.tool(SessionSearchTool::new(session_id))
```

## ChainOfCommand API (session_manager.rs:1264-1389)
- `add_supervisor(subordinate_id, supervisor_id)` — registers relationship, validates no cycles
- `remove_supervisor(supervisor_id)` — removes supervisor from both maps
- `get_supervisors(subordinate_id)` — returns Vec<Uuid>
- `get_subordinate(supervisor_id)` — returns Option<Uuid>
- `cleanup_subordinate(subordinate_id)` — bulk cleanup

## Session Creation/Destruction
- `create_session_with_id(id, model, project, name)` — core creation path
- `destroy_session(id)` — cleanup ChainOfCommand + remove session + interrupt

## BackgroundSession Key Fields
- `id: Uuid`, `name: RwLock<String>`, `role: RwLock<Option<String>>`
- `status: AtomicU8` (Idle=0, Running=1, Interrupted=2, Paused=3, Compacting=4)
- `provider_id: RwLock<Option<String>>`, `model_id: RwLock<Option<String>>`

## Conclusion
AgentManagerTool follows the identical pattern. The key difference is the handler needs `Arc<SessionManager>` access for session creation/destruction, while SessionSearch only needed read-only access to message stores.
