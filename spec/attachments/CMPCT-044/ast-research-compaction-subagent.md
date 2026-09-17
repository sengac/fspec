# AST Research — CMPCT-044 (Clean compaction sub-agent triggered on API context-overflow errors)

Feature: spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature

AST-based code structure research performed before the testing phase.
Tools: AstGrep (syntax-structure search) + targeted Read of the matched spans.

## 1. Error classifier module — `rust/cli/src/interactive/error_classifiers.rs`

```
AstGrep language=rust pattern='pub fn is_prompt_too_long_error($$$ARGS) -> $RET { $$$BODY }'
  → error_classifiers.rs:15  pub fn is_prompt_too_long_error(error_str: &str) -> bool
```

- `is_prompt_too_long_error(&str) -> bool` (line 15): pure string classifier; closed substring list, PROV-010 `budget_tokens` exclusion at the top. **The new `is_context_overflow_error` must live beside it and be a strict superset.**
- `is_transient_network_error(&str) -> bool` (line 73): its pattern list includes `"stream closed before completion"`, `"unexpected eof"`, `"incomplete message"` — the classification-order hazard from Rule [1].
- `classify_compaction_branch(&anyhow::Error, &Arc<Mutex<TokenState>>) -> CompactionBranch` (line 256): must run FIRST in the cascade (typed `PromptCancelled` downcast) — Rule [8] deferral.
- `extract_prompt_cancelled(&anyhow::Error) -> Option<&Vec<Message>>` (line 160): the structural-downcast pattern the new classifier should mirror for wrapped errors (walk `error.chain()`).

## 2. Stream loop error cascade — `rust/cli/src/interactive/stream_loop.rs`

```
AstGrep language=rust (content search) 'is_prompt_too_long_error|classify_compaction_branch|begin_compaction_recovery'
  → line 1688 classify_compaction_branch
  → line 1773 begin_compaction_recovery (Path C)
  → line 1806 is_prompt_too_long_error + has_compactable_turns (Path B)
  → line 1842 begin_compaction_recovery (Path B)
  → line 1982 is_transient_network_error (NET-001)
  → line 2127 strip_failed_tool_call_tail (BUG-170/185)
  → line 2159-2160 terminal arm (emit_error + return Err)
```

Cascade order today: stall → compaction-branch (PromptCancelled) → prompt-too-long → image → truncation → network → BUG-170 strip → **terminal**.
The new overflow branch slots in at the Path B position (lines 1806-1857) — replacing/extending `is_prompt_too_long_error` — which is already before NET-001 (line 1982). Rule [1] is satisfied by construction; tests must lock the ordering.

`in_loop_compaction_restart!` macro (line 640) owns `compaction_retry_count` + `MAX_COMPACTION_RETRIES` (Rule [8] budget shared with sub-agent escalation — last scenario).

`has_compactable_turns = !convert_messages_to_turns(&session.messages).is_empty()` (line 1810) — the "no compactable turns" scenario gate.

## 3. Compaction primitives — `rust/cli/src/interactive_helpers.rs`

```
AstGrep language=rust pattern='pub async fn execute_compaction($$$ARGS) -> $RET { $$$BODY }'
  → interactive_helpers.rs:547
AstGrep language=rust pattern='pub fn reset_session_to_reminders($$$ARGS) -> $RET { $$$BODY }'
  → interactive_helpers.rs:235  (usize, usize)
```

- `reset_session_to_reminders(&mut Session) -> (usize, usize)` (line 235): partition → clear → restore reminders → clear turns. **The pin primitive reuses this** (Rule [3]).
- `recalculate_token_tracker(&mut Session)` (line 205): re-derives input_tokens from post-clear messages.
- `execute_compaction` (line 547): in-view flow (orphan guard → detect_existing_dag → reset → inject FRESH/INCREMENTAL instruction → recalc). Reference for instruction selection (Rule [6]).
- `detect_existing_dag` lives in `crate::compaction_dag` (line 552-554 use): returns `(dag_content, max_turn_end)`.

## 4. Fallback DAG — `rust/cli/src/compaction_dag.rs`

```
AstGrep language=rust pattern='pub fn force_inject_fallback_dag($$$ARGS) { $$$BODY }'
  → compaction_dag.rs:237
```

- `force_inject_fallback_dag(&mut Session, &Arc<AtomicBool>, &str)` (line 237): reset_session_to_reminders → wrap_dag_content → push → recalculate → clear flag. **Directly reusable for the free fallback** (Rule [4], Rule: missing-handler scenario).
- `extract_partial_dag_nodes(&[Message]) -> Vec<String>` (line 186): recovers partial `<dag-node>` blocks — needed by the "partial blocks recovered" scenario. For the sub-agent case the partial blocks arrive as a STRING (sub-agent final text), so a string-based variant or `DAG_NODE_BLOCK_RE` reuse is required.
- `COMPACTION_INSTRUCTION_FRESH` (line 34) / `COMPACTION_INSTRUCTION_INCREMENTAL` (line 96): the two instruction templates the sub-agent task prompt must adapt (Rules: FRESH/INCREMENTAL scenarios).

## 5. DeepSearch sub-agent template — `rust/agent-loop/src/deep_search_handler.rs`

```
AstGrep language=rust pattern='pub async fn execute_deep_search($$$ARGS) -> $RET { $$$BODY }'
  → deep_search_handler.rs:120
```

- `execute_deep_search(project_path, query, scope, max_depth, provider_name, model_id, depth, max_recursion_depth, context_window, max_output_tokens) -> Result<String, String>` (line 120): the full ephemeral-sub-agent lifecycle — `Uuid::new_v4()` → `set_session_search_handler(ephemeral, create_handler(project_path, Arc(AtomicBool::new(false))))` + `SessionSearchCleanup` drop guard → provider dispatch (`ProviderManager::with_provider_and_model`, BUG-102) → 7 read-only tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch; `SUB_AGENT_TOOL_COUNT == 7` compile-time assert) → `tokio::time::timeout(deep_search_wall_clock_timeout, ...)` (AMGR-016) → final answer string.
- `build_and_run_agent` (line 435): provider match + `run_agent!` macro (non-streaming `RigAgent::prompt` except codex/zai streaming).
- `custom_provider_registered(provider_name)` branch (line 486): Rhai custom providers via `CustomProvider::create_rig_agent`.

**The compactor sub-agent spawner (`execute_compaction_subagent`) copies this structure 1:1** with: a compaction task prompt (parent session id embedded), the parent's `existing_dag`/`last_user_message` threaded into the prompt, and the same timeout + cleanup guards. Tool surface stays the 7 read-only tools (Rule [2], read-only scenario).

## 6. apply_pending_dag mutation — `rust/agent-loop/src/inject_summary_handler.rs`

```
AstGrep language=rust pattern='pub fn apply_pending_dag($$$ARGS) -> $RET { $$$BODY }'
  → inject_summary_handler.rs:245
```

- `apply_pending_dag(&mut codelet_cli::session::Session, &Arc<Mutex<Option<String>>>) -> Option<Vec<DagNodeMeta>>` (line 245): take pending DAG → optional auto-append `<dag-files>` → `parse_dag_nodes` → `reset_session_to_reminders` → push wrapped DAG → `recalculate_token_tracker`. **This is the exact clear-and-pin mutation the trigger performs handler-side** (Rule [3]). For the sub-agent path the pinned content comes from the sub-agent's final response instead of `pending_dag_content`; the same helper body applies.

## 7. Handler-registry pattern — `rust/tools/src/deep_search/mod.rs`

```
pub type DeepSearchHandler = Arc<dyn Fn(String, Option<String>, usize, usize)
    -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>
static DEEP_SEARCH_HANDLERS: Lazy<RwLock<HashMap<Uuid, DeepSearchHandler>>>
pub fn set_deep_search_handler(session_id: Uuid, handler: Option<DeepSearchHandler>)
pub fn has_deep_search_handler(session_id: Uuid) -> bool
```

**The new `CompactorSubAgentHandler` follows the identical registry pattern** in codelet-tools (new module `rust/tools/src/compaction_subagent.rs`):

```
pub type CompactorSubAgentHandler = Arc<dyn Fn(CompactorRequest)
    -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> + Send + Sync>;
```

where `CompactorRequest` carries: parent_session_id, project_path, provider_name, model_id, context_window, max_output_tokens, existing_dag (Option<(String, usize)>), last_user_message (Option<String>), api_error (String). Return: `Ok(dag_text)` or `Err(reason)`. Registry: `set_compactor_subagent_handler` / `has_compactor_subagent_handler` / `clear_all_compactor_subagent_handlers` (test hygiene, mirrors the DeepSearch/SessionSearch registries).

Why a registry (not a direct call): `codelet-cli` cannot depend on `codelet-agent-loop` (arrow: agent-loop → cli exists; the reverse would be a cycle, and cli must stay a leaf-ish shared engine). The stream loop therefore calls an optional handler resolved by `session_id`; the agent-loop crate registers the real spawner per session (mirroring `register_deep_search_handler` in `bridges.rs`); when absent, the trigger uses the free fallback (scenario: "Missing compactor handler still guarantees a reduced session").

## 8. Session struct — `rust/cli/src/session/mod.rs`

```
AstGrep language=rust pattern='pub struct Session { $$$FIELDS }'
  → session/mod.rs:31
```

Relevant fields for the pin: `messages: Vec<rig::message::Message>` (line 38), `turns: Vec<ConversationTurn>` (line 42), `token_tracker: TokenTracker` (line 46), `annotations` (line 51). `Session::new(Option<&str>)` (line ~109) for tests. The pin primitive takes `&mut Session` — the stream loop already owns it mutably; the agent-loop terminal arm acquires `session.inner.lock().await`.

## 9. Agent-loop terminal-error arm — `rust/agent-loop/src/agent_loop.rs`

- Lines 1527-1545: `if let Err(e) = result` → `StreamChunk::error` → `Idle` → `Done`. **No compaction recovery today** — the new terminal-error branch (background-session scenario) slots here, before the `StreamChunk::error` emit: classify the error with the new classifier; on match, run the compactor recovery entry point (pin under `session.inner` lock, or fallback) and continue to Idle.
- Line 724-726: per-session handler registration site (`set_session_search_handler` + `register_deep_search_handler`) — **the compactor handler registers here** (handler-lifecycle scenario) and is cleaned up at lines 1502-1517 (end-of-turn cleanup block).
- Lines 1400-1499 (CMPCT-020 watchdog): unchanged; coordinates via `compaction_in_progress` flag (flag-discipline scenario).

## 10. Persistence — what the sub-agent reads

- `agent-loop/src/persist.rs`: `persist_user_message`, `persist_assistant_message_internal`, `persist_tool_result_internal`, `persist_token_state` (REFAC-007) — turns are persisted to the session manifest **during** the stream (via `BackgroundOutput`), so by the time the overflow error fires, the parent's turns are on disk. SessionSearch (`agent-loop/src/session_search_handler.rs::create_handler`) reads them via `load_session` + `get_session_messages_full` (`codelet_core::persistence`). **Rule [2] is satisfiable: the sub-agent's SessionSearch over the parent session id sees the full persisted history.**

## Conclusions / implementation plan derived from AST research

| New code | Location | Reuses |
|---|---|---|
| `is_context_overflow_error(&anyhow::Error) -> bool` + widened string helper | `cli/src/interactive/error_classifiers.rs` | `is_prompt_too_long_error` substring list (superset), `extract_prompt_cancelled` chain-walk pattern |
| `pin_compaction_dag(&mut Session, &Arc<AtomicBool>, &str)` | `cli/src/interactive_helpers.rs` | `reset_session_to_reminders` + `wrap_dag_content` + `recalculate_token_tracker` (same body as `apply_pending_dag`) |
| Compactor task prompt builder (FRESH/INCREMENTAL, parent id, "output DAG as final answer") | `cli/src/compaction_dag.rs` | `COMPACTION_INSTRUCTION_FRESH` / `_INCREMENTAL` templates, `detect_existing_dag` |
| `CompactorSubAgentHandler` registry + `CompactorRequest` | `tools/src/compaction_subagent.rs` (new) | `deep_search/mod.rs` registry pattern |
| `execute_compaction_subagent(...)` spawner | `agent-loop/src/compaction_subagent.rs` (new) | `deep_search_handler.rs` 1:1 (ephemeral id, SessionSearch handler+drop guard, provider inheritance, 7 read-only tools, wall-clock timeout) |
| `register_compactor_subagent_handler(...)` | `agent-loop/src/bridges.rs` | `register_deep_search_handler` (facade_override, model, context window capture) |
| Stream-loop overflow branch (before NET-001, after PromptCancelled) | `cli/src/interactive/stream_loop.rs` | `begin_compaction_recovery` + `in_loop_compaction_restart!` (shared retry budget) |
| Agent-loop terminal-error recovery branch | `agent-loop/src/agent_loop.rs` | classifier + registry lookup + `pin_compaction_dag`/fallback under `inner` lock |

Test strategy per scenario (mapping to the 20 feature scenarios):
- Classifier scenarios → pure unit tests in `error_classifiers.rs` `#[cfg(test)]` + a dedicated `cli/tests/cmpct044_overflow_classifier_test.rs` (string corpus incl. provider variants, wrapped anyhow chains, exclusions).
- Cascade-ordering scenarios → shape tests (grep-locked ordering of classifier calls in stream_loop.rs source, the pattern established by `rpc084_streaming.rs` / `compaction_error_cascade_test.rs`) + a behavior test through a stub provider that yields the unmatched overflow error.
- Pin primitive scenarios → unit tests on a real `Session` (temp data dir via `set_data_directory`, `RecordingOutput` pattern from `compaction_error_cascade_test.rs`).
- Sub-agent scenarios → registry unit tests (tools crate, mock handler like `deep_search/tests.rs:358`) + spawner shape tests (agent-loop crate: ephemeral id, tool count, timeout, cleanup — mirroring `deep_search_handler/tests.rs`).
- Fallback scenarios → unit tests on `pin_compaction_dag` + fallback assembly (partial-block extraction from a string, generic node creation).
- Integration scenarios (lifecycle, handler registration, budget) → shape/structural tests (source-grep of registration/cleanup call sites in `agent_loop.rs`, macro-budget check) since a full LLM round-trip is out of scope for hermetic tests.
