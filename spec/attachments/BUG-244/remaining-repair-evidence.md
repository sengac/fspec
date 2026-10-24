# BUG-244 / BUG-245 remaining repair evidence

Exclusive TUI/sessions follow-up. No core, agent-loop, commits, Done calls or
work-unit transitions. Parent RequestStarted decoder match arm is untouched.

## Executable test-first gates

- `/tmp/bug244-245-audit-red.log`: replay state 0/2, inspector 1/2. Historical
  pages erased current waiting; idle polls retained 30 completed tasks; inspection
  read a replacement source generation. All three production causes repaired.
- `/tmp/bug244-245-finish-tui.log`: cold-restored active resume reset input
  counters from 317 to 0. Successful resume now consumes manifest initialization
  only after both model and token restoration return successfully.
- `/tmp/bug245-compaction-marker-red.log`: marker count 0 instead of 1.
  Archival projection now adds explicit committed compaction metadata, not a
  synthetic user message, preserving summary/generation/time/covered boundary.
- `/tmp/bug245-finish-repair.log`: four existing archive scenarios GREEN,
  including marker boundary, physical provider input, missing-original retry,
  and bounded large archive. A new full-coverage/no-suffix boundary regression
  executes RED; iterator now emits the final marker after draining originals.
- Same log: additive `cold-empty-failure-display.feature` maps exclusively to
  `bug245_cold_empty_failures.rs`. GREEN through two fresh child processes:
  input/failure/input/failure/input/success in causal order, no metadata or safe
  failure prose in model replay, no durable manifest changes.

## In-progress final verification

Only existing release-slim artifacts and `-j2` are used. Tests always redirect
to files and are inspected separately (no test/head-or-tail shell guard issue).

- `/tmp/bug244-245-finish-green.log`: rendered 5/5, delivery 9/9, inspector
  2/2, slash resume 5/5, historical waiting and polling retention 2/2 GREEN.
  Two expected follow-ups remained RED in that intermediate executable:
  the newly added live-waiting regression, and a scripted reconnect fixture
  broadcasting fresh text absent from its authoritative pages. The latter
  fixture now appends the live event before broadcasting, matching the real
  backend contract without weakening any assertion. Final reconnect target:
  `/tmp/bug244-reconnect-final-green.log`.
- `/tmp/bug245-final-green.log`: archive model separation, cold empty failures,
  prior cold terminal display and transcript replay, with `--no-fail-fast`.

Historical rendering no longer applies live waiting mutations. A feature-first
subscriber regression additionally checks fresh live content still dismisses
waiting through both transports, without duplicating rendering. Its executable
RED is `/tmp/bug244-live-waiting-red.log`: waiting remained `Some("live waiting")`
instead of `None`. Live content actions now update only transient waiting chrome
and wake authoritative rendering; they never append another content copy.
Final focused waiting validation: `/tmp/bug244-live-waiting-green.log`.

Coverage mappings are reconciled through fspec (unlink every stale test mapping
before relinking; link appends rather than updating). All five BUG244/245
features now have one mapping per scenario, 100% coverage and successful
`audit-coverage`. The additive cold feature has exactly one test file.
Scoped rustfmt --check (including shared initialization files) and global
`git diff --check` pass. Global feature validation reports success; all three
newly edited feature files individually pass validation and tag validation.

Parent messages returned session_busy, not queued; this file provides concrete
progress without waiting on a supervisor that is busy integrating other scopes.

## Completion audit, 2026-10-23

The 18:18 `/tmp/bug244-reconnect-final-green.log` is an incomplete dependency
build, not an executed GREEN result. The old
`/tmp/bug244-245-final-clippy.log` ends with missing tree-sitter build output
directories, not a successful quality check. Neither log justifies completion.
The previous `/tmp/bug245-final-green.log` does contain passing sessions targets;
fresh verification is nevertheless required against the current shared sources.

Fresh scoped commands use the existing shared release-slim directory and `-j2`,
with no direct rustc invocation, replacement profile, rustflags or target path:

```sh
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_rendered_transcript --test bug244_transcript_delivery \
  --test bug244_replay_state --test bug245_history_access \
  --test session_restore_tui113 --test slash_resume_rpc049 --no-fail-fast
cargo test --profile release-slim -j2 -p codelet-sessions \
  --test bug245_archive_model_separation --test bug245_cold_empty_failures \
  --test bug250_cold_terminal_display --test bug250_transcript_replay --no-fail-fast
```

Logs: `/tmp/bug244-245-completion-tui.log` and
`/tmp/bug245-completion-sessions.log`. Both initially wait behind another
worker's shared Cargo build. No production edits or card transitions have been
made during this audit.

Read-only fspec `audit-coverage` passes for all six linked features, including
`slash-command-resume`: respectively 16/16, 14/14, 10/10, 4/4, 2/2 and 8/8
referenced files exist. Test line mappings were also inspected against the
corresponding scenario functions. Global fspec feature validation succeeds.
Scoped rustfmt passes (`/tmp/bug244-245-completion-format.log`); staged and
unstaged `git diff --check` both pass. All new delivery/TUI modules are below
300 lines; the largest is `dispatch_transcript.rs` at 296 lines.

Global tag validation remains nonzero: 1582 files pass and 526 unrelated files
have violations (`/tmp/bug244-245-completion-tags.log`). None names any of the
six linked features. Existing staged and dirty work is preserved; work-unit
storage is not edited manually, and no commit is created.

The read-only completion review found one remaining BUG-244 gap in the existing
failed-resume scenario: `TranscriptAction::Resumed` clears suspension even on
failure, allowing a later poll to fetch an empty page and erase the failure
notice before a successful resume. The feature now explicitly includes polling
and live wakeups after failure, and its existing scenario test checks that both
remain blocked while preserving the notice. The queued TUI Cargo run will supply
executable RED before any corresponding production change. The separately
reviewed first-active-resume acceptance case is already covered by
`bug244/live_resume.rs`, included in `bug244_transcript_delivery`.

Additional current verification logs are
`/tmp/bug244-completion-fragments.log` (six shared assembler regressions) and
`/tmp/bug244-245-completion-quality.log` (scoped library Cargo check, followed by
Clippy with `--no-deps -- -D warnings`). They are queued behind the same shared
Cargo lock, not yet successful evidence. Reproducible read-only specification
and mapping outputs are saved as `/tmp/bug244-245-completion-features.log`
(2108 valid features) and `/tmp/bug244-245-completion-coverage.log`.

Executable completion RED is now recorded in
`/tmp/bug244-245-completion-tui.log`, exit 101. The original 35 other tests pass:
rendered 5/5, waiting/task state 3/3, delivery 9/9, history inspector 2/2,
reconnect 12/12 and four other slash-resume tests. The strengthened failed-resume
test fails at its post-poll `transcript_requests().is_empty()` assertion, proving
the production gate defect rather than a build failure.

Only after that RED, the production resume response handler was changed to
release initial hydration suspension on success only. Failed resume preserves
the gate and its scoped notice across polls and live content notifications.
No shared sessions file was modified. The changed production module is 298
lines and the test target 248 lines; both pass scoped rustfmt. Obsolete test
headers describing the removed buffered-output hydration were removed.
Scenario line mappings were refreshed exclusively with fspec unlink/link
commands; the repaired failure scenario intentionally has no implementation
mapping until its new GREEN execution.

Full TUI rerun with the identical six targets and flags is queued in
`/tmp/bug244-245-completion-tui-green.log`. The original RED log is preserved.

Fresh sessions execution completed in `/tmp/bug245-completion-sessions.log`:
BUG-245 archive/model separation 5/5 and cold-empty failures 1/1 pass, including
the physical provider request and two fresh empty-failure child processes.
BUG-250 display-only transcript replay also passes 1/1. The combined command
exits 101 because `bug250_cold_terminal_display` fails at line 76: restored model
message count is 4, expected 3. Its earlier prefix/failure order and original
terminal identity/timestamp assertions pass. This is not a failure of the six
BUG-245 acceptance tests, but it is an unresolved neighboring regression and
must not be reported as an entirely GREEN sessions command.

BUG-249/250 owner 07887505-fee3-4a51-8f54-a54b435a33e0 and the supervisor were
notified through AgentManager; both returned session_busy, so neither message
was queued. No BUG-250 tests or shared decoder/core files were changed here.
The owning worker already has the same regression target queued in its own
Cargo sequence and should investigate the actual model payload before changing
the count assertion. Cards remain implementing pending final verification.

With all eight BUG-245 scenarios now freshly verified (five archival/model,
one fresh-process empty-failure, two TUI inspector), BUG-245 was advanced through
the fspec command to validating. Scoped Cargo library check/Clippy are running;
the neighboring BUG-250 failure remains explicitly reported rather than hidden.
BUG-244 remains implementing until the repaired failed-resume scenario is GREEN.

The BUG-250 owner subsequently corrected the neighboring test's unstable total
message count: it now asserts the exact conversational assistant prefix after
excluding system reminders, while retaining all original terminal assertions.
The owner's `/tmp/bug249-250-completion-cargo.log` now records both neighboring
sessions regressions passing. No BUG-250 edit was made by this worker. The same
four-target sessions command is queued again as
`/tmp/bug245-completion-sessions-green.log`, preserving the earlier failure log
and requiring a clean combined result rather than relabelling that earlier run.

## Final validation gates

`/tmp/bug244-245-completion-tui-green.log` exits 0: rendered transcript 5/5,
live/historical state 3/3, delivery 9/9, inspector 2/2, reconnect 12/12 and
slash resume 5/5. This includes the new failed-resume polling/live-content gate.
`/tmp/bug245-completion-sessions-green.log` exits 0: archive/model separation
5/5, cold-empty failure 1/1, cold terminal 1/1 and display-only replay 1/1.
`/tmp/bug244-completion-fragments.log` exits 0 with 6/6 assembler tests.
These are 50 passing top-level tests, plus the fresh-process child assertions.

The first BUG-244 validating attempt correctly rejected several test files in
one feature. Existing acceptance cases were reorganized without behavior changes:
the first-live-resume case now lives in `active-session-first-resume.feature`,
the three waiting/polling cases in `transcript-hydration-live-state.feature`,
and the previously-cold-resumed case joins the rendered hydration feature.
All 18 BUG-244 scenarios are preserved; every feature now maps to one test file.
Coverage generation was isolated to a four-feature temporary mirror to avoid
touching unrelated sidecars, then generated sidecars were synchronized and all
test/implementation links populated through fspec. No work-unit data was edited
manually. All eight BUG-244/245 features are 100% mapped and pass audit-coverage.
Global validation now passes 2112 features; global tag validation still reports
the same 526 unrelated violations and no owned-feature violations.

Only feature-reference headers changed in the two affected Rust test modules.
Their post-reorganization Cargo verification is
`/tmp/bug244-completion-split-green.log`, using:

```sh
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_replay_state --test bug244_transcript_delivery --no-fail-fast
```

Both cards advanced to validating through fspec after behavior verification and
the traceability repair. Neither is marked done: strict quality is blocked.
The Cargo check portion of `/tmp/bug244-245-completion-quality.log` succeeds,
but Clippy exits 101 at the unchanged
`rust/fspec-tui/src/store/agent_view/token_state.rs:100` with
`clippy::collapsible_match`. `git diff --quiet HEAD --` for that path exits 0,
confirming the offending code is baseline, not this task's patch. No lint was
disabled and no unrelated production repair was made without coordination.
The supervisor notification returned session_busy and was not queued.

Exact quality commands, from `rust/`:

```sh
cargo check --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types --lib
cargo clippy --profile release-slim -j2 \
  -p codelet-fspec-tui -p codelet-sessions -p codelet-rpc-types \
  --lib --no-deps -- -D warnings
```

The assembler command was `cargo test --profile release-slim -j2
-p codelet-rpc-types --test transcript_fragments` from `rust/`.
At handoff the 12-test, post-feature-split recheck is still waiting for the
shared Cargo artifact lock in `/tmp/bug244-completion-split-green.log`.
This pending recheck and the strict baseline Clippy failure are not called
completed gates. Both cards deliberately remain validating for supervisor
integration; no done transition is requested.

## Continued quality completion

The remaining Clippy gate is a canonical, semantics-preserving match-guard
refactor within the assigned TUI ownership, not a new behavior or a lint waiver.
After the recorded strict Clippy failure, `token_state.rs` now uses the suggested
guard on `ContextFillUpdate` instead of nesting the identical condition inside
that match arm. Both branches still ignore legacy fill updates after a committed
generation and apply them before one. The module remains below 300 lines.
No other baseline production code or shared delivery file was changed.

Current check and strict Clippy rerun:
`/tmp/bug244-245-completion-quality-green.log`. The final TUI command retains all
six acceptance/regression targets and adds existing token-state and committed-
projection behavior checks:

```sh
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_rendered_transcript --test bug244_transcript_delivery \
  --test bug244_replay_state --test bug245_history_access \
  --test session_restore_tui113 --test slash_resume_rpc049 \
  --test token_state_realtime_recompute_rpc101 \
  --test cmpct068_committed_projection --no-fail-fast
```

Its log is `/tmp/bug244-245-completion-final-tui.log`. Both cards remain validating
until these final executable gates finish successfully. The previous blocker
and failed quality log remain preserved rather than overwritten.

The quality rerun now exits 0: both scoped Cargo check and strict Clippy pass
without lint suppression (`/tmp/bug244-245-completion-quality-green.log`). The
eight-target final TUI attempt exits 101 before executing tests because Cargo
cannot open an object while assembling
`libdatafusion_functions-007c9c3c8ee10c0d.rlib`. This is an artifact build failure,
not behavioral RED. The earlier two-target post-split Cargo job is still building
shared dependencies. It will be allowed to finish before retrying the final
suite, avoiding additional overlapping test builds against the same artifacts.
No target cleanup, direct rustc invocation, alternate profile, target directory,
lint disable, commit or manual work-unit edit was used.

Further validation exposed an existing contract conflict in the optional old
RPC-101 target (2 pass, 7 fail): it expects the local cumulative-counter fill
recompute that CMPCT-067 explicitly forbids. No fallback was restored. The latest
authoritative results and final card states are recorded in
`completion-validation.md`; this file retains the chronological audit evidence.
