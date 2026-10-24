# BUG-226 validation

## ACDD evidence

- Read the research attachment, repository guidance, testing guide, and existing
  selector, dispatch, and durable model-switch specifications.
- Recorded Example Mapping and user story while specifying; wrote two feature
  files before regression tests, with exact scenario step comments.
- TUI behavioral red: `/tmp/bug226-tui-red.log`, six failed and one passed.
  Failures expose optimistic selection, missing feedback, alias mismatch,
  missing runtime feedback, and unordered writes.
- First TUI green: `/tmp/bug226-tui-green.log`, seven passed.
- Final TUI behavioral/regression run: `/tmp/bug226-tui-final-tests.log`,
  eight new tests, eight RPC-022 tests, and seven RPC-018 tests passed.
- Supplemental coverage adds later rejection after an earlier accepted pending
  switch and a real selector render assertion.
- Card moved to implementing after the first TUI source patch, rather than
  immediately before that patch. Behavioral red preceded all production edits.

## Legacy red and green

`/tmp/bug226-napi-red.log` initially failed compiling unrelated BUG-220 NAPI
types. No workaround or empty noop-mode run was counted as behavioral red.
After the owning worker restored compatibility, the real legacy tests ran.
`/tmp/bug226-napi-behavioral-red.log` records two rejected-switch failures and
one successful-switch pass before legacy production edits. Both failures show
labels and threshold override changed despite unchanged live selection.

Both setters now stage the threshold override, select first, and publish the
override, resolved cached limits, and labels only on success under the existing
inner lock. Ordinary registry validation, Codex bypass, profile direct selection,
scripted-limit precedence, facade, and capability work remain intact.
`/tmp/bug226-napi-green-repaired.log`: all three legacy tests pass.

## Final gates

- `/tmp/bug226-final-check.log`: both scoped test-target checks pass.
- `/tmp/bug226-final-tui-regressions.log`: eight BUG-226, eight RPC-022,
  and seven RPC-018 tests pass against the final shared source state.
- `/tmp/bug226-final-clippy.log`: both scoped test-target clippy checks pass
  with workspace defaults; no BUG-226 diagnostics or lint suppressions.
- Strict TUI clippy remains blocked by unrelated
  `store/agent_view/token_state.rs:100` (`collapsible_match`), captured in
  `/tmp/bug226-tui-final-clippy.log`. NAPI also has six existing dead-code
  warnings and an unrelated unused Result at `session_bindings.rs:1555`.
  These are not fixed or suppressed by BUG-226. Default clippy success is not
  represented as strict warnings-as-errors success.
- `/tmp/bug226-final-spec-quality.log`: both features and registered tags
  validate; both direct coverage audits pass. Eight TUI and three legacy
  scenarios each map to exactly one test, with implementation links.
- `/tmp/bug226-final-fmt.log`: scoped rustfmt and tracked whitespace checks pass.

The TUI coverage statistics retained seven scenarios after the eighth was
added; corrected only this owned coverage file's cached total and percentage.
Final `fspec review BUG-226` is recorded in `/tmp/bug226-final-review.log`.

## Commands

From `rust/`, all Cargo commands use
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` and `-j 2`:

- `cargo test -p codelet-fspec-tui --test bug226_accepted_model -j 2`
- `cargo test -p codelet-napi --test bug226_legacy_model_atomicity -j 2`
- `cargo test -p codelet-fspec-tui --test bug226_accepted_model --test app_dispatch_rpc022 --test app_bootstrap_rpc018 -j 2`
- `cargo check -p codelet-fspec-tui --test bug226_accepted_model -j 2`
- `cargo clippy -p codelet-fspec-tui --test bug226_accepted_model -j 2 -- -D warnings`
- `cargo check -p codelet-napi --test bug226_legacy_model_atomicity -p codelet-fspec-tui --test bug226_accepted_model -j 2`
- `cargo clippy -p codelet-napi --test bug226_legacy_model_atomicity -p codelet-fspec-tui --test bug226_accepted_model -j 2`

Feature checks use `fspec validate <path>`, `fspec validate-tags <path>`, and
`fspec audit-coverage <feature-name>` for the two owned features. All mutating
fspec commands use the `/tmp/fspec-acdd-board.lock` flock wrapper. Formatting
uses `rustfmt --edition 2021 --check --config skip_children=true` against the
eleven owned Rust files. No unscoped Cargo tests or live endpoint calls run.

## Scope and limits

Exact BUG-226 file ownership (shared files contain other workers' hunks):

1. `rust/fspec-tui/src/app/dispatch_model_sync.rs` (new)
2. `rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs`
3. `rust/fspec-tui/src/app/dispatch_session_chrome.rs` (model refresh only)
4. `rust/fspec-tui/src/app/mod.rs` (`dispatch_model_sync` registration only)
5. `rust/fspec-tui/src/app/state/state_types.rs` (`model_sync` field/init only)
6. `rust/fspec-tui/src/components/mod.rs` (`ModelSelectionLoaded` only)
7. `rust/fspec-tui/src/views/model_selector/state.rs`
8. `rust/fspec-tui/tests/common/model_backend.rs` (new)
9. `rust/fspec-tui/tests/bug226_accepted_model.rs` (new)
10. `rust/napi/tests/bug226_legacy_model_atomicity.rs` (new)
11. `spec/features/accepted-model-selection.feature` (new)
12. `spec/features/accepted-model-selection.feature.coverage` (new)
13. `spec/features/legacy-model-selection-atomicity.feature` (new)
14. `spec/features/legacy-model-selection-atomicity.feature.coverage` (new)
15. `spec/attachments/BUG-226/validation.md` (new)
16. `spec/work-units.json` (BUG-226 metadata/workflow/Example Mapping only)
17. `rust/napi/src/session_bindings.rs` (both model setters only)

TUI changes serialize model selection and chrome hydration per originating
session, reject stale snapshots, and read accepted identity through existing
backend session metadata rather than guessing from display names or requested
aliases. Rejections and metadata failures produce session-bound visible notices.
No RPC schema or inference request is added.

Existing session metadata and ModelInfo are separate RPC reads, not a single
atomic cross-client server snapshot. Per-session serialization protects this
TUI's own request ordering. Concurrent external clients remain a limitation.

Parent owns staging and commits. Nothing is staged or committed by this worker.
Card is validating, not done: behavioral tests and default scoped gates pass,
but the previously attempted strict warnings-as-errors gate remains blocked
by unrelated diagnostics. Parent owns resolution of that remaining gate.
