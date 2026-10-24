# BUG-230 ACDD evidence

## Scope and dependency gate

- Implementation and BUG-230 workflow changes began only after querying
  `spec/work-units.json` and confirming SESS-003 was `done`.
- No SESS-003 production, specification, or workflow changes were made here.
- No commits, branches, or staging operations were performed.
- The multiple-session policy is a technical implementation decision under
  supervisor direction, not a quotation or claimed response from a human.

## Specifications and RED

The parent-drafted `session-work-unit-focus-projection.feature` defines seven
scenarios. All seven have exactly one primary test mapping in
`rust/fspec-tui/tests/bug230_focus_projection.rs`.

1. The two original regressions failed at runtime: board occupancy stayed on
   the previous card and a cleared session header inherited the global card.
   Evidence: `/tmp/bug230-red.log`.
2. Expanding to seven scenarios first failed compilation because the focus
   action was absent: `/tmp/bug230-seven-red.log`.
3. Adding only the action/reducer test seam exposed seven runtime failures,
   before implementing projection: `/tmp/bug230-seven-runtime-red.log`.
   Result: `0 passed; 7 failed`.
4. Real embedded/WebSocket integration first failed on the missing backend
   API, then timed out awaiting focus projection. Evidence:
   `/tmp/bug230-transports-red.log` and
   `/tmp/bug230-transports-runtime-red.log`.
5. A delayed legacy board attachment independently failed the cleared-badge
   assertion: `/tmp/bug230-legacy-badge-red.log`.
6. Read-only review identified creation-order, retained-revision, and delayed
   acknowledgement cases. Supplemental tests exposed the failures before
   their fixes: `/tmp/bug230-review-red.log`,
   `/tmp/bug230-creation-race-red.log`, and
   `/tmp/bug230-lifecycle-red.log`. The creation-order fixture was corrected
   to contain a real card and rechecked against the old acknowledgement path.
7. The supervisor supplied `session-focus-recovery-snapshots.feature` and its
   one-to-one wire test. It failed by timeout before the adapter consumed live
   recovery snapshots: `/tmp/bug230-recovery-snapshots-red.log`.

## Production integration

- `FspecBackend` exposes typed focus get/set operations and a focus receiver;
  embedded and WebSocket implementations delegate to the shared service.
- Bootstrap/reconnect subscribe before fetching authoritative focus. Lagged
  subscriptions refetch live session focus. The WebSocket adapter consumes
  both focus events and server recovery snapshots, including explicit NONE.
- The store retains the full authoritative snapshot and revision independently
  of tab visibility. First revision zero is accepted; duplicates and older
  updates cannot replace accepted focus, including after close/reopen.
- Menu chips, agent/mux headers, and board occupancy use the same per-session
  projection. Headers never fall back to global selection. Occupancy selects
  the earliest open focused session and is rebuilt on focus and removal.
- Remote observations do not bind stale global selection. Lazy, plain-dialog,
  and isolated-dialog creation carry captured local card intent explicitly.
- Board attachment preserves the legacy authorization bind. SESS-003's legacy
  setter already invokes shared focus; the acknowledgement reads authoritative
  focus rather than writing it again and resurrecting an older request.
- BUG-180 live metadata refresh remains intact. Revisionless legacy attachment
  actions cannot override an initialized authoritative snapshot.

## GREEN and quality

The final scoped run passed **900 tests across 27 target result groups**:
673 library tests and 227 integration tests. All 15 BUG-230 tests pass across
the seven-scenario primary target, six recovery/lifecycle regressions, the
real transport test, and the dedicated live-recovery-snapshot test.

Full output: `/tmp/bug230-validation-green-final.log`.

Reproduce from `rust/`:

```sh
cargo test --profile ci-test -p codelet-fspec-tui -j 4 --no-fail-fast --lib \
  --test bug230_focus_projection --test bug230_focus_recovery \
  --test bug230_focus_transports --test bug230_recovery_snapshots \
  --test bug180_wu_status_sync --test work_unit_binding_rpc050 \
  --test session_restore_tui113 --test rpc434_session_events_tui_consumption \
  --test rpc437_session_cleared_tui_consumption \
  --test reconnect_resubscribe_rpc415 --test menu004_mux_surface --test mux002 \
  --test bug163_mux_agent_panes_render_distinct_sessions \
  --test source_shape_rpc009 --test source_shape_rpc024 \
  --test source_shape_rpc025 --test source_shape_rpc050 \
  --test view_agent_unit_rpc029 --test app_bootstrap_rpc009 \
  --test board_agent_navigation_rpc012 --test prov101_session_creation_decline \
  --test merge_worktree_rpc057 --test shift_right_create_session_dialog_rpc097 \
  --test source_shape_stores_rpc012 --test source_shape_rpc057 \
  --test tui111_sanitize_ingress
cargo check --profile ci-test -p codelet-fspec-tui -j 4
cargo clippy --profile ci-test -p codelet-fspec-tui --lib -j 4 \
  --test bug230_focus_projection --test bug230_focus_recovery \
  --test bug230_focus_transports --test bug230_recovery_snapshots
```

- Check passed: `/tmp/bug230-check-final.log`.
- Clippy passed: `/tmp/bug230-clippy-final2.log`. Its sole warning is an
  existing `collapsible_match` in untouched `store/agent_view/token_state.rs`.
- Targeted rustfmt check passes for every changed production file and owned
  test/helper: `/tmp/bug230-fmt-targeted.log`. Broad package fmt reports an
  unrelated existing layout in
  `tests/model_selector_refresh_forces_catalog_refetch_bug207.rs:75`; that file
  was preserved unchanged.
- `git diff --check` passes. All new TUI files and both generated coverage
  files are below 300 lines; existing `store/agent_view.rs` remains 299 lines.
- Both BUG-230 features pass format, validate, validate-tags, and coverage
  audit. Coverage is 100%: seven primary scenarios plus one snapshot scenario,
  with one test mapping each and explicit implementation/integration links.
- Independent read-only review confirmed the identified race and lifecycle
  issues were resolved; no further findings within those refinements.
