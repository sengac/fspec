# AST Research — RLCD-003 (workflow gate wiring)

Research for the RLCD-003 integration seams, performed in the specifying
phase (2026-09-24). Tool: AstGrep + Read + Grep.

## 1. Integration boundary — FspecToolFacadeWrapper (agent-mode only)

File: `rust/tools/src/facade/wrapper.rs`

- `pub struct FspecToolFacadeWrapper { facade: BoxedFspecToolFacade, session_id: Uuid }` (line 1087)
  - `pub fn new(facade: BoxedFspecToolFacade, session_id: Uuid) -> Self` (line 1101)
  - `pub fn session_id(&self) -> Uuid` (line 1111)
- `impl Tool for FspecToolFacadeWrapper` (line 1116):
  - `const NAME: &'static str = "fspec_facade_wrapper"`
  - `type Error = ToolError; type Args = FacadeArgs; type Output = String`
  - `fn name(&self) -> String` → `self.facade.tool_name()`
  - `async fn call(&self, args: FacadeArgs) -> Result<String, ToolError>` (line 1138):
    1. `check_pre_tool_hook(self.session_id, &self.name(), &args.0)?` (HOOK-013)
    2. `let internal_params = self.facade.map_params(args.0)` → `Err(enrich_facade_arg_error(...))` on error
    3. `if !has_fspec_handler_for_session(self.session_id) { return Err(ToolError::Execution{ tool: "fspec", ... }) }`
    4. `let result = execute_fspec_command_for_session(self.session_id, FspecRequest { command, args_json, project_root, provider })`
    5. `result.success` → `Ok(result.data [+ system_reminder])`; else `Err(ToolError::Execution { tool: "fspec", message: result.error ... })`

**RLCD-003 insertion point**: between step 3 (handler present) and step 4
(execute). `run_rlcd_gate(self.session_id, &internal_params.command,
&internal_params.args, &internal_params.project_root)`; `Rejected{reason}`
→ `Err(ToolError::Execution { tool: "fspec", message: reason })` WITHOUT
calling the handler; `Execute` → normal path unchanged.

Note: `FspecToolFacadeWrapper::call` is a synchronous-looking `async fn` that
calls the synchronous handler; the RLCD gate entry is async (supervisor
`ensure_ready` + client `classify`), so `call` awaits it in place (already
async).

## 2. Handler stub API (test seam)

File: `rust/tools/src/fspec_handler.rs`

- `pub struct FspecRequest { pub command: String, pub args_json: String, pub project_root: String, pub provider: String }` (line 32)
- `pub struct FspecResult { pub success: bool, pub data: String, pub error: Option<String>, pub system_reminder: Option<String> }` (line 45)
- `pub type FspecHandler = Arc<dyn Fn(FspecRequest) -> FspecResult + Send + Sync>` (line 69)
- `pub fn set_fspec_handler_for_session(session_id: Uuid, handler: Option<FspecHandler>)` (line 80)
- `pub fn has_fspec_handler_for_session(session_id: Uuid) -> bool` (line 94)
- `pub fn execute_fspec_command_for_session(session_id: Uuid, request: FspecRequest) -> FspecResult` (line 105)
- `pub fn clear_all_fspec_handlers()` (line 132) — test cleanup

Test pattern (already used by facade tests): register an `Arc<dyn Fn(FspecRequest) -> FspecResult>`
stub that records the request into `Arc<Mutex<...>>` and returns a
`success: true` `FspecResult` — then assert whether the gate rejected
(request NOT recorded) or executed (request recorded).

## 3. Pause API (Triple pause + session allowances)

File: `rust/tools/src/tool_pause.rs`

- `pub enum PauseKind { Continue, Confirm, Triple }` (Triple = BLOCK-007: Allow Once / Allow Session / Deny)
- `pub struct PauseRequest { pub kind: PauseKind, pub tool_name: String, pub message: String, pub details: Option<String> }`
- `pub enum PauseResponse { Resumed, Approved, Denied, Interrupted, AllowOnce, AllowSession }`
- `pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>`
- `pub fn set_pause_handler(session_id: Uuid, handler: Option<PauseHandler>)`
- `pub fn pause_for_user(session_id: Uuid, request: PauseRequest) -> PauseResponse` (no handler registered ⇒ `Resumed`)
- `pub fn has_pause_handler(session_id: Uuid) -> bool`

Session allowances (BLOCK-005), file `rust/tools/src/blocklist/middleware.rs`:

- `static SESSION_ALLOWANCES: LazyLock<RwLock<HashSet<String>>>` (line 22) — process-global (not per-session keyed)
- `pub fn allow_for_session(pattern: &str)` (line 299)
- `pub fn is_session_allowed(pattern: &str) -> bool` (line 309)
- `pub fn clear_session_allowances()` (line 319)

Existing consumer pattern (blocklist middleware `check_file_path`):
`is_session_allowed(&pattern)` short-circuit → `pause_for_user(session_id, PauseRequest{ kind: Triple, ... })` →
`AllowOnce ⇒ Ok`, `AllowSession ⇒ allow_for_session(pattern) + Ok`,
`Denied|Interrupted ⇒ Err(BlockedError{...})`, `_ ⇒ Ok`.
RLCD-003 mirrors this exactly with pattern key `rlcd-gate:<command>`.

Because `SESSION_ALLOWANCES` is a process-global static, tests that set/clear
allowances must run under `serial_test::serial` (same convention as the
blocklist middleware tests).

## 4. RLCD-001/002 building blocks (already implemented)

File: `rust/tools/src/rlcd/service.rs` (AstGrep verified):

- `pub async fn health_check(url: &str, timeout: Duration) -> Result<String, RlcdError>` — returns the model name from /health (never hardcoded)
- `pub struct RlcdSupervisor` with `pub fn new(config: RlcdConfig)`, `pub async fn ensure_ready(&self, budget: Duration) -> Result<String, RlcdError>` (line 208 — returns model; `Err(RlcdError::Unreachable{url, detail})` when the budget elapses; `!config.enabled` ⇒ immediate `Unreachable` with detail "rlcd disabled in user config")
- `pub fn global_supervisor() -> &'static RlcdSupervisor` (RLCD_STATUS)

File: `rust/tools/src/rlcd/config.rs`:

- `pub struct RlcdConfig { enabled: bool, url: String, backend: String, spawn: RlcdSpawnConfig }` — serde camelCase; **no `gate` field yet** (RLCD-003 adds `gate: RlcdGateConfig` with `#[serde(default)]`)
- `pub fn load_rlcd_config() -> RlcdConfig` (env `FSPEC_USER_DIR` / `~/.fspec` + `fspec-config.json`)
- `pub fn load_rlcd_config_from(config_path: &Path) -> RlcdConfig` — path-injectable core (missing/malformed ⇒ defaults)

File: `rust/tools/src/rlcd/client.rs`:

- `pub trait RlcdClient { async fn classify(&self, base_url: &str, model: &str, state: &str, questions: &[RlcdQuestion], budget: Duration) -> Result<RlcdResponse, RlcdError> }`
- `pub struct HttpRlcdClient` (Default) — POST `{base}/v1/classifier`, body `{model, state, questions:{qid:{type, instructions, criteria?}}}`
- `pub struct RlcdQuestion { qid: String, r#type: String, instructions: Option<String>, criteria: Option<serde_json::Value> }`
- `pub struct RlcdResponse { model: String, answers: serde_json::Value, usage: serde_json::Value }`
- `RlcdError`: `Unreachable{url, detail}` / `Health{detail}` / `Validation{detail}` (422) / `Busy{retry_after}` (429) / `Server{detail}` (5xx/timeout)

File: `rust/tools/src/rlcd/mod.rs` — module re-exports (RLCD-003 adds `gate`).

## 5. Facade trait (test facade impl target)

File: `rust/tools/src/facade/traits.rs` (line 204):

```rust
pub trait FspecToolFacade: Send + Sync {
    fn provider(&self) -> &'static str;
    fn tool_name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;
    fn map_params(&self, input: Value) -> Result<InternalFspecParams, ToolError>;
}
pub type BoxedFspecToolFacade = Arc<dyn FspecToolFacade>;
```

`InternalFspecParams { command: String, args: String, project_root: String }`
(`rust/tools/src/facade/fspec_facade.rs`, line 25).

A minimal test facade (`TestFspecFacade`) returning fixed
`InternalFspecParams` + `definition()` with an empty `json!({"type":"object"})`
schema is enough to construct `FspecToolFacadeWrapper::new(test_facade, session_id)`
in the RLCD-003 test suite.

## 6. Mock server (reused from RLCD-001/002 suites)

`rust/tools/tests/rlcd_mock.rs` (included per-suite via `#[path = "rlcd_mock.rs"] mod rlcd_mock;`):

- `MockRlcd::ready(model)` / `MockRlcd::with_health(Option<&str>)` (None ⇒ accept-and-close = "never ready")
- `set_answers(json)`, `set_status(status, body)`, `set_retry_after(v)`, `set_delay_ms(ms)`
- `classifier_hits() -> usize`, `classifier_bodies() -> Vec<String>` (request capture)
- `base_url()`, `port()`, `start().await`, `stop().await`

Classifier fixture for the gate (choice answer):
`{"model":"<model>","answers":{"gate":{"type":"choice","choice":"<argmax>","confidence":0.9,"probabilities":{"proceed":0.9,"hold":0.05,"ask_user":0.05}}},"usage":{"input_tokens":12,"output_tokens":0}}`

## 7. Design consequences (captured in the finalized API contract note)

- Gate entry is async ⇒ `FspecToolFacadeWrapper::call` (already `async fn`) awaits it in place.
- Unreachable-engine tests: point `rlcd.url` (via `FSPEC_USER_DIR` temp config) at a closed port ⇒ `ensure_ready` fast-fails (first poll down; local auto-spawn skipped because the test env has no `rlcd` binary on PATH… NOTE: the `rlcd` binary DOES exist on this machine — tests must use a remote-style URL (e.g. 10.255.255.1:9999) or a closed 127.0.0.1 port where spawn also fails; a closed 127.0.0.1 port triggers the spawn attempt: `select_port` finds the port free ⇒ spawn `rlcd serve --port <chosen>` ⇒ a REAL server could start. SAFEST: use the non-local URL form `http://10.255.255.1:9999` (connect-only, never spawned) — proven by the RLCD-002 unreachable test.)
- Session allowances are global ⇒ `#[serial]` on allowance-touching tests.
- `rlcd.enabled=false` ⇒ `ensure_ready` errors immediately ⇒ fail-open execute (no network).

## 8. Deterministic test probability vectors (16 scenarios)

Feature file: `spec/features/rlcd-semantic-gate-for-fspec-workflow-commands-agent-mode.feature`
(14 scenarios; classifier fixture answers `gate` with the vector below).

Defaults: holdThreshold 0.65, askThreshold 0.40, proceedMargin 0.15.

| # | scenario | P(proceed) | P(hold) | P(ask_user) | outcome (default thresholds) |
|---|----------|-----------|---------|------------|------------------------------|
| 1 | proceed executes normally | 0.90 | 0.05 | 0.05 | argmax proceed, margin 0.85 > 0.15 ⇒ Proceed |
| 2 | hold rejects before execution | 0.05 | 0.80 | 0.15 | argmax hold ⇒ Hold (P(hold) 0.80 ≥ 0.65 also) |
| 3 | ask_user + Deny rejects | 0.25 | 0.20 | 0.55 | argmax ask_user ⇒ AskUser → Deny |
| 4 | ask_user + AllowOnce, then prompt again | 0.40 | 0.10 | 0.50 | AskUser → AllowOnce (executes; re-prompts) |
| 5 | ask_user + AllowSession suppresses | 0.40 | 0.10 | 0.50 | AskUser → AllowSession (allowance set; 2nd call executes, no RLCD hit) |
| 6 | unreachable engine fails open | — | — | — | ensure_ready Err ⇒ Execute + warn |
| 7 | ungated command bypasses gate | — | — | — | "list-work-units" not in gate.commands ⇒ Execute, 0 classifier hits |
| 8 | config-added gate command (delete-work-unit) holds | 0.05 | 0.90 | 0.05 | gate.commands=[update-work-unit-status, delete-work-unit] ⇒ Hold |
| 9 | state includes title/epic | 0.90 | 0.05 | 0.05 | assert captured request state mentions AUTH-001 + "User Login" + "authentication" |
| 10 | missing work-units.json ⇒ raw state | 0.90 | 0.05 | 0.05 | state names command+args, no title; review still happens |
| 11 | disabled config bypasses gate | — | — | — | rlcd.enabled=false ⇒ Execute, 0 classifier hits |
| 12 | thin-margin proceed ⇒ ask_user | 0.50 | 0.12 | 0.38 | argmax proceed but margin 0.12 ≤ 0.15 ⇒ AskUser |
| 13 | P(hold) ≥ holdThreshold with other argmax ⇒ hold | 0.50 | 0.70 | 0.80 | P(hold) 0.70 ≥ 0.65 ⇒ Hold (priority over ask_user) |
| 14 | holdThreshold=0.95 in config ⇒ same answer asks | 0.50 | 0.70 | 0.80 | P(hold) 0.70 < 0.95, argmax ask_user ⇒ AskUser |

Notes:
- Scenario 13 proves the threshold path independently of argmax: with the
  default 0.65, `P(hold)=0.70` trips the threshold even though ask_user is
  argmax; with a raised threshold (14) the same answer drops to AskUser.
- Scenario 12 proves the conservative default: argmax proceed with a thin
  margin never auto-proceeds.
- The gate's ONE choice question criteria object is always
  `{"proceed": "Proceed with the command", "hold": "Hold — bad timing or risky", "ask_user": "Ask the user"}`.
