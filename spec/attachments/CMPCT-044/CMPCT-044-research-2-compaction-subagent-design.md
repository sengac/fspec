# CMPCT-044 Research 2 — Design: Clean Compaction Sub-Agent (DeepSearch-Style)

## Goal

When the API rejects a request because the context is too large — in *any*
wording the string classifiers miss — a **separate, fully clean compaction
agent** (an ephemeral sub-agent, shaped like DeepSearch) takes over:

1. It reads the over-full session **via SessionSearch** (its own context stays
   tiny).
2. It builds the DAG summary (FRESH or INCREMENTAL, same contract as today's
   in-view DAG flow).
3. The parent session is **cleared and the DAG is pinned to it** (the
   `inject_summary` mutation, `apply_pending_dag`).
4. The parent's stream loop resumes with a fresh, reduced context.

The compaction intelligence runs in a **separate context** from the poisoned
one. This decouples "can the model still operate?" from "is the context
recoverable?" — today the same (already overflowing) agent is asked to build
the DAG, which is exactly what fails when the overflow error is unmatched.

## The DeepSearch template (what we copy)

`rust/agent-loop/src/deep_search_handler.rs::execute_deep_search`:

```
1. Uuid::new_v4()                       ← ephemeral session id (clean context)
2. set_session_search_handler(ephemeral_id, create_handler(project_path, Arc(AtomicBool(false))))
   + SessionSearchCleanup drop guard
3. build_system_prompt(scope)           ← compaction variant of the instruction
4. ProviderManager::with_provider_and_model(provider, model)  ← inherit parent's LLM
5. Build rig agent with read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash,
   SessionSearch (+ DeepSearch when recursing)
6. tokio::time::timeout(wall_clock, build_and_run_agent(...))  ← AMGR-016 guard
7. On completion: drop guards clean up handlers; return final answer string
```

The returned "final answer" for DeepSearch is the answer text. For the
compaction sub-agent it is **the DAG content** — the agent's last tool call
is `inject_summary`-equivalent, but bound to the *parent* session (see below),
or the handler extracts the `<dag-node>` blocks from the final response.

## Differences from DeepSearch (what's new)

### 1. Tool surface: SessionSearch + (optionally) read-only code tools, NO inject_summary in the toolset

Two options for how the sub-agent "pins" the result:

- **Option A (recommended): handler-side pin, not a tool call.**
  The sub-agent's job ends with producing the DAG as its final response text
  (DeepSearch pattern — no extra tool, no new per-session handler registry
  entry). The *handler* (parent-side code, not the LLM) then runs
  `reset_session_to_reminders(parent)` + push `wrap_dag_content(dag)` +
  `recalculate_token_tracker` — i.e. `apply_pending_dag`'s mutation — under
  the parent session lock. This is strictly safer: the sub-agent cannot
  mis-target the pin, cannot loop, and the pin happens exactly once, in
  production code. The sub-agent's system prompt says "output the complete
  DAG as your final answer, do not call any other tool after".
- **Option B: register an `inject_summary` handler for the ephemeral session
  that captures `(parent_session_id, dag)` into a `Mutex<Option<String>>`.**
  Closer to the in-view flow's semantics (the agent explicitly "pins"), and
  reuses `InjectSummaryTool` as-is. Cost: a new handler registration + the
  handler's return contract must not claim it mutated a session it doesn't own.

Option A keeps the sub-agent a pure *reader* (7 read-only tools, exactly the
DeepSearch base set) and puts all mutation in the trusted parent-side code.
Option B needs no prompt discipline but adds a tool-surface + registry change.
**Recommend A for v1; B if prompt-discipline proves unreliable in testing.**

### 2. SessionSearch target = the parent session

`SessionSearchAction::Show { session_id: Some(parent_id), max_turns, start_turn,
end_turn }` and `Search { query, ... }` already resolve **external** session
ids (`handle_show` → `load_session(target_id)` → `get_session_messages_full`).
The ephemeral session's handler is created with the same `project_path` as the
parent, so its `recent`/`search` scans include the parent's manifest. The
sub-agent can therefore survey the dying session exactly like a
cross-session researcher.

One subtlety: the sub-agent's **own** turns also get persisted (its own
session manifest), so `Show "current"` vs `Show <parent_id>` must be explicit
in the prompt — the instruction should hard-code the parent's UUID in the task
("Use SessionSearch with session_id `<parent-uuid>` …"), because the
sub-agent's *current* session is its own clean one.

### 3. Trigger input: the parent's state at failure time

The trigger passes to the handler:
- `parent_session_id: Uuid` (for SessionSearch targeting + prompt embedding)
- `project_path: PathBuf`
- `provider_name` / `model_id` (inheritance, BUG-102 pattern)
- `context_window`, `max_output_tokens` (for prompt budget hints)
- `existing_dag: Option<(content, max_turn_end)>` — captured from
  `session.messages` via `detect_existing_dag` **before** the clear attempt,
  so the sub-agent gets the INCREMENTAL instruction when a DAG exists
- `last_user_message: Option<String>` — the prompt to resume after pinning
- The raw API error string (for the sub-agent's context — it can mention the
  overflow when summarizing the "current state" node)

### 4. Timeout + budget

- Wall-clock: reuse `deep_search_wall_clock_timeout()` (AMGR-016).
- Turn/depth: the sub-agent gets a fixed `max_depth` (e.g. 12–20 tool turns —
  enough for ~6–8 SessionSearch calls + DAG composition).
- On timeout/failure: **free fallback** — `force_inject_fallback_dag`
  (Level-3 of the CMPCT-020 watchdog, `compaction_dag.rs:237`), which
  builds a minimal DAG from any partial `<dag-node>` blocks or a generic
  "auto-recovered" node and pins it without any LLM. The session is still
  recoverable; it just loses detail. This makes the whole path
  convergence-guaranteed: either the sub-agent DAG or the fallback DAG,
  never an unrecoverable oversized context.

## Proposed flow (end to end)

```
API error E reaches stream_loop.rs error arm
  ├─ classify_compaction_branch(E) == Recover?            → existing Path C (unchanged)
  ├─ is_stall_timeout_error(E)?                           → terminal (unchanged)
  ├─ is_context_overflow_error(E)  [NEW classifier]
  │    (prompt-too-long OR structured status-code shape)
  │     AND has_compactable_turns?
  │      YES → begin_compaction_recovery(...)             (existing: partial-text save,
  │               pop trailing user prompt, lifecycle events)
  │            run COMPACTOR SUB-AGENT (new)
  │              ├─ spawn ephemeral session, SessionSearch→parent
  │              ├─ sub-agent builds DAG (FRESH or INCREMENTAL)
  │              ├─ on Ok(dag): parent lock → reset_session_to_reminders
  │              │               → push wrapped DAG → recalculate tracker
  │              │               (== apply_pending_dag mutation, run once)
  │              └─ on Err/timeout: force_inject_fallback_dag (free)
  │            re-issue stream with retry prompt (policy per CMPCT-028)
  │            continue (same error cascade governs the retry)
  │      NO  → continue to image/truncation/network classifiers (unchanged)
  ├─ is_image_content_error / is_truncated_tool_call      → unchanged
  ├─ is_transient_network_error                           → unchanged (BUT only after
  │                                                          overflow check — ordering!)
  └─ terminal                                              → unchanged (BUG-170/185 strip)
```

Ordering note: the overflow check must run **before** `is_transient_network_error`
because some providers' overflow aborts present as stream-level errors that the
network classifier would swallow (see Research 1, "Classification-order hazard").

## Where each trigger site plugs in

| Site | Access to parent `Session` | Notes |
|---|---|---|
| `stream_loop.rs` error arm (CLI + background, since agent-loop runs the CLI stream loop) | `&mut Session` in hand | Direct; `execute_compaction` already works here. Sub-agent spawn is an `async` call inside the same loop; stream is parked. |
| `agent_loop.rs` terminal-error arm (background sessions) | `session.inner.lock().await` | Must ensure the sub-agent's SessionSearch reads complete **before** the clear mutation; the clear runs under the lock, stream already returned. |
| Pre-prompt Path A (optional extension) | `&mut Session` | Path A already compacts in-view; only fall back to the sub-agent if in-view setup itself errors. |

## Session state & persistence discipline

- **Before** the sub-agent runs: nothing is cleared. The parent keeps its full
  `session.messages` (needed for SessionSearch reads AND for the
  `detect_existing_dag` capture). The in-view flow's "clear then instruct"
  sequence is **not** used here — the sub-agent works from disk.
- **The in-view flow's `compaction_in_progress` flag** must be set for the
  duration (it gates SessionSearch Layer-0 trimming — see
  `session_search_handler.rs` / `session-search-trimming` — so the sub-agent's
  reads get trimmed payloads, keeping its own context small).
- **After** a successful pin: the persisted manifest still holds the old
  turns (append-only history — that's *the point*: SessionSearch can
  drill down later via `[SessionSearch: turns X-Y]` references). Only the
  in-memory `session.messages` is replaced by reminders + DAG, exactly like
  today's `apply_pending_dag`.
- **Token tracker**: recalculated from the new message list
  (`recalculate_token_tracker`), and `reset_after_compaction()` for the
  cumulative-billing fields (same as `execute_compaction_and_capture_events`).
- **Persistence of the clear event**: no manifest rewrite is needed for the
  clear itself (the manifest is history; the session manifest's token state is
  updated via `update_session_tokens` on the next `persist_token_state`).
  This mirrors how the existing in-view DAG flow behaves — the manifest is
  never truncated.

## System prompt for the sub-agent (sketch)

Derived from `COMPACTION_INSTRUCTION_FRESH` / `_INCREMENTAL`
(`rust/cli/src/compaction_dag.rs:34-116`) with two changes:
- "Your conversation history" → "Session `<parent-uuid>`" — every
  SessionSearch call must pass `session_id: <parent-uuid>` explicitly.
- Final step: instead of "Call inject_summary(content)", → "Output the
  complete DAG (all `<dag-node>` blocks + `<dag-files>` section) as your
  final response. Do not call inject_summary — it is not available."

The `build_system_prompt` mechanism in `codelet_tools` (used by DeepSearch for
scope + learnings context) is the natural home for the compaction variant.

## Lifecycle events / UX

The trigger site emits the existing structured events so the TUI/JS contract
(`compaction-status-lifecycle`, `agentview-compaction-badge-auto-hide`) holds:
- `compaction_started` + `compaction_progress("Compaction sub-agent working", 0, total_turns)`
  before spawning (same emission discipline as `begin_compaction_recovery`).
- `compaction_continuing` + `CompactionComplete` after the pin
  (`emit_post_injection_events` contract, CMPCT-038 measurement basis:
  recalculated post-pin total, not the summary size).
- `compaction_failed(reason)` on sub-agent failure **only if the fallback
  also fails** — the fallback DAG path still emits `CompactionComplete`
  (the session IS reduced), matching the CMPCT-020 Level-3 behavior.

## Failure matrix

| Failure | Action |
|---|---|
| Sub-agent times out (AMGR-016 guard) | Fallback DAG (partial nodes if any, else generic node) → pin → resume |
| Sub-agent returns text with no parseable `<dag-node>` | Same as timeout: fallback (extract_partial_dag_nodes first) |
| Provider/model unavailable for sub-agent | Fallback DAG immediately (no LLM attempt) → resume |
| Parent session lock poisoned | `into_inner()` recovery (existing pattern in `begin_compaction_recovery`) |
| Fallback pin itself errors | Terminal error, session left as-is (current behavior) |
| Sub-agent itself hits context overflow (pathological huge sessions) | Bounded max_depth + timeout both apply; fallback catches it. (Its context only holds SessionSearch *previews*, so this is very unlikely.) |
| Retry after pin overflows again | Existing `MAX_COMPACTION_RETRIES = 3` budget applies to the whole
  overflow-recovery branch (in-view + sub-agent rounds counted together) |

## Relationship to existing mechanisms (what stays)

- **Paths A/B/C/D in-view flow**: unchanged for everything the hook catches
  (threshold-based) and for matched prompt-too-long strings. The sub-agent is
  the **second stage**: if in-view compaction's retry stream fails again with
  an overflow error, *that* error also routes here (the retry stream is
  governed by the same cascade — see `in_loop_compaction_restart!` comment
  "BUG 5 is impossible by construction"). So the sub-agent naturally becomes
  the escalation for repeated in-view failures without new wiring: the error
  cascade simply routes the second overflow to it.
- **CMPCT-020 watchdog**: unchanged — it governs "in-view compaction in
  progress but no inject_summary". If the sub-agent pins the DAG, the
  watchdog sees `has_pending_dag == false` and a cleared flag — no conflict.
- **BUG-170/185 tail strip**: unchanged, runs at terminal errors; the
  overflow branch now sits *before* it.

## New/changed code inventory (estimation basis)

1. `error_classifiers.rs`: `is_context_overflow_error(error: &anyhow::Error, status_hint: Option<u16>)`
   — structured + widened-string detection, reusing existing exclusions. ~50 LOC + tests.
2. `agent-loop/src/compaction_subagent.rs` (new, ~250 LOC): `execute_compaction_subagent(...)`
   — the DeepSearch-shaped spawner (ephemeral id, handler registration + drop
   guards, provider inheritance, timeout, prompt build, parse-or-fallback).
3. `stream_loop.rs`: new branch in the error arm (~80 LOC) — invoke
   `begin_compaction_recovery`, call the sub-agent, pin under the existing
   helpers, `in_loop_compaction_restart!`. Reuses the macro.
4. `compaction_dag.rs`: `build_subagent_system_prompt(existing_dag, parent_id, last_user_message)`
   (~40 LOC) — FRESH/INCREMENTAL variant with explicit session id.
5. `agent_loop.rs`: terminal-error arm hook for background sessions (~30 LOC)
   — reuse the same entry point; ensure `inner` lock ordering.
6. Tests: classifier proptests (provider-wording corpus), sub-agent unit test
   with a stub handler (handler registry is injectable — DeepSearch tests show
   the pattern), integration test: oversized fixture session + fake provider
   error → assert session cleared + DAG pinned + stream resumed; fallback
   path test (sub-agent failure → fallback DAG).
7. Feature file: `spec/features/compaction-subagent-on-api-overflow.feature`
   (~8 scenarios) + tag registration.

Rough estimate: **8–13 story points** (multiple components: classifier,
sub-agent handler, two trigger sites, lifecycle wiring, fallback guarantee,
tests). Fits under the 13-point cap as a single work unit; if the provider
error-surface research widens (e.g. status-code plumbing from rig's streaming
errors needs a patch), split the classifier into its own unit.
