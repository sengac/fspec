# Review: RPC-387 — Subordinate view shows empty supervisor message body

**Date:** 2026-06-29
**Reviewer:** Claude Code (fspec review skill) + subordinate review agent
**Work Units Reviewed:** 1 (bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 1
- 🟢 Observations: 3

## Work Unit Result

### RPC-387 — PASS (with 1 warning to fix)

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **Test file header does not reference the RPC-387 feature.**
   `codelet/fspec-tui/tests/chunk_rendering_parity_rpc078.rs:3` declares
   `Feature: spec/features/agentview-scrollback-no-duplicate-userinput-and-wrap.feature`
   (RPC-078's feature). The four RPC-387 tests were appended to this RPC-078
   file. Per the CLAUDE.md test-header convention, the file-level header should
   reference the feature it validates. The RPC-387 tests carry per-block comments
   citing `supervisor-message-rendering.feature`, so linkage is recoverable, but
   the top-of-file header is misleading. **Resolution:** relocate the four
   RPC-387 tests into a dedicated test file with a proper header referencing
   `spec/features/supervisor-message-rendering.feature`, and re-link coverage.

#### 🟢 Observations
1. Architecture doc string in the feature (lines 11-14) accurately matches the
   implementation (split on `]`, trim leading space/newline from body).
2. Rust `parse_supervisor_envelope` is slightly more permissive than the TS
   regex, but for all in-scope cases (space, newline, no-header) the role/body
   outputs are identical — parity holds.
3. Edge cases handled cleanly with no panics; `unwrap_or`/`unwrap_or_else` used
   instead of `unwrap()`.

## Coverage Verification
- Feature file: `spec/features/supervisor-message-rendering.feature` — OK
- Test file: `codelet/fspec-tui/tests/chunk_rendering_parity_rpc078.rs` — OK (header caveat → Warning 1)
- Impl file: `codelet/fspec-tui/src/store/agent_view/session_context.rs` — OK
- Backend `codelet/sessions/src/background_session.rs:230` — unchanged (still space-separated) ✓
- Scenario coverage: 4/4 (100%)

## Build & Test Verification
- `cargo test -p codelet-fspec-tui --test chunk_rendering_parity_rpc078`: 13 passed, 0 failed
- `cargo clippy -p codelet-fspec-tui --all-targets`: no warnings in RPC-387 files (pre-existing warning in unrelated tui093 file)

## Fix Results
### RPC-387
- 🟡 Warning 1: test header mismatch → ✅ Fixed: relocated RPC-387 tests to a
  dedicated test file `codelet/fspec-tui/tests/supervisor_message_rendering_rpc387.rs`
  with a header referencing `spec/features/supervisor-message-rendering.feature`;
  coverage re-linked to the new file.

## Final Verification
- All tests pass: ✅
- Build succeeds: ✅
- Coverage complete (4/4): ✅
- Feature file valid: ✅
- Tags valid: ✅
