# CMPCT-011 Post-Review: Remaining Fixes & Downstream Work

## Status: Done (with known deferred items)

Code review completed 2026-03-09. All feature file acceptance criteria are satisfied.
Everything compiles (`cargo check --workspace` clean) and all tests pass.

---

## Deferred to CMPCT-012: stream_loop Annotation Wiring

The `annotation_detector` module is built and tested but is not yet called from
any production code path. The per-turn-annotation-detection.feature scenarios
test the **detector logic** (input tool calls → output annotations), not the
stream_loop integration.

**What's needed (CMPCT-012 scope):**

1. In `stream_loop.rs` post-turn logic, convert the turn's tool call tracking
   data into `Vec<ToolCallInfo>` structs
2. Call `detect_annotations(&TurnContext { current, previous })` after each
   completed turn
3. Serialize `Vec<StructuralAnnotation>` into the persisted message metadata
   (e.g. as a `"annotations"` key in the metadata HashMap)
4. Track `previous_tool_calls` across turns for ErrorResolution detection

This was described in CMPCT-011's description and Arch Note [1], but the feature
file scenarios don't test it at the wiring level, and stream_loop.rs is still on
the legacy compaction path — CMPCT-012 is the card that migrates it.

---

## Deferred to CMPCT-012: stream_loop / repl_loop Legacy Migration

Both `stream_loop.rs` and `repl_loop.rs` still call `execute_compaction_legacy()`.
The old function was renamed (not deleted) to bridge until CMPCT-012 migrates
the threshold trigger to the in-view flow.

**Current state:**
- `execute_compaction()` (new, in-view flow) — used only by `session_compact()` NAPI binding
- `execute_compaction_legacy()` (old, batch LLM) — used by stream_loop and repl_loop

**What CMPCT-012 must do:**
- Thread `Arc<AtomicBool>` through `run_agent_stream` call chain to reach
  `execute_compaction()` from stream_loop
- Update repl_loop similarly
- Both callers switch from `_legacy` to the new signature

---

## Deferred to CMPCT-013: Legacy Code Deletion

After CMPCT-012 validates the in-view flow end-to-end:

- Delete `execute_compaction_legacy()` from `interactive_helpers.rs`
- Delete `deprecated.rs` (PreservationContext, BuildStatus)
- Delete `selector.rs` (TurnSelector)
- Delete LLM anchor detection in `anchor.rs`
- Remove `#[allow(dead_code)]` on `persist_compaction_state()` and
  `persist_anchor_point()` — delete them
- Remove `#[allow(deprecated)]` from `compactor.rs`

---

## Minor Code Quality Notes (non-blocking)

### 1. `apply_conditional_trimming()` is test-only

The free function in `session_search_handler.rs` is only used in tests.
Consider moving it to `#[cfg(test)]` or removing it — the `ConditionalTrimmer`
struct is the real production API.

### 2. Token recalculation duplication

Both `execute_compaction()` and the inject_summary handler independently
iterate messages and call `count_tokens()` to recalculate
`session.token_tracker.input_tokens`. Could extract a shared
`recalculate_token_tracker(session)` helper. Low priority.

### 3. Test placement

The `execute_compaction` tests live in `inject_summary_handler.rs`'s test module
(codelet-napi crate) because they need `Session::from_provider_manager()`.
Slightly awkward but functional — no action needed unless tests are reorganized.
