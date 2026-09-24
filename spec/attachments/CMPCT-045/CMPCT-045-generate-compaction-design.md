# CMPCT-045 Design — GenerateCompaction Tool (DeepSearch Clone)

## Goal

Clone the **DeepSearch tool** into a new **GenerateCompaction** tool. When the
tool runs it:

1. Takes an **optional target session id** (argument `session_id`). When
   omitted, the target is the **calling session** (the session whose tool
   registry entry invoked the tool).
2. Runs the **existing DAG compaction algorithm** against that target session
   in a **clean ephemeral sub-agent** (DeepSearch pattern — the sub-agent's
   own context is tiny; it surveys the target session exclusively via
   SessionSearch).
3. Returns the **DAG as the tool result string**, and the **handler side
   (production code, not the LLM) pins the DAG to the target session** —
   i.e. performs the `apply_pending_dag` mutation (clear to reminders, push
   the wrapped DAG, recalculate the token tracker).

This makes compaction an **explicit, on-demand tool call** the agent can make
any time (and a sub-agent can invoke on a *different* session) instead of
only the passive error-cascade recovery of CMPCT-044.

## DeepSearch sections to clone (the template)

| What | Where | Notes |
|---|---|---|
| Tool definition + registry | `rust/tools/src/deep_search/mod.rs` | `DeepSearchTool` (lines 382–493), `DeepSearchHandler` type (line 215), `DEEP_SEARCH_HANDLERS` static (line 227), `set_deep_search_handler` / `has_deep_search_handler` / `clear_all_deep_search_handlers` (lines 234–287). Copy 1:1: `GenerateCompactionTool`, `GenerateCompactionHandler`, `GENERATE_COMPACTION_HANDLERS`, `set_generate_compaction_handler` / `has_generate_compaction_handler` / `clear_all_generate_compaction_handlers`. |
| Tool args | `DeepSearchArgs` (`rust/tools/src/deep_search/mod.rs:79`) | `GenerateCompactionArgs { session_id: Option<String> }` — optional UUID string of the target session; `None` ⇒ calling session. (Validation: parse as `uuid::Uuid`; on parse error return `ToolError::Validation` with usage hint, mirroring the empty-query validation at line 463.) |
| Sub-agent spawner | `rust/agent-loop/src/deep_search_handler.rs` | `execute_deep_search` (line 120): ephemeral `Uuid::new_v4()` (line 133), SessionSearch handler registration + `SessionSearchCleanup` drop guard (lines 144–153), provider inheritance via `ProviderManager::with_provider_and_model` (BUG-102, lines 535–541), `build_and_run_agent` with the 7 read-only tools (line 435; `SUB_AGENT_TOOL_COUNT == 7` compile-time assert at line 451), AMGR-016 wall-clock timeout via `codelet_cli::interactive::deep_search_wall_clock_timeout()` (line 234). `generate_compaction_handler.rs::execute_generate_compaction` copies this structure 1:1 with the compaction prompt instead of the DeepSearch system prompt. |
| Per-session registration | `rust/agent-loop/src/bridges.rs` | `register_deep_search_handler` (line 32) captures `facade_override()`/`current_provider_name()`, `current_model_id()`, `raw_model_context_window()`, `raw_model_max_output_tokens()` from the inner `Session` and registers the handler closure. Copy: `register_generate_compaction_handler` with the same capture; the closure also captures a means to reach the **target session's in-memory messages** (see below) and the owning `SessionManager` (needed to lock other sessions' `inner`). |
| Registration site | `rust/agent-loop/src/agent_loop.rs:740` | `register_deep_search_handler(session.id, &inner_session, project_path)` is called at session creation (and after `/model`/`/provider` changes). Add `register_generate_compaction_handler(...)` immediately beside it. |
| End-of-turn cleanup | `rust/agent-loop/src/agent_loop.rs:1507` | `codelet_tools::set_deep_search_handler(session.id, None)` in the cleanup block (lines 1501–1517). Add `set_generate_compaction_handler(session.id, None)` beside it. |
| Tool-surface source of truth | `codelet_tools::SUB_AGENT_TOOL_NAMES` (line 64) | The sub-agent keeps exactly the 7 read-only tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) — no `inject_summary`, no Write/Edit. The pin is handler-side (CMPCT-044 Rule [3] / design Option A). |
| Registry tests | `rust/tools/src/deep_search/tests.rs` (handler mock at line 358) | Copy the mock-handler registry test pattern for the new registry. |

## Existing DAG compaction algorithm (what the tool runs)

| What | Where | Notes |
|---|---|---|
| Compaction instructions | `rust/cli/src/compaction_dag.rs` | `COMPACTION_INSTRUCTION_FRESH` (line 34) and `COMPACTION_INSTRUCTION_INCREMENTAL` (line 96). The sub-agent prompt adapts these: "Your conversation history" → "Session `<target-uuid>`" (every SessionSearch call passes `session_id: <target-uuid>` explicitly — the sub-agent's *own* session is its ephemeral one), and the final step changes from "Call inject_summary(content)" → "Output the complete DAG (all `<dag-node>` blocks + `<dag-files>` section) as your final response; do not call any other tool after." |
| FRESH vs INCREMENTAL selection | `detect_existing_dag` (`rust/cli/src/compaction_dag.rs:146`) | Scan the **target session's messages** for the `<!-- type:compaction-dag -->` marker; `Some((content, max_turn_end))` ⇒ INCREMENTAL with existing DAG embedded, `None` ⇒ FRESH. Must be captured **before** any clear. For the calling session the handler already holds `&Session`; for another session it loads via the `SessionManager` (in-memory `inner` under lock) or, if the target is not a live session, from persistence (`load_session` + `get_session_messages_full`, the same path SessionSearch's `handle_show` uses — `rust/agent-loop/src/session_search_handler.rs`). |
| Pin mutation (handler-side) | `rust/agent-loop/src/inject_summary_handler.rs:245` (`apply_pending_dag`) | The exact clear-and-pin sequence: `reset_session_to_reminders` → push `wrap_dag_content(dag)` as a User message → `recalculate_token_tracker`. Primitives: `reset_session_to_reminders` (`rust/cli/src/interactive_helpers.rs:235`), `recalculate_token_tracker` (line 205), `wrap_dag_content` / `parse_dag_nodes` (`rust/core/src/compaction/model.rs`). The tool's pin is `apply_pending_dag`'s body with the DAG source being the sub-agent's final response instead of `pending_dag_content`. |
| Free fallback | `rust/cli/src/compaction_dag.rs:237` (`force_inject_fallback_dag`) | On sub-agent timeout / failure / unparseable output: extract partial `<dag-node>` blocks from the sub-agent's final text (string variant of `extract_partial_dag_nodes`, line 186 — reuse `DAG_NODE_BLOCK_RE`), else emit the generic `Auto-recovered: compaction timeout` D1 node (the Level-3 shape in `agent_loop.rs:1446-1465`), then pin it with zero LLM cost. |
| DAG parsing | `codelet_core::compaction::parse_dag_nodes` | Used to validate the sub-agent's returned text actually contains parseable `<dag-node>` blocks before treating it as a success; otherwise fall back. |

## Proposed flow

```
Agent (or any session) calls GenerateCompaction { session_id: Some(uuid) | None }
  ├─ target = session_id.unwrap_or(self.session_id)
  ├─ Handler lookup: GENERATE_COMPACTION_HANDLERS[target's registry entry]
  │    (registered per-session at agent_loop.rs:740 site; the closure captured
  │     provider/model/envelope + project path + SessionManager)
  ├─ Capture target's existing DAG (detect_existing_dag) BEFORE any clear:
  │    - calling session: from the registered closure's session access
  │    - other live session: SessionManager → inner.lock() → messages
  │    - non-live session: persistence load (SessionSearch handle_show path)
  ├─ Spawn ephemeral sub-agent (DeepSearch shape):
  │    Uuid::new_v4() → SessionSearch handler (project_path, compaction_trimming=true
  │    so Layer-0 trimming applies to its reads of the target) → drop guard
  │    → provider inheritance (BUG-102) → 7 read-only tools → AMGR-016 wall-clock
  │    timeout → prompt = FRESH/INCREMENTAL instruction + target UUID + task
  │    ("build the DAG, output it as your final response")
  ├─ Ok(dag_text):
  │    parse_dag_nodes validates ≥1 <dag-node>
  │    → pin under target lock: reset_session_to_reminders + push wrapped DAG
  │      + recalculate_token_tracker (+ reset_after_compaction on tracker)
  │    → clear compaction_in_progress
  │    → tool result = DAG text (so the caller sees what was pinned)
  └─ Err(timeout | parse | no handler):
       free fallback (partial nodes else generic node) → pin → tool result =
       fallback DAG text (structured note that it's a fallback)
```

## Locking discipline

- **Calling session**: the tool call runs *inside* the agent turn, so the
  stream loop is parked and the turn's `inner` lock is held by the agent
  loop — the pin for the calling session must run **after** the tool result
  is delivered, at a point where the handler can acquire the target's lock
  without self-deadlock. Concretely: the spawner returns the DAG to the
  tool result immediately; the pin for the *calling* session is performed
  by the handler at end-of-turn / inject point (same discipline as
  `apply_pending_dag_and_emit` in `agent_loop.rs`) — i.e. the handler
  stashes the result in `pending_dag_content` for the calling session and
  lets the existing end-of-turn `apply_pending_dag` path consume it, or pins
  directly once the turn's lock is released. For *other* sessions the pin
  runs immediately under the target's `inner` lock (no deadlock: the target
  is a different session than the one holding the current turn lock).
- **Target ≠ caller**: acquire target `inner` lock for (a) the
  `detect_existing_dag` capture and (b) the pin; never hold it across the
  sub-agent await.

## Flag & lifecycle discipline

- `compaction_in_progress` for the **target** session is set for the
  sub-agent run (gates SessionSearch Layer-0 trimming on the sub-agent's
  reads) and cleared after the pin — consistent with the in-view flow
  (CMPCT-044 Rule [5]).
- Emitted on the **target** session's output: `compaction_started` +
  `compaction_progress("Compaction sub-agent working", ...)` before spawn;
  `CompactionComplete` after the pin with the recalculated post-pin token
  basis (CMPCT-038 measurement rule); `compaction_failed(reason)` only if
  the fallback pin also fails.

## New/changed code inventory

| Item | Location |
|---|---|
| `GenerateCompactionTool` + `GenerateCompactionHandler` registry + args | `rust/tools/src/generate_compaction/mod.rs` (new; clone of `deep_search/mod.rs`) |
| `execute_generate_compaction(...)` spawner | `rust/agent-loop/src/generate_compaction_handler.rs` (new; clone of `deep_search_handler.rs`) |
| `register_generate_compaction_handler(...)` | `rust/agent-loop/src/bridges.rs` (beside line 32) |
| Registration + cleanup call sites | `rust/agent-loop/src/agent_loop.rs` (~line 740 and ~line 1507) |
| Compaction prompt builder (FRESH/INCREMENTAL, target UUID, "output DAG as final response") | `rust/cli/src/compaction_dag.rs` (`build_generate_compaction_prompt`) |
| Pin primitive callable with a DAG string (handler-side) | reuse `apply_pending_dag` body / `force_inject_fallback_dag` in `rust/cli/src/compaction_dag.rs` + `rust/agent-loop/src/inject_summary_handler.rs` |

## Out of scope (stays in CMPCT-044)

- The stream-loop error-cascade overflow classifier and automatic
  trigger wiring. CMPCT-045 builds the *tool* (explicit, on-demand
  compaction); CMPCT-044's trigger can reuse the same handler entry point
  when it is implemented.

## Test strategy (per ACDD phase)

- Tools-crate registry tests: mock `GenerateCompactionHandler` (mirror
  `deep_search/tests.rs:358`), set/has/clear lifecycle, arg validation
  (bad UUID → `ToolError::Validation`).
- Agent-loop spawner shape tests: ephemeral id ≠ target id, provider/model
  inheritance values, 7-tool surface (`SUB_AGENT_TOOL_COUNT`), timeout
  guard, handler cleanup (mirror `deep_search_handler/tests.rs`).
- Prompt-builder unit tests: FRESH when target has no DAG, INCREMENTAL with
  embedded DAG + `last_compacted_turn` when it does; target UUID present in
  both; "output DAG as final response" instruction present,
  `inject_summary` absent.
- Pin primitive unit tests on a real `Session`: messages replaced by
  reminders + wrapped DAG, `turns` cleared, tracker recalculated; fallback
  pin from partial/generic nodes.
- Feature file + scenarios generated from the Example Map before any code.
