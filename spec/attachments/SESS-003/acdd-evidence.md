# SESS-003 implementation and ACDD evidence

## Decisions

These are implementation decisions authorized by the continuation instruction,
not quotations attributed to a human.

- Activity focus is independent of authorization and isolation/worktree binding.
- Persist focus and authorization in separate manifest fields. Activity clear
  never removes the stage authorization binding, including after restore.
- Legacy manifests restore explicit NONE, without interpreting the session name.
- Restore reconciles done/deleted cards to NONE and refreshes current metadata.
- Revisioned events and initial SessionInfo snapshots use one shared service for
  embedded and WS transports. Duplicate selection is idempotent even if its
  attribution source differs. Newer revisions supersede older replies/snapshots.
- A busy inner lock schedules a deferred latest-state reminder. Turn entry also
  reconciles before provider construction. No Rig-to-sessions dependency is added.

## Specification first

`session-work-unit-focus.feature` was written, formatted and validated before
the backend integration tests. The transport scenario was subsequently split
into `session-work-unit-focus-transports.feature` to honor the workflow's
one-feature/one-test-file rule, then formatted and validated before its final
coverage mapping. Both features are tagged SESS-003.

Coverage generation used an isolated temporary project containing only these
features, so the global generator did not mutate unrelated cards' sidecars.
Mappings and all work-unit transitions use normal fspec commands.

## Expected RED

- `/tmp/sess003-red-final.log`: backend integration target failed with 23
  missing-method/field errors for the new focus API and manifest state. Incidental
  fixture API typos were corrected before this clean RED capture.
- `/tmp/sess003-transport-red.log`: real-manager embedded/WS integration target
  failed with 10 missing API/type/field errors before transport production edits.
- `/tmp/sess003-auth-red.log`: two backend scenarios passed; restore scenario
  failed with actual authorization `None` versus expected `Some("testing")`,
  before adding independent authorization persistence and restoration.

## Public API for dependent cards

`codelet_rpc_types::WorkUnitFocus` contains `revision: u64`,
`context: Option<WorkUnitContext>` and `source: String`. Default is revision 0,
context None, source `initial`. `apply_if_newer(incoming)` accepts only a strictly
newer revision. Context has authoritative `id`, `title`, and `status` strings.

`SessionManager::set_session_focus(&SessionId, Option<&str>, &str)` returns
`Result<WorkUnitFocus, activity_focus::FocusError>`. ID resolution uses the
invoking session's project. `None` is an explicit clear, not a guessed card.
Errors leave focus and revision unchanged. This activity-only API does not alter
authorization, lifecycle, isolation or another session.

`SessionManagerHandle` and tarpc RPC expose:

- `get_work_unit_focus(session_id) -> Result<WorkUnitFocus, String>`
- `set_work_unit_focus(session_id, Option<String>, source: String)
  -> Result<WorkUnitFocus, String>`
- Handle/shared service/embedded transport `work_unit_focus_rx()` broadcasts
  `(SessionId, WorkUnitFocus)`.

Consumers must use the new focus API for activity display. The legacy
`get_work_unit_context` remains the independent authorization binding and may
remain populated after activity focus is cleared.

WS uses appended `SessionEvent::FocusChanged { session_id, focus }`; connect
snapshots include `SessionInfo.work_unit_focus`. A lagging WS focus stream emits
a fresh full session snapshot. Existing board context attachment delegates to
focus mutation while preserving its separate permission binding.

The manifest fields are `work_unit_focus` and `work_unit_authorization`, both
serde-defaulted for legacy files. Updates reuse `update_session_manifest`, not
stale whole-manifest replacement. SessionSearch recent summaries use persisted
focus instead of parsing session names.

## Validation

All Cargo commands ran from `rust/` with explicit packages/targets and `-j 6`.
Test runs used `--profile ci-test` and debug-info environment overrides. No
workspace-wide test command was run.

- `cargo test --profile ci-test -p codelet-sessions --test sess003_work_unit_focus`
  passed all 3 scenarios; final log `/tmp/sess003-focus-final.log`.
- `cargo test --profile ci-test -p codelet-rpc-server --test sess003_focus_transport`
  passed the real-manager embedded/two-WS-clients/reconnect scenario; final log
  `/tmp/sess003-transport-final.log`.
- Session regressions `rpc422_session_persistence` (12),
  `wt004_session_tool_callbacks` (13), and the three new backend scenarios passed
  with `-- --test-threads=1`; `/tmp/sess003-backend-green-final.log`.
- Transport regressions `rpc433_session_event_fan_out` (4),
  `rpc436_session_cleared_fan_out` (1), and the new transport scenario passed;
  `/tmp/sess003-transport-regression.log`.
- Backend `cargo check -p codelet-sessions -p codelet-agent-loop` and
  `cargo clippy -p codelet-sessions -p codelet-agent-loop --lib` passed;
  `/tmp/sess003-backend-check.log`, `/tmp/sess003-backend-clippy-final.log`.
- Strict Clippy for the new backend integration target passed;
  `/tmp/sess003-backend-test-clippy.log`.
- Strict Clippy for the new transport integration target passed;
  `/tmp/sess003-transport-test-clippy-final.log`.
- Transport library check/Clippy and `cargo check -p codelet-rpc-types --features
  napi` passed; `/tmp/sess003-transport-check.log`,
  `/tmp/sess003-transport-clippy.log`, `/tmp/sess003-transport-napi-check.log`.
- Scoped package formatting passed for core/sessions/agent-loop/rpc-types/rpc/
  rpc-server, plus a direct rustfmt check for the changed embedded transport
  source; `/tmp/sess003-fmt-final.log`. `git diff --check` passed.
- Both feature files passed format, validate and validate-tags. Both coverage
  audits passed, with 100% scenario coverage (3/3 backend, 1/1 transport), one
  scenario-to-test mapping each, and explicit implementation integration links.

## Unrelated validation observations

An initial parallel WT-004 regression run had one failure in its existing
singleton fallback test: the facade's process-global callback had not been
registered before that test ran. The ordered single-thread run passed all 13.
No unrelated callback-registration change was made.

A broad package fmt check including all embedded tests reported existing
formatting in `rpc-embedded/tests/embedded_session_repl.rs:64`. That unrelated
file is untouched; changed embedded production source passes targeted rustfmt.

The NAPI focus metadata and SessionInfo are output-only because NAPI does not
deserialize `u64` fields. No future-card production behavior is implemented;
TUI test/helper edits only initialize the newly required SessionInfo field.
