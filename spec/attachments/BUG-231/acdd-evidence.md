# BUG-231 ACDD and API evidence

## Ownership and decisions

SESS-003 and BUG-230 were both verified `done` in `spec/work-units.json`
before BUG-231 workflow or production changes. Only BUG-231 workflow is owned
here. No commits, branches or staging operations are performed.

The three Example Mapping answers record supervisor-authorized implementation
decisions, not purported quotations from a human: explicit attribution wins;
completion/deletion clear all matching activity without releasing authorization;
explicit follow-up on an already-done card is allowed.

## Specification and test-first evidence

`session-status-focus-reconciliation.feature` was formatted, validated and
tag-validated before tests. Seven drafted scenarios plus busy raw-thread and
watcher lifecycle scenarios map one-to-one to nine tests in
`rust/sessions/tests/bug231_status_focus.rs`. Tests use shared temporary-directory
helpers, real stores/managers, provider facades, fspec-core dispatch, filesystem
watchers and bounded waits. Helper modules remain below 300 lines.

- `/tmp/bug231-red-clean.log`: clean compilation RED with 13 errors, exclusively
  missing invocation context, production handler factory and reconciliation APIs.
  Incidental fixture path/map-key mistakes were corrected before this capture.
- `/tmp/bug231-runtime-red.log`: **5 passed, 4 failed** after extracting the
  handler and implementing the independently missing policy API. Real facade
  execution still retained the old card/stage, replacement failed by timeout,
  and raw-thread reconciliation panicked because no Tokio runtime was entered.
  These failures preceded the boundary hook, replacement binding and captured
  runtime fixes respectively.
- The real agent-turn helper additionally drives a local OpenAI-compatible SSE
  provider through production agent-loop registration. Its first exploratory
  runs used the wrong provider tool name (`fspec_command` instead of `fspec`);
  those timeouts are **not claimed as legitimate registration RED**.
- `/tmp/bug231-cancellation-red.log`: an idle subscription retained its callback
  after cancellation while its sender remained alive; bounded resource-release
  assertion failed before introducing a cancellation wakeup.
- `/tmp/bug231-review-red.log`: independent review produced two further real
  regressions before their fixes: a bridge metadata callback could not acquire
  the authorization read lock during refresh, and metadata-only revision
  advancement suppressed a valid legacy switch. A third failure in this run
  was a test expecting a canceled receiver to remain subscribed; that fixture
  expectation was corrected, not treated as a product regression.

## Attribution preservation handoff

`codelet_tools::execution_context` exposes:

- `WorkUnitAttribution::{Absent, Card(String), None}`.
- `InvocationReceipt { session_id: Uuid, project: PathBuf, attribution,
  admission: u64, focus_revision: u64 }`.
- `scope_invocation(receipt, future)` and `current_invocation()`.

`SessionManager::admit_tool_invocation(&SessionId, WorkUnitAttribution)` validates
against that session's project and returns `Result<InvocationReceipt, FocusError>`.
Explicit admission sets/clears activity before execution, without modifying the
authorization/worktree binding. Repeated identical attribution gets a fresh
admission sequence even when the visible focus revision remains idempotent.
Unknown-card admission fails without advancing activity or admission.

TOOL-026 must validate/strip reserved model metadata, then keep this receipt
outside provider/business arguments by wrapping the **actual delegated tool
future** in `scope_invocation`. Task-local scope is not automatically inherited
by newly spawned Tokio tasks or blocking threads: carry the receipt explicitly
and re-scope inside any such execution boundary. Never infer explicit-vs-absent
from source strings, current focus, or the administrative command target.

`codelet_agent_loop::fspec_handler::create_handler` is production-used by the
shared standalone/NAPI agent loop. It captures the scoped receipt (or admits a
legacy Absent invocation), executes the shared dispatcher/callback, and only
after success reconciles authoritative card state before publishing the result.
Legacy inference requires equal admission and a non-regressed visible revision;
metadata-only revision advances do not count as newer activity. Explicit card or
NONE never permits inferred switching. Sessionless NAPI/CLI commands do not
guess the global active session.

## Watcher and authorization policy

`SessionManager::reconcile_work_units(&Path)` always rereads authoritative disk
state under each matching session's admission guard. Watcher snapshots signal
resynchronization; queued status values are never applied directly. Already-done
explicit focus survives duplicate/delayed notifications. Metadata refresh does
not admit new activity, and authorization refresh is independent of focus NONE.
Deleted bindings retain the last known authorization instead of becoming
unrestricted. No focus access occurs inside manifest-update closures.

`WorkUnitSubscription` subscribes before the initial authoritative observation,
resynchronizes immediately on lag, and serializes cancellation with in-flight
callbacks. SharedFspecService owns/replaces the adapter automatically; NAPI uses
the same adapter with singleton identity plus generation-guarded cache delivery.
Busy-session reminders use the runtime handle captured during session creation,
so raw filesystem watcher threads never call `Handle::current()`.

## Validation

Commands run from `rust/` with package scope, `--profile ci-test` and `-j4`.
No workspace-wide test run is performed.

- `/tmp/bug231-regressions-final.log`: **29 passed** across the nine-scenario primary
  target, `sess003_work_unit_focus` (3), `wt004_session_tool_callbacks` (13),
  and `paused_chunk_delivery_rpc409` (4), using `-- --test-threads=1`.
  This includes the real provider-driven agent turn, callback front door,
  dropped-subscription resource release, bounded metadata callback reentry,
  real watcher replacement and deliberately lagged broadcast receiver.
  Final assertions also retain a nonempty worktree binding and the restrictive
  `specifying` authorization stage after deleting its bound card.
- `/tmp/bug231-check-final.log`: package-scoped check passes for sessions,
  agent-loop, RPC, standalone fspec and NAPI. Seven existing NAPI warnings
  concern unused compatibility helpers and ignored `clear_history()` result;
  their code is unchanged.
- `/tmp/bug231-fmt-final.log`: targeted formatting passes for all owned production
  edits and tests. `reorder_imports=false` preserves an unrelated pre-existing
  tools-root re-export ordering. `git diff --check` passes.
- The feature passes format, validate, validate-tags and coverage audit.
  Scenario coverage is **100% (9/9)** with exactly one test mapping per scenario
  and explicit implementation/integration mappings. This is scenario coverage,
  not a claim of 100% instrumented Rust line coverage.
  Explicit dispatcher validation with `{"file":"spec/features/session-status-focus-reconciliation.feature"}`
  reports `validCount: 1, invalidCount: 0`. A separate project-wide tag audit
  reports 526 existing violations in other feature files and 1,526 valid files
  (`/tmp/bug231-tags.log`); no unrelated specifications were changed.
- Read-only independent review found the two races and idle-worker leak above;
  follow-up inspection confirmed all three fixes with no additional findings.

- `/tmp/bug231-clippy.log`: library Clippy passes for core, tools, sessions,
  agent-loop, RPC and NAPI. Only the same seven unrelated NAPI warnings remain.
- `/tmp/bug231-test-clippy.log`: the new integration target and all helper
  modules pass Clippy without warnings.

## Unrelated existing regression

`/tmp/bug231-agent-regression.log` runs `codelet-agent-loop --test
rpc084_streaming`: six source-contract tests pass and
`run_with_provider_macro_streams_via_run_agent_stream_with_images` fails.
Its string-based macro extractor selects the small provider-adapter macro body
and expects a streaming call there. Both that test and `agent-loop/src/dispatch.rs`
are unchanged from HEAD (`git diff --exit-code` verified). No unrelated test or
dispatch change was made. This failure is not represented as a passing suite.

## Supplemental GREEN

- `/tmp/bug231-transport.log`: the real-manager embedded/two-WebSocket-client
  focus event and reconnect snapshot scenario passes (1 test).
- `/tmp/bug231-core.log`: canonical work-unit snapshot tests pass (2 tests).
- `/tmp/bug231-tools.log`: existing per-session Fspec handler registration,
  dispatch and concurrent isolation tests pass (4 tests).
- Total targeted GREEN is **36 tests**. The separately reported source-contract
  target contributes another six passing tests and one existing failure.

Reproduce the primary/regression run:

```sh
cargo test --profile ci-test -p codelet-sessions -j4 \
  --test bug231_status_focus --test sess003_work_unit_focus \
  --test wt004_session_tool_callbacks --test paused_chunk_delivery_rpc409 \
  -- --test-threads=1
cargo test --profile ci-test -p codelet-rpc-server -j4 --test sess003_focus_transport
cargo test --profile ci-test -p codelet-core -j4 --lib work_units::tests
cargo test --profile ci-test -p codelet-tools -j4 --lib fspec_handler::tests
```

All new files are below 300 lines. Final free disk space is about 7.8 GiB;
subsequent workers should continue using bounded package/target-scoped builds.

BUG-231 is `done` through the normal fspec workflow. The feature carries `@done`;
SESS-003 and BUG-230 remain `done`. The final coverage audit passes all 22 mapped
paths with nine single test mappings and implementation integration links.
