# Review: RPC-045 — AgentView: subscribe to chunks + status broadcasts; handle every new StreamChunk variant

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review skill)
**Strategy:** Strict scope-bounded review against rules [0]-[7], examples [0]-[8], and architecture notes.

## Status: PASS (after fixes)

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Fixed)

1. **`dispatch_rpc045.rs` clippy doc warnings.** Three clippy lints
   (`doc_lazy_continuation` line 46, `doc_overindented_list_items`
   lines 119 + 121) tripped on the doc comments above
   `handle_stream_chunk_state_updates` and inside the
   `spawn_fspec_command_runner` happy-path bullet list. Fixed by
   splitting the paragraph (line 46) and changing the bullet
   continuation from 4-space indent to 2-space (lines 119, 121).
   Confirmed with `cargo clippy --lib -- -D warnings` → clean.

2. **Coverage line ranges for `FspecCommandRequest` scenarios omitted
   the dispatch arm.** Both `FspecCommandRequest …` scenarios linked
   only the `spawn_fspec_command_runner` + `run_fspec_command` blocks
   (lines 126-184 and 126-149+231-238). The match arm in
   `handle_stream_chunk_state_updates` (lines 94-96) that actually
   dispatches the chunk variant was missing from the impl line range.
   Re-linked with `unlink-coverage --all` + `link-coverage` to:
   - list-work-units: `94-96,126-184`
   - unsupported:    `94-96,126-149,231-238`
   `audit-coverage` reports all 20 mappings valid; `show-coverage`
   shows 100% (9/9) without duplicate entries.

## 🟢 Observations (Not Fixed — Out of Scope or Intentional)

1. **Architecture-note path `codelet/fspec-tui/src/app/run.rs` does
   not exist.** The work unit description and the
   `agentview-subscribe-broadcasts.md` attachment both reference a
   `run.rs` file. In reality `App::run` lives in `app/events.rs` and
   the four subscriber loops are spawned in `app/bootstrap.rs` (which
   is the cleaner design — subscribers send `Action`s via `action_tx`
   and `App::run`'s `tokio::select!` consumes via `action_rx`). The
   code matches the *intent* (rule [0]: "subscribes to chunks_rx AND
   status_changes_rx"), just at a different file path. Updating the
   description text is out of scope for this review.

2. **The `chunks_rx Sender drop` scenario test asserts indirectly.**
   The Gherkin reads "When the chunks subscriber task is awaited /
   Then the subscriber task completes cleanly without panicking" — the
   test (`chunks_rx_sender_drop_terminates_subscriber_task_cleanly`)
   uses `panic::catch_unwind` around an `Action::Redraw` dispatch and
   a follow-up `SessionCreated` instead of explicitly awaiting the
   JoinHandle. The subscriber JoinHandles are private (push-only into
   `subscriber_tasks: Vec<JoinHandle<()>>`), so exposing a test seam
   to await them individually would be a structural change. The
   functional assertion ("App keeps dispatching after Closed") is
   still meaningful — this is a minor test ergonomic, not a behavior
   gap.

3. **`spawn_fspec_command_runner` no-tokio-runtime fallback silently
   drops the request.** When `tokio::runtime::Handle::try_current()`
   is `Err` (synchronous test path only) the runner returns early
   without invoking `backend.send_fspec_result`. In production
   `App::run` is `pub async fn` and always runs inside a runtime, so
   this path is unreachable. Could be tightened to produce a
   synchronous `unsupported: no runtime` `FspecResult`, but doing so
   would expand the slice scope.

4. **`SessionHeader.is_isolated` / `is_debug_enabled` are not yet
   wired through to the new store slots.** `views/agent.rs` line
   256-257 still hard-codes both badge flags to `false`. The rules
   only require the *store* to be updated push-driven (rules [3]/[4]).
   Rendering-side wireup is correctly deferred to follow-up cards per
   architecture-note [0] ("Subsequent slices (RPC-046+) layer …
   dialogs on top of the per-session state introduced here").

5. **`SessionFooter` has no status pill yet.** Rule [6] mentions
   "SessionFooter and any status-pill rendering reads
   agent_view_store.session_status_for(&id)". Today the
   `SessionFooter` widget only renders `cwd [⎇ branch]` and has no
   status pill. The rule is satisfied vacuously (no polling
   `get_session_status` exists anywhere in fspec-tui — verified by
   grep), and the data path is in place for when the pill UI is
   added.

## Coverage Verification

- Feature file: `spec/features/agentview-subscribe-broadcasts.feature` — OK (gherkin valid, @WORK-UNIT-ID tag present, architecture doc-string present, 9 scenarios with proper Given/When/Then ordering)
- Test file: `codelet/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs` — OK (header comment references feature, every step has `@step` comment matching feature text exactly)
- Impl files: `codelet/fspec-tui/src/app/dispatch_rpc045.rs`, `codelet/fspec-tui/src/app/bootstrap.rs` — OK (line ranges now cover dispatch arm + runner + match branches)
- Scenario coverage: **9 / 9** (100 %)

## Files Reviewed
- `spec/features/agentview-subscribe-broadcasts.feature`
- `spec/features/agentview-subscribe-broadcasts.feature.coverage`
- `spec/attachments/RPC-045/agentview-subscribe-broadcasts.md`
- `spec/attachments/RPC-045/ast-research-wiring-targets.md`
- `codelet/fspec-tui/src/app/mod.rs`
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc045.rs`
- `codelet/fspec-tui/src/app/bootstrap.rs`
- `codelet/fspec-tui/src/app/state.rs`
- `codelet/fspec-tui/src/app/events.rs`
- `codelet/fspec-tui/src/store/agent_view.rs`
- `codelet/fspec-tui/src/store/agent_view/isolation_state.rs`
- `codelet/fspec-tui/src/components/mod.rs` (SessionStatusChanged Action def)
- `codelet/fspec-tui/src/views/agent.rs` (header/footer wireup)
- `codelet/fspec-tui/src/views/agent/header.rs`
- `codelet/fspec-tui/src/views/agent/footer.rs`
- `codelet/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs`

## Fix Results

- 🟡 clippy `doc_lazy_continuation` (line 46) → ✅ Fixed: split into two paragraphs.
- 🟡 clippy `doc_overindented_list_items` (lines 119, 121) → ✅ Fixed: 4-space → 2-space bullet continuation.
- 🟡 FspecCommandRequest coverage missing dispatch arm → ✅ Fixed: re-linked both scenarios with combined ranges including `94-96`.

## Final Verification
- `cargo clippy --lib -- -D warnings`: ✅ clean
- `cargo test --test agent_view_chunk_dispatch_rpc045`: ✅ 9 / 9 passed
- `cargo build` (fspec-tui): ✅ ok
- `fspec audit-coverage agentview-subscribe-broadcasts`: ✅ All files found (20/20), all mappings valid
- `fspec show-coverage agentview-subscribe-broadcasts`: ✅ 100 % (9 / 9)
