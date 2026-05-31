# Review: RPC-056 — /blocklist view + blocklist RPC surface

**Date:** 2026-05-24
**Reviewer:** Claude Code (fspec review skill)
**Type:** Single-story review (no children — RPC-056 is a leaf of epic RPC-030)
**Status (pre-fix):** done
**Status (post-fix target):** done

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 5 (all documentation drift between example map and implementation)
- 🟢 Observations: 2

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)

### W1 — Rule [8] contradicts implementation and architecture note [2]
Rule [8] says:
> "The session-disabled set lives on BlocklistView itself (in-memory HashSet<String> of rule ids) — mirroring the TS AgentView state; toggling is a synchronous UI mutation with NO backend round-trip (TS parity — disabled state is purely UI)"

But the implementation in `codelet/fspec-tui/src/views/blocklist/mod.rs` defines `BlocklistView` as `{ rules, selected_index }` only — the disabled set is **not** stored on the view. It is lifted to `AgentViewStore.blocklist_disabled_by_session` via `Action::ToggleBlocklistRule` and the dispatch handler in `dispatch_rpc056.rs::handle_toggle_blocklist_rule`. Architecture note [2] correctly describes this lift, so rule [8] internally contradicts architecture [2].

### W2 — Example [5] is internally contradictory
Example [5] first says:
> "session_disabled is reset to empty (the view holds the set; opening a fresh view recreates it)"

Then says the opposite:
> "Rust mirrors TS by storing the disabled set on AgentViewStore.blocklist_disabled (session-scoped lift) so reopening the view paints the same disabled set."

The second sentence reflects the actual implementation (and matches the scenario "Session-disabled set persists across close/reopen of the view"). The first sentence is stale.

### W3 — Architecture note [2] misstates the `BlocklistView` struct shape
Architecture [2] says:
> "BlocklistView state shape: { rules: Vec<BlocklistRuleInfo>, selected_index: usize, session_disabled: HashSet<String> }"

Actual struct (`views/blocklist/mod.rs:57-60`):
```rust
pub struct BlocklistView {
    pub rules: Vec<BlocklistRuleInfo>,
    pub selected_index: usize,
}
```
There is no `session_disabled` field. The set is passed as a `&HashSet<String>` parameter to `render()` and is owned by `AgentViewStore`.

### W4 — Architecture note [3] claims a workspace path that doesn't exist
Architecture [3] says:
> "Uses the workspace path captured by codelet-sessions::SessionManager at construction time."

`SessionManager::new()` (`codelet/sessions/src/session_manager.rs:182`) takes no arguments and stores no workspace path. The implementation in `handle_impl.rs::blocklist_list` actually uses `std::env::current_dir()`. The note is inaccurate but the behaviour is correct (matches the TS frontend's `blocklistLoad(cwd)` default).

### W5 — Feature files were unformatted
`fspec check` reported formatting violations on all three RPC-056 feature files. Fixed via `fspec format`.

## 🟢 Observations (Nice to Have)

### O1 — `views/blocklist/mod.rs` is 418 lines
CLAUDE.md (the TypeScript-flavoured agent guide) recommends files under 300 lines. The Rust codebase doesn't enforce this rule strictly — `provider_settings/mod.rs` is 520 lines and several agent-view files are at 294-297. The 418 lines here are well-organised (struct, key handler, two render helpers, free function `derive_category`, tests) and refactoring would not improve readability. No action.

### O2 — Example [5] mixing rule + example concerns
Example [5] embeds a long architectural justification ("Rust mirrors TS by storing the disabled set on AgentViewStore..."). This crosses example-card and architecture-note boundaries. Could be split into a cleaner example + a separate note. Low priority. No action.

## Coverage Verification
- Feature file `spec/features/rpc056-blocklist-view-cross-transport-parity.feature` — ✅ OK (1/1 scenarios)
- Feature file `spec/features/rpc056-blocklist-view-dispatch.feature` — ✅ OK (14/14 scenarios)
- Feature file `spec/features/rpc056-blocklist-view-source-shape.feature` — ✅ OK (9/9 scenarios)
- Test files (3) — all `@step` comments present and match Gherkin text
- Implementation files (9 distinct sources) — all linked with line ranges that point to real code
- Scenario coverage: 24/24 (100%)

## Test & Build Verification
- `cargo test -p codelet-fspec-tui --test blocklist_view_rpc056 --test source_shape_rpc056 --test rpc056_cross_transport_parity`:
  - blocklist_view_rpc056: 14/14 passed
  - rpc056_cross_transport_parity: 1/1 passed
  - source_shape_rpc056: 9/9 passed
- `cargo build -p codelet-fspec-tui`: ✅ clean

## ACDD Discipline Check
- ✅ Feature files present with `@RPC-056` tag
- ✅ User story present (Yellow card)
- ✅ All scenarios have Given/When/Then ordering — no preconditions-after-Then antipatterns
- ✅ All scenarios have corresponding tests
- ✅ All tests use `@step` comments that match Gherkin step text verbatim
- ✅ No unanswered questions remain
- ✅ Implementation wired end-to-end (slash command → action bus → backend RPC → action bus → view fold)
- ✅ Cross-transport parity invariant proven (both transports increment the same stub counter)

## Files Reviewed
- spec/features/rpc056-blocklist-view-cross-transport-parity.feature
- spec/features/rpc056-blocklist-view-dispatch.feature
- spec/features/rpc056-blocklist-view-source-shape.feature
- codelet/fspec-tui/src/views/blocklist/mod.rs
- codelet/fspec-tui/src/app/dispatch_rpc056.rs
- codelet/fspec-tui/src/store/agent_view/blocklist_state.rs
- codelet/fspec-tui/src/views/navigator.rs
- codelet/fspec-tui/src/transport/mod.rs
- codelet/fspec-tui/src/transport/embedded.rs
- codelet/fspec-tui/src/transport/websocket.rs
- codelet/fspec-tui/tests/blocklist_view_rpc056.rs
- codelet/fspec-tui/tests/source_shape_rpc056.rs
- codelet/fspec-tui/tests/rpc056_cross_transport_parity.rs
- codelet/fspec-tui/tests/common/mod.rs (relevant slice)
- codelet/rpc-types/src/lib.rs (lines 470-501)
- codelet/rpc/src/lib.rs (lines 378-382, 1408-1413)
- codelet/core/src/session_manager_handle.rs (lines 580-805, 1462-1468)
- codelet/sessions/src/handle_impl.rs (lines 880-960)

## Fix Results

### W1 — Rule [8] contradicts implementation → ✅ Fixed
- Soft-deleted rule [8]
- Added new rule [12]: "The session-disabled set is lifted off the BlocklistView and stored on AgentViewStore.blocklist_disabled_by_session..."

### W2 — Example [5] is internally contradictory → ✅ Fixed
- Soft-deleted example [5]
- Added new example [10] describing only the actual persistence-on-reopen behaviour

### W3 — Architecture [2] misstates struct shape → ✅ Fixed
- Soft-deleted architecture note [2]
- Added new architecture note [5] with the accurate struct shape and the AgentViewStore-by-reference flow

### W4 — Architecture [3] claims non-existent workspace capture → ✅ Fixed
- Soft-deleted architecture note [3]
- Added new architecture note [6] reflecting `std::env::current_dir()` resolution + matching TS's `blocklistLoad(cwd)`

### W5 — Feature files were unformatted → ✅ Fixed
- `fspec format` applied to all three RPC-056 feature files
- Re-validation: all three pass `fspec validate`

## Final Verification
- `cargo test -p codelet-fspec-tui --test blocklist_view_rpc056 --test source_shape_rpc056 --test rpc056_cross_transport_parity`: ✅ 24/24 passed
- `cargo build -p codelet-fspec-tui`: ✅ clean
- `fspec validate` on all three RPC-056 feature files: ✅ all valid
- `fspec show-coverage` on all three features: ✅ 100% (24/24 scenarios)
- Example map alignment with implementation: ✅ corrected

## Status Walk
done → implementing → specifying (to remove rule/example) → testing → implementing → validating → done

All transitions used `skipTemporalValidation` since the underlying code did not change — only the example map and feature-file formatting.
