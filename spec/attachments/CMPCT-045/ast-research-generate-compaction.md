# AST Research — CMPCT-045 (GenerateCompaction tool)

AstGrep analysis of the existing DeepSearch/compaction code that CMPCT-045 clones.
Captured during the specifying phase on 2026-09-17.

## 1. DeepSearch tool definition (clone template)

- `rust/tools/src/deep_search/mod.rs:397` — `impl Tool for DeepSearchTool { ... }`
  - `NAME = "DeepSearch"`, `Args = DeepSearchArgs`, `Output = String`, `Error = ToolError`
  - `call()`: HOOK-013 pre_tool_hook → arg validation (empty query → `ToolError::Validation`
    with `codelet_common::tool_usage::append_usage_to_message`) → `execute_deep_search(...)`
    (handler registry lookup per CALLING session_id) → `ToolError::Execution` on handler error.
- `rust/tools/src/deep_search/mod.rs:215` — `DeepSearchHandler` type alias:
  `Arc<dyn Fn(String, Option<String>, usize, usize) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>`
- `rust/tools/src/deep_search/mod.rs:227` — `static DEEP_SEARCH_HANDLERS: Lazy<RwLock<HashMap<Uuid, DeepSearchHandler>>>`
- `rust/tools/src/deep_search/mod.rs:64` — `SUB_AGENT_TOOL_NAMES: [&str; 7]` + `SUB_AGENT_TOOL_COUNT`
  (single source of truth; handler asserts count == 7 at compile time).

## 2. DeepSearch spawner (clone template)

- `rust/agent-loop/src/deep_search_handler.rs:120` — `pub async fn execute_deep_search(...)`
  1. `Uuid::new_v4()` ephemeral session (line 133)
  2. `session_search_handler::create_handler(project_path, Arc::new(AtomicBool::new(false)))`
     + `set_session_search_handler(ephemeral, Some(...))` + `SessionSearchCleanup` drop guard (144–153)
  3. optional recursion handler + GraphSearch handler (RLM-002 / KGRAPH-009 — NOT needed for compaction)
  4. `build_system_prompt` (DeepSearch) — replaced by `build_generate_compaction_prompt` (codelet-cli)
  5. AMGR-016: `tokio::time::timeout(deep_search_wall_clock_timeout(), build_and_run_agent(...))` (234–261)
  6. `build_and_run_agent` (line 435): custom-provider dispatch via `CustomProvider::create_rig_agent`
     (PROV-104), else `ProviderManager::with_provider_and_model(provider_name, model_id,
     context_window, max_output_tokens)` (BUG-102) + per-provider `build_and_run!` macro
     with the 7 read-only tools. `const _: () = assert!(SUB_AGENT_TOOL_COUNT == 7)` at line 451.
  7. `provider_uses_streaming_execution` ("codex" | "zai") → `collect_final_response_from_stream`
     with NET-001 transient-retry.

## 3. Per-session registration (clone template)

- `rust/agent-loop/src/bridges.rs:32` — `pub fn register_deep_search_handler(
    session_id: Uuid, inner_session: &codelet_cli::session::Session, project_path: std::path::PathBuf)`
  captures `facade_override().or(current_provider_name)`, `current_model_id()`,
  `raw_model_context_window()`, `raw_model_max_output_tokens()`; closure forwards to
  `execute_deep_search(..., depth=0, ...)`.
- Registration sites: `rust/agent-loop/src/agent_loop.rs:740` (session creation) and
  `rust/napi/src/session_bindings.rs:2078,2268` (model/provider changes — napi twin; the LIVE
  agent-loop twin re-registers at its own model-change site when one is added).
- End-of-turn cleanup: `rust/agent-loop/src/agent_loop.rs:1507` —
  `codelet_tools::set_deep_search_handler(session.id, None);`

## 4. Existing DAG compaction algorithm

- `rust/cli/src/compaction_dag.rs:34` — `COMPACTION_INSTRUCTION_FRESH` (in-view: agent calls inject_summary)
- `rust/cli/src/compaction_dag.rs:96` — `COMPACTION_INSTRUCTION_INCREMENTAL` with
  `{existing_dag_content}` + `{last_compacted_turn}` placeholders
- `rust/cli/src/compaction_dag.rs:136` — `COMPACTION_DAG_MARKER = "<!-- type:compaction-dag -->"`
- `rust/cli/src/compaction_dag.rs:146` — `pub fn detect_existing_dag(messages: &[Message]) -> Option<(String, usize)>`
- `rust/cli/src/compaction_dag.rs:171` — `DAG_NODE_BLOCK_RE` (Lazy<Regex>)
- `rust/cli/src/compaction_dag.rs:186` — `pub fn extract_partial_dag_nodes(messages: &[Message]) -> Vec<String>`
  (message-list variant; CMPCT-045 needs a STRING variant over the sub-agent's final response)
- `rust/cli/src/compaction_dag.rs:237` — `pub fn force_inject_fallback_dag(
    session: &mut Session, compaction_in_progress: &Arc<AtomicBool>, dag_content: &str)`
  = reset_session_to_reminders + push wrap_dag_content + recalculate_token_tracker + clear flag.
  **This is the exact shape of the CMPCT-045 pin primitive** (plus `reset_after_compaction`).
- `rust/cli/src/interactive_helpers.rs:235` — `pub fn reset_session_to_reminders(session) -> (usize, usize)`
  (partition_for_compaction → clear → restore reminders → clear turns)
- `rust/cli/src/interactive_helpers.rs:205` — `pub fn recalculate_token_tracker(session)`
- `rust/cli/src/interactive_helpers.rs:547` — `execute_compaction` (in-view flow): sets flag BEFORE
  detect_existing_dag (step order: flag → detect → reset → pick FRESH/INCREMENTAL → push instruction → recalc).
- `rust/agent-loop/src/inject_summary_handler.rs:245` — `apply_pending_dag`
  (take pending → auto-append dag-files → parse_dag_nodes → reset → push wrapped → recalc)
- `rust/agent-loop/src/inject_summary_handler.rs:323` — `apply_pending_dag_and_emit`
  (apply + `emit_post_injection_events(emit, original_tokens, compacted_tokens)`
  where compacted_tokens = recalculated `session.token_tracker.input_tokens`)
- `rust/agent-loop/src/agent_loop.rs:1333` — end-of-turn call site:
  `apply_pending_dag_and_emit(&mut inner_session, &session.pending_dag_content,
   session.pre_compaction_tokens.load(...), &|chunk| session.handle_output(chunk))`
  — this is the deferred-pin path CMPCT-045 uses for the CALLING-session target.
- `rust/agent-loop/src/agent_loop.rs:1446-1471` — Level-3 fallback shape (partial nodes else
  generic `Auto-recovered: compaction timeout` D1 node + `force_inject_fallback_dag`).

## 5. Session / manager primitives

- `rust/sessions/src/background_session.rs:295` — `pub inner: Arc<Mutex<codelet_cli::session::Session>>` (tokio Mutex)
- `rust/sessions/src/background_session.rs:453` — `pub compaction_in_progress: Arc<AtomicBool>`
- `rust/sessions/src/background_session.rs:458` — `pub pending_dag_content: Arc<std::sync::Mutex<Option<String>>>`
- `rust/sessions/src/background_session.rs:620` — `owning_manager() -> Option<Arc<SessionManager>>`
- `rust/sessions/src/session_manager.rs:640` — `get_session(&self, id: &str) -> Result<Arc<BackgroundSession>, String>`
- `rust/sessions/src/background_session.rs:884` — `snapshot_pre_compaction_tokens() -> u32`
- `rust/sessions/src/background_session.rs:929` — `get_status()/set_status(SessionStatus)`
- `rust/sessions/src/background_session.rs:968` — `handle_output(chunk: StreamChunk)`
- `rust/sessions/src/background_session.rs:1533` — `set_compaction_progress(Option<CompactionProgress>)`
- `rust/core/src/compaction/model.rs:210` — `TokenTracker::reset_after_compaction()`
- `rust/core/src/compaction/model.rs:386/454` — `wrap_dag_content` / `parse_dag_nodes`
- `rust/rpc-types/src/lib.rs:1411` — `StreamChunk` (CompactionComplete at 1483; no CompactionStarted
  chunk — start is signalled by `SessionStateChange(Compacting)` + polled `CompactionProgress`)
- `rust/agent-loop/src/background_output.rs:501-547` — StreamEvent→chunk mapping:
  CompactionStarted → `set_status(Compacting)` + `snapshot_pre_compaction_tokens()` +
  `SessionStateChange(Compacting)`; CompactionComplete → Idle + `compaction_complete(...)`.

## 6. Provider tool chains (wiring sites for GenerateCompactionTool)

- `rust/providers/src/claude.rs:552` `.tool(DeepSearchTool::new(session_id))`
- `rust/providers/src/openai.rs:465`
- `rust/providers/src/gemini.rs:211`
- `rust/providers/src/codex/mod.rs:423`
- `rust/providers/src/zai.rs:281`
- `rust/providers/src/copilot/rig_agent.rs:102`
- `rust/providers/src/custom/custom_provider.rs:270,294` (both build paths)

## 7. Test patterns to clone

- `rust/tools/src/deep_search/tests.rs:381` — mock `DeepSearchHandler` registry dispatch tests
  (set/has/clear lifecycle, call() dispatch, no-handler error, per-session isolation)
- `rust/agent-loop/src/deep_search_handler/tests.rs` — streaming-path, wall-clock-timeout,
  timeout-message shape tests
- `rust/cli/tests/compaction_recovery_unification_test.rs` — `RecordingOutput` fake implementing
  `StreamOutput` to assert emitted event counts (pattern for flag/event tests)
- `rust/cli/tests/compaction_real_integration_coverage_test.rs` — source-shape assertions
  (e.g. `src.contains("begin_compaction_recovery(")`) for wiring-site invariants
