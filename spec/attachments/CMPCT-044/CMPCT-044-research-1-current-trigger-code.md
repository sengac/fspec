# CMPCT-044 Research 1 — Current Compaction Trigger Code & The Missed-Error Gap

## Where compaction is triggered today

There are **four trigger paths** that all funnel into `begin_compaction_recovery`
(`rust/cli/src/interactive/recovery_compaction.rs`) and then the in-loop
`in_loop_compaction_restart!` macro (`rust/cli/src/interactive/stream_loop.rs:640`):

| Path | Trigger | Site |
|------|---------|------|
| **A** | Pre-prompt: `estimated_total > threshold` before streaming starts | `stream_loop.rs:299-351` (CTX-005) |
| **B** | API returns "prompt is too long" mid-stream — matched by **string classifier** | `stream_loop.rs:1806-1857` (Path B) |
| **C** | Hook cancel — `PromptError::PromptCancelled` in error chain, detected by **typed structural downcast** | `stream_loop.rs:1688-1788` (Path C, via `classify_compaction_branch`) |
| **D** | Gemini continuation exhaustion | `gemini_continuation.rs` (Path D) |

Plus the **CMPCT-032 post-loop safety net** (`stream_loop.rs:2213-2301`) which
catches any clean-exit path that left `compaction_needed=true` behind.

## The threshold side (works fine)

`CompactionHook` (`rust/core/src/compaction_hook.rs`) is a rig
`StreamingPromptHook`:

- `on_completion_call` (BEFORE each API call): `MAX(last_known_total, estimated_payload) > threshold` → sets `compaction_needed` + `cancel_sig.cancel()`.
- `on_stream_completion_response_finish` (AFTER each API call): captures real usage from the API.
- Threshold from `resolve_compaction_threshold` (per-model, user override > family default > legacy formula).

When the hook cancels, rig yields `PromptError::PromptCancelled` (in our patched
`rig-core`), which **is** caught: `extract_prompt_cancelled`
(`error_classifiers.rs:160`) walks the anyhow chain via `downcast_ref` and
`classify_compaction_branch` routes it to `Recover`. This path is solid
(CMPCT-025/026/032 hardened it extensively).

## The API-error side (the gap this card addresses)

Path B relies on `is_prompt_too_long_error` (`error_classifiers.rs:15`), a
**closed list of English substrings**:

```
"prompt is too long" | "maximum context length" | "context_length_exceeded"
| "too many tokens" | "exceeds the model"
| invalid_request_error AND (token OR maximum)
```

### Why this misses real errors

1. **Provider wording drift.** Every provider phrases overflow differently:
   - OpenAI: `This model's maximum context length is 200000 tokens. However, you
     requested...` (covered — "maximum context length"), but newer responses use
     `invalid_request_error` + `context_length` phrasings that miss the
     `invalid_request_error AND (token|maximum)` conjunct when the body says
     e.g. "the input length exceeds..."
   - OpenAI-compatible / Bedrock / Vertex / self-hosted: arbitrary bodies
     (`"context length exceeded"`, `"Input too long"`, `413`, `RangeNotSatisfied`
     from some gateways) — none matched.
   - `is_prompt_too_long_error` lowercases but does **not** look at HTTP status
     codes at all, so a 400/413 whose body doesn't contain any of the six
     substrings falls through.
2. **Error text is opaque after wrapping.** By the time rig yields the error,
   the original API message may be buried under `StreamingError::Prompt(Box<…>)`
   and `.context()` layers; `e.to_string()` on the outer error may render only
   the wrapper, not the provider body.
3. **The fallback is terminal.** Anything that misses Path B drops into the
   cascade order in `stream_loop.rs:1670-2161`:
   `stall → prompt_too_long → image → truncation → network → terminal`.
   An unmatched overflow error hits the terminal arm: `emit_error` +
   `return Err(...)` (lines 2159-2160), with only the BUG-170/185
   `strip_failed_tool_call_tail` mitigation — the session keeps its oversized
   context and dies on the next turn too. **This is the reported bug.**

### Classification-order hazard

Note the cascade order: `is_transient_network_error` is checked **after**
prompt-too-long, and its pattern list includes `"stream closed before
completion"` / `"unexpected eof"` — provider 400s that abort the SSE stream
mid-body can be *misclassified as network errors*, silently retried
`MAX_NETWORK_RETRIES` times against the same oversized payload, then terminal.
A context-aware classifier must run **before** the network-retry arm.

## What happens after Path B fires today (for contrast)

`begin_compaction_recovery` (in-place, same session, same agent):

1. Pop trailing User prompt (not consumed by API).
2. `flush_partial_state_before_compaction` — save partial assistant text +
   flush token tracker.
3. Set `compaction_needed`, clear tool progress callback, emit lifecycle events.
4. `in_loop_compaction_restart!`:
   - `execute_compaction` (`interactive_helpers.rs:547`): **in-view DAG flow** —
     partition messages → clear → restore system reminders → inject
     `COMPACTION_INSTRUCTION_FRESH`/`INCREMENTAL` as a **user message** →
     recalculate tracker. **No LLM call** at this point.
   - Re-issue the stream with "Continue" / resume prompt; **the same agent
     (same oversized-then-reduced context) must then build the DAG itself via
     SessionSearch and call `inject_summary`**.
   - Bounded by `MAX_COMPACTION_RETRIES = 3`.

So the current recovery *assumes the agent can continue inside the same
session*. If the context overflow was caused by content the in-view flow
cannot shed fast enough (or the agent itself errors again — the retry budget
is only 3), the session is stuck.

## The agent-loop layer (background sessions) is even thinner

`agent_loop.rs` (codelet-agent-loop) runs turns via
`codelet_cli::interactive::run_agent_stream_with_images` — so Paths A–D + the
post-loop net exist *inside* the stream for background sessions too. But:

- No **compaction watchdog equivalent for API-error triggers**: the existing
  CMPCT-020 watchdog (agent_loop.rs:1400-1499) only covers "compaction was in
  progress but the agent never called `inject_summary`" — it force-injects a
  fallback DAG. It does NOT cover "API error killed the turn before compaction
  ever started".
- Terminal `Err` from the stream becomes `StreamChunk::error` + `Idle`
  (agent_loop.rs:1527-1545) — the turn just dies; recovery is left to the
  next user message, which replays the same oversized context.

## The pieces that already exist for the proposed design

| Capability | Where | Status |
|---|---|---|
| Ephemeral sub-agent with clean session + 7 read-only tools | `deep_search_handler.rs` (`execute_deep_search`) | **Exists** — template to copy |
| SessionSearch handler for arbitrary (incl. ephemeral) sessions | `agent-loop/session_search_handler.rs::create_handler` | **Exists** |
| SessionSearch can read ANY session by id (incl. the dying one) | `SessionSearchAction::Show { session_id: Some(id) }` | **Exists** — `handle_show` resolves external session ids via `load_session` + `get_session_messages_full` |
| Clear-to-reminders primitive | `interactive_helpers.rs::reset_session_to_reminders` | **Exists** |
| DAG wrap/parse | `codelet_core::compaction::wrap_dag_content` / `parse_dag_nodes` | **Exists** |
| "Pin summary to session" primitive | `inject_summary_handler.rs::apply_pending_dag` (clear → restore → push wrapped DAG → recalc tracker) | **Exists** — exactly the "clear + pin" mutation needed |
| Fallback force-inject DAG (no LLM) | `compaction_dag.rs::force_inject_fallback_dag` | **Exists** — usable as the last-resort when the sub-agent itself fails |
| Per-session handler registry (SessionSearch, DeepSearch, InjectSummary) | `codelet_tools` static `RwLock<HashMap<Uuid, Handler>>` + drop-guard cleanup pattern | **Exists** |
| Provider/model inheritance for sub-agents | `ProviderManager::with_provider_and_model` (BUG-102) | **Exists** |
| Wall-clock timeout guard for sub-agents | `deep_search_wall_clock_timeout()` + `tokio::time::timeout` (AMGR-016) | **Exists** — copy |
| Compaction lifecycle UI events (Started/Progress/Complete/Failed) | `StreamOutput::emit_compaction_*` + `BackgroundOutput` handling | **Exists** |
| Typed PromptCancelled downcast | `error_classifiers.rs::extract_prompt_cancelled` | **Exists** — shows the robust-detection pattern |
| Session persistence of turns | `agent-loop/persist.rs` + `BackgroundOutput` (assistant/tool/token persistence) | **Exists** — turns survive into the manifest so SessionSearch can read them |

## Gaps (does NOT exist yet)

1. **A context-overflow classifier robust to provider wording** — no status-code
   awareness, no open-ended heuristics; a new `is_context_overflow_error`
   (or an expanded `is_prompt_too_long_error` + structured-error variant) is needed.
2. **A compaction sub-agent handler** (DeepSearch-shape): ephemeral session,
   SessionSearch tool aimed at the *parent's* session id, an inject-summary
   target bound to the *parent's* session, bounded retries, timeout guard.
3. **A "pin compaction DAG to an arbitrary session" primitive callable from
   outside the dying stream** — `apply_pending_dag` takes
   `&mut codelet_cli::session::Session`; the trigger site must obtain the
   session (lock `BackgroundSession.inner` in agent-loop, or `&mut Session`
   in the CLI stream loop) and run the clear-and-pin **synchronously with the
   stream parked** to avoid message-list races.
4. **Trigger wiring in the error cascade** — insert the new recovery branch
   in `stream_loop.rs` before the terminal arm (and before NET-001), plus the
   agent-loop terminal-error arm for background sessions.
5. **Persistence of the cleared+DAG session state** — after an out-of-band
   clear, `session.messages` (in-memory) and the persisted manifest must stay
   consistent for the next turn; today the DAG flow relies on
   `apply_pending_dag_and_emit` running inside `agent_loop` while holding
   `inner` — a sub-agent that mutates the parent's `Session` needs the same
   lock discipline.
6. **UI/UX** — the "compacting" badge/progress contract
   (`compaction-status-lifecycle`, `agentview-compaction-badge-auto-hide`)
   currently assumes in-session compaction; an external sub-agent doing the
   work must emit the same `CompactionStarted/Progress/Complete` chunks so the
   TUI/JS contract holds.

## Open risks

- **Locking**: `apply_pending_dag` needs `&mut Session`; in background sessions
  that means `session.inner.lock().await` in the agent-loop error arm. The
  stream loop (CLI) already holds `&mut Session`, so the CLI path is simpler.
  Must confirm the sub-agent's SessionSearch reads happen **before** the
  clear, and that clearing happens after the stream is fully parked.
- **Cost**: the sub-agent makes real LLM calls (SessionSearch + DAG building),
  unlike the in-view flow's zero-cost setup. Needs a bounded budget (max
  sub-agent turns / timeout) and a free fallback (force-inject partial DAG) on
  failure.
- **Double-compaction races**: if the in-view path also fires (e.g. hook
  cancel on the retry stream), both paths must coordinate via the existing
  `compaction_in_progress` flag.
- **Classifier scope creep**: widening Path B strings too aggressively can
  misfire on e.g. "maximum output tokens" (truncation, not context overflow) —
  PROV-010 already excludes `budget_tokens`; the new classifier must keep the
  same exclusions and prefer structured signal (status code + error body
  shape) over substring matching.
