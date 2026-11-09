# TUI-119 design (draft, from TUI-115)

The per-session queued list lives in `rust/fspec-tui/src/store/agent_view/busy_inputs.rs` (`BusyInputs { steers, queued }`). Today an entry leaves the list only when the session echoes the oldest queued text as `StreamChunk::UserInput`.

1. On `StreamChunk::Interrupted { queued_inputs }` / `InterruptedWithReason { queued_inputs }`, replace the list with the session's authoritative texts (oldest first), keeping the `n` of entries that survive in order, so the `Queued #n` transcript lines still match.
2. On a session reset (`SessionContext::reset_scrollback`, /clear), clear both steers and queued entries.

## Implementation (2026-11-02)

- Feature: spec/features/queued-list-resyncs-from-interrupts-and-clears-on-reset.feature (6 scenarios), tests in rust/fspec-tui/tests/tui119_queued_list_resync.rs (reuses tests/tui115_support).
- Correction to step 1: live `Interrupted`/`InterruptedWithReason` chunks are transcript content, so the App routes them through `TranscriptAction::Live` → `App::handle_live_transcript_wake`, not `record_chunk`. Transcript pages replay journal interrupts with an empty `queued_inputs` (sessions/src/replay_transcript.rs), and those can be historic. So the resync runs only on the live path (`SessionContext::apply_live_interrupt_queue` → `BusyInputs::resync_queued`). A scenario guards that a replayed interrupt leaves the list alone.
- Numbering: `next_queued_n` is one past the HIGHEST listed n (a resync can put a new entry ahead of an older-numbered one); `record_queued` uses it too.
- Step 2: `SessionContext::reset_for_clear` (reset_scrollback + empty BusyInputs) replaces `reset_scrollback` in the two /clear paths: `AgentView::reset_local_scrollback` (local /clear) and `App::handle_session_cleared` (session_cleared event). `recover_transcript_generation` still calls plain `reset_scrollback` and keeps the list, because the session still holds those inputs.
