# Epic Review: BUG-149 — Live tool output not folded into TUI card (empty tool_call_id)

**Date:** 2026-06-30
**Reviewer:** Claude Code (fspec review skill, worker-executed)
**Work Units Reviewed:** 1 (standalone bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 3 (1 fixed, 2 accepted)
- 🟢 Observations: several (informational)

## Work Unit Result

### BUG-149: Live tool output not folded into TUI card — PASS

The fix is runtime-correct, reaches the real TUI path, preserves session isolation (RPC-398/BUG-126
exact-match, no global fallback added), and introduces no `unwrap()/expect()/panic!/todo!/unimplemented!`.
All 7 scenario tests pass; RPC-398 and the fspec-tui streaming parity suites remain green.

## 🔴 Critical Issues
None. Verified directly:
- **Runtime-correct set/clear.** SET (`*g = Some(tool_call.id.clone())`, stream_loop.rs:915-917)
  is inside the real ToolCall arm, after `handle_tool_call(...)` and before
  `tool_execution_in_progress = true;`. CLEAR (`*g = None`, 985-987) is inside the real
  ToolResult arm (`handle_tool_result` at 946), before `tool_execution_in_progress = false;`.
- **Non-empty id at runtime.** Emit closure (486-491) reads the holder and passes `&id`; during
  serial tool execution the holder is `Some(tool_call.id)`, so the real provider id reaches the
  wire and the TUI exact-id fold matches.
- **WHO CALLS THIS.** `run_agent_stream_internal` is the single impl behind public
  `run_agent_stream_with_images`, invoked from `agent-loop/src/dispatch.rs:88` and `agent_loop.rs`
  (+ NAPI mirror) — the live fspec-tui path. Fix is reachable end-to-end.

## 🟡 Warnings
1. **Gherkin step ordering invalid in feature 1, scenario 3 (stray-emit)** — had a `Given` after a
   `Then` (`Given/When/Then/Given/Then`). → ✅ **FIXED**: scenario rewritten as
   "Stray progress with no active tool call is dropped without panic" with valid ordering
   (`Given/When/Then/Then`); test `@step` comments realigned; coverage relinked. Feature valid.
2. **CLI tests are source-shape string scans, not behavioral** — asserting on the text of
   stream_loop.rs rather than executing the private generic loop. → ⚪ **ACCEPTED**: no seam to drive
   `run_agent_stream_internal` in isolation (private, generic over `<M,O,E>`, needs a live rig stream
   + Session with no id field). TUI-side behavior IS covered behaviorally in the fspec-tui suite;
   the scans correctly pin the anti-regression (no empty-id emit, active-id threaded). Brittle to
   refactor but defensible — same precedent as RPC-398's registration-key tests.
3. **Scenario "card unchanged" delegated to the fspec-tui suite via comment** — the CLI stray-emit
   test proves "no panic" and points at `progress_with_empty_id_matches_no_card_and_is_dropped`
   for the card-unchanged assertion. → ⚪ **ACCEPTED**: claim is true and behaviorally proven in the
   sibling crate; indirect but acceptable given the crate split.

## 🟢 Observations
- `unwrap_or_default()` in the emit closure is `Option::unwrap_or_default` (yields empty String when
  no tool active), NOT `.unwrap()` — cannot panic, compliant, clippy-clean.
- Architecture note [0] retains the boilerplate "Uses bcrypt... N/A" prefix — cosmetic only.
- Provider-selection verdict (Section E): **CONFIRMED unrelated.** The two failing lib tests
  (`test_start_interactive_mode_and_see_startup_card`, `test_switch_provider_during_session`) fail
  with "Multiple providers are credentialed but none was explicitly selected" because they call
  `ProviderManager::new()` which reads the host's real credentials (`~/.codelet`, `~/.fspec`) beyond
  the 2 env keys they set. The stream_loop diff adds ONLY an `Arc<Mutex<Option<String>>>` holder +
  emit-id threading — zero references to ProviderManager/provider selection — and is downstream of
  provider selection. Pre-existing, environment-dependent flake; not introduced by this change.

## Coverage Verification
- Feature files:
  - `spec/features/stream-loop-threads-active-tool-call-id-into-progress.feature` (3 scenarios, EMIT side) — OK
  - `spec/features/tool-progress-carries-tool-call-id.feature` (3 scenarios, MATCH side) — OK
- Test files:
  - `codelet/cli/tests/tool_progress_tool_call_id_bug149.rs` (4 tests) — OK
  - `codelet/fspec-tui/tests/tool_progress_tool_call_id_bug149.rs` (3 tests) — OK
- Impl files:
  - `codelet/cli/src/interactive/stream_loop.rs` (465-493, 913-988) — OK
  - `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs` (199-226, unchanged match side) — OK
- Scenario coverage: 6/6 (100%); audit-coverage: all files found on both features.

## Build & Test Results
- `cargo test -p codelet-cli --test tool_progress_tool_call_id_bug149` → 4 passed
- `cargo test -p codelet-fspec-tui --test tool_progress_tool_call_id_bug149` → 3 passed
- `cargo test -p codelet-cli --test tool_progress_registration_key_rpc398` → 3 passed (RPC-398 intact)
- `cargo test -p codelet-fspec-tui` → all 214 test binaries pass (parity/collapse suites green)
- `cargo build -p codelet-cli -p codelet-agent-loop` → OK
- `cargo clippy -p codelet-cli` → clean
- `cargo fmt` → applied

## Files Reviewed
- codelet/cli/src/interactive/stream_loop.rs
- codelet/cli/tests/tool_progress_tool_call_id_bug149.rs
- codelet/fspec-tui/tests/tool_progress_tool_call_id_bug149.rs
- codelet/fspec-tui/src/store/agent_view/chunk_processor.rs
- codelet/cli/src/interactive/agent_runner.rs, output.rs, stream_handlers.rs
- codelet/agent-loop/src/dispatch.rs, agent_loop.rs, background_output.rs
- spec/features/stream-loop-threads-active-tool-call-id-into-progress.feature
- spec/features/tool-progress-carries-tool-call-id.feature
- spec/attachments/BUG-149/investigation.md

## Fix Results
- 🟡 Warning 1 (invalid Gherkin ordering) → ✅ Fixed (scenario rewritten, tests realigned, coverage relinked, feature valid).
- 🟡 Warning 2 (source-shape CLI tests) → ⚪ Accepted (no isolable seam; behavior covered TUI-side).
- 🟡 Warning 3 (delegated card-unchanged assertion) → ⚪ Accepted (proven in sibling crate).

## Final Verification
- All BUG-149 tests pass ✅ · RPC-398 regression pass ✅ · fspec-tui suite pass ✅
- Build ✅ · clippy clean ✅ · fmt applied ✅
- Feature files valid ✅ · Coverage 6/6 (100%) · audit-coverage all files found ✅
- Provider-selection failures confirmed pre-existing & unrelated ✅
