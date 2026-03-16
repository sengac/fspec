# AMGR-003 AST Research — Tool & Session Infrastructure

## Tool Implementation Pattern

Two variants in the codebase:

### 1. Direct Tool (e.g., ReadTool, WriteTool)
- Struct carries `session_id: Uuid`
- `impl Tool for X` with `call()` doing the work directly
- Used for tools with no cross-boundary dependencies

### 2. Handler-Delegated Tool (e.g., SessionSearchTool, InjectSummaryTool)
- Static global `HashMap<Uuid, Handler>` keyed by session_id
- Handler registered from `session_manager.rs` before agent_loop, unregistered after
- `call()` looks up handler by session_id, delegates to it
- `clear_all_*_handlers()` for test teardown

**AgentManager should use the handler pattern** since it needs access to SessionManager (which lives in codelet-napi, not codelet-tools).

### Handler Type Signature
```rust
pub type SessionSearchHandler =
    Arc<dyn Fn(SessionSearchAction, Uuid) -> SessionSearchResult + Send + Sync>;
```

### Handler Registry
```rust
static SESSION_SEARCH_HANDLERS: Lazy<RwLock<HashMap<Uuid, SessionSearchHandler>>> = ...;

pub fn set_session_search_handler(session_id: Uuid, handler: Option<SessionSearchHandler>) { ... }
pub fn execute_session_search(session_id: Uuid, action: SessionSearchAction) -> SessionSearchResult { ... }
```

## Tool Registration in Providers

All 5 providers have `create_rig_agent(session_id, preamble, thinking_config)`:
- Claude, OpenAI: use raw tool structs (`.tool(SessionSearchTool::new(session_id))`)
- Gemini, ZAI, Codex: wrap some tools in FacadeToolWrapper for provider-native names

For AgentManager: register `.tool(AgentManagerTool::new(session_id))` in all 5 providers.

## Session Infrastructure (session_manager.rs)

### BackgroundSession key fields
- `id: Uuid`, `name: RwLock<String>`, `project: String`
- `status: AtomicU8` (Idle=0, Running=1, Interrupted=2, Paused=3, Compacting=4)
- `provider_id / model_id: RwLock<Option<String>>`
- `supervisor_broadcast: broadcast::Sender<StreamChunk>` (capacity 256)
- `supervisor_input_tx/rx: mpsc::channel<SupervisorInput>` (capacity 16)

### SessionManager singleton
- `sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>` (ordered)
- `chain_of_command: ChainOfCommand` (subordinate→supervisors HashMap)
- `MAX_SESSIONS = 10` (but spawn is unbounded per AMGR-002 rules)

### Existing NAPI functions to reuse
- `session_create_supervisor(subordinate_id, model, project, name) → supervisor_id`
- `session_get_subordinate(session_id) → Option<subordinate_id>`
- `session_get_supervisors(session_id) → Vec<supervisor_id>`
- `session_set_role(session_id, role_name, brief, auto_inject)`
- `supervisor_inject(supervisor_id, message)` — sends to subordinate
- `SessionManager::instance().destroy_session(id)` — cleanup

### ChainOfCommand
- `add_supervisor(subordinate_uuid, supervisor_uuid)`
- `remove_supervisor(subordinate_uuid, supervisor_uuid)`
- `get_supervisors(subordinate_uuid) → Vec<Uuid>`
- `get_subordinate(supervisor_uuid) → Option<Uuid>`
- `cleanup_session(uuid)` — removes all relationships for a session
- `is_empty()` — check if no entries

### SupervisorInput
- `session_id: String`, `role_name: String`, `message: String`
- `images: Vec<SupervisorInputImage>` (optional, BRIDGE-007)
- `new(session_id, role_name, message) → Result<Self, String>` — validates non-empty message
- `format_supervisor_input(input) → String` — formats for injection

### SupervisorRole
- `name: String`, `brief: Option<String>`, `auto_inject: bool`
- `new(name, brief) → Result<Self, String>` — validates non-empty name

## Test Patterns

1. **Handler isolation**: `with_clean_handlers()` + `#[serial]`
2. **Mock handlers**: `Arc<dyn Fn(...) -> ... + Send + Sync>` closures
3. **`@step` comments**: Map directly to Gherkin steps
4. **Full tool call**: `tool.call(args).await` → parse JSON → assert
5. **Serde roundtrip**: Verify deserialization from JSON strings
6. **AtomicBool verification**: Assert handler was called

## File Locations

- Tools: `codelet/tools/src/` (module or single .rs file)
- Tool tests: `codelet/tools/src/<tool>/tests.rs` (in-module) or `codelet/tools/tests/<tool>_test.rs`
- Providers: `codelet/providers/src/{claude,codex,openai,gemini,zai}.rs`
- Session manager: `codelet/napi/src/session_manager.rs`
- Handler registration: Inside `agent_loop()` in session_manager.rs (~line 5378)
