# RPC-421 — Premature `/compact` reduction notice: root-cause dossier

**Date:** 2026-07-08 (restored after add-attachment self-copy truncation)
**Author:** Supervisor session (first-principles DeepSearch trace)
**Symptom:** Running `/compact` prints a `[compaction] ~9X% reduction (N → tiny tokens, 0 turns
summarised)` line **immediately**, long before the compaction has actually produced its DAG summary.
A second (accurate, post-CMPCT-038) notice arrives later from the `CompactionComplete` chunk. The
first line is fabricated data.

---

## 1. Trace of the `/compact` flow (embedded)

1. TUI slash handler (`codelet/fspec-tui/src/app/dispatch_slash_commands.rs`, Compact arm, ~line
   88-93): `backend.compact_session(session_id).await` → embedded transport → tarpc →
   `codelet/sessions/src/handle_impl.rs:261 compact_session`.
2. `handle_impl.rs:295`: `original_tokens = inner.token_tracker.input_tokens` (correct).
3. `handle_impl.rs:301-303`: `execute_compaction(...)` —
   (`codelet/cli/src/interactive_helpers.rs:533-622`) **clears** the message list down to system
   reminders + the compaction instruction (`reset_session_to_reminders`, lines 230-232 clear
   messages/turns) and recalculates the tracker (line 613).
4. `handle_impl.rs:310`: `compacted_tokens = inner.token_tracker.input_tokens` — **measured at the
   trough**: reminders + instruction only. The DAG summary does not exist yet.
5. `handle_impl.rs:327`: `send_input("Continue")` kicks the agent loop, which will (much later)
   build the DAG via SessionSearch and call `inject_summary`.
6. `handle_impl.rs:332-340`: returns
   `CompactionResult { compression_ratio: compression_ratio(original, compacted) * 100.0,
   turns_summarized: 0, turns_kept: 0, ... }` — a ~90-99% "reduction" describing an intermediate
   state that never survives, with admitted-fake turn counts ("0 by design", lines 336-339).
7. TUI slash handler Ok branch: `format_compaction_notice(&result)` → `Action::EmitSessionNotice`
   → **notice #1 (fabricated)** in scrollback.
8. Later: the agent's `inject_summary` fires `emit_post_injection_events`
   (`codelet/agent-loop/src/inject_summary_handler.rs:85-103`) → the single
   `StreamChunk::CompactionComplete` → `dispatch_stream_chunks.rs:126-158` → badge set + **notice #2
   (real numbers — accurate once CMPCT-038 lands)**.

The double emission is currently documented as intentional
(`dispatch_stream_chunks.rs` RPC-047 comment: "double-emission for /compact is acceptable parity
with the TS Ink original") and mandated by the RPC-047 spec (rule 2 + the "emits a success notice
... on Ok" scenario). Parity preserved the *shape* of the TS behaviour but the *content* of notice
#1 is fabricated in the DAG flow, so the parity argument no longer justifies it.

## 2. Why the engine side cannot simply "return the right number"

At RPC-return time the final compacted size is **unknowable** — the DAG summary is produced
asynchronously by the agent after `send_input("Continue")`. Blocking the RPC until injection would
hang `/compact` for the full DAG-build duration. Therefore the truthful design is:

- the RPC result answers "did compaction start successfully?" (Ok/Err),
- the `CompactionComplete` chunk is the **single source of truth for the numbers**.

## 3. Required fix

1. **`dispatch_slash_commands.rs` Compact arm, Ok branch**: stop emitting the success notice from
   the RPC result. Keep:
   - Err branch: `[error] /compact failed: {reason}` (unchanged),
   - no-current-session silent no-op (unchanged),
   - focused-session-only semantics (unchanged).
   The success notice now comes exclusively from the `CompactionComplete` chunk handler
   (`dispatch_stream_chunks.rs:155-158`), which fires for both `/compact` and auto-compaction —
   scrollback gets exactly **one** accurate `[compaction] ...` line per compaction.
2. **`format_compaction_notice`** stays single-sourced (may move next to its now-sole caller —
   implementer's choice; keep doc comments truthful either way).
3. **Amend the RPC-047 spec** (`spec/features/slash-command-compact.feature`) via fspec commands:
   - rule 2 → success notice originates from the `CompactionComplete` chunk; the slash handler emits
     no success notice from the RPC result;
   - update the "emits a success notice ... on Ok" scenario to assert exactly ONE notice, sourced
     from the chunk, and NO notice before the chunk arrives;
   - update the two-session scenario's notice expectations accordingly;
   - update the architecture doc-string paragraph describing the double emission;
   - rewrite the "double-emission is acceptable parity" comment in `dispatch_stream_chunks.rs`.
4. **Update tests**: `codelet/fspec-tui/tests/slash_compact_rpc047.rs` — the Ok-notice scenarios
   change from "RPC result produces the notice" to "chunk produces the sole notice"; add a
   regression test asserting NO `[compaction]` line exists after the RPC returns but before the
   chunk is dispatched, and exactly one after.
5. **Engine (`handle_impl.rs`) is out of scope** for behavior change: its return value remains (it
   still carries original_tokens and the started-successfully signal). Optionally doc-comment lines
   332-340 that consumers MUST NOT present `compression_ratio`/`compacted_tokens` from this result
   as final numbers in the DAG flow. Same note applies to the NAPI twin `session_bindings.rs:3129`
   (TS frontend behaviour itself is out of scope — observation only).

## 4. Interaction with sibling cards

- **Depends on RPC-420** (same files: `dispatch_slash_commands.rs`, `slash_compact_rpc047.rs`,
  RPC-047 feature file; RPC-420 fixed the unit math first, this card then removes the fabricated
  emission).
- **CMPCT-038** makes notice #2's numbers honest. This card makes notice #2 the only notice. All
  three together produce: one notice, right number, right badge.

## 5. Acceptance sketch (refine during Example Mapping)

- `/compact` Ok → NO immediate `[compaction]` line; when the `CompactionComplete` chunk arrives,
  exactly ONE `[compaction] X.X% reduction (...)` line lands in the originating session's scrollback.
- `/compact` Err → `[error] /compact failed: {reason}` (unchanged).
- `/compact` with no session → silent no-op (unchanged).
- Auto-compaction (chunk without prior slash command) → exactly one notice (unchanged behaviour).
- Focus switched away before chunk arrives → notice still lands on the originating session
  (existing `Action::EmitSessionNotice` routing, unchanged).

## 6. Verification commands

```bash
cd codelet && cargo test -p codelet-fspec-tui slash_compact
cargo test -p codelet-fspec-tui
cargo check --workspace
```
