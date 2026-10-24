# BUG-241 ACDD and validation evidence

## Scope and recovered work

The fix follows the pinned Codex revision
`bb64b894fd67158be0f874872e6d7bfe717e7368`. The original upstream comparison
remains historical investigation, not a claim that implementation was complete.
PROV-153's boolean-clearing and full-snapshot-notification policies are explicitly
superseded in its specifications and design attachment.

Recovered sessions:

- Parent: `7a093866-f589-4d59-817e-3a86a189e9fd`.
- Provider implementation: `835edae3-a61c-4fa5-8da9-c3cf1d40f572`.
- TUI implementation: `7c389836-29b4-4884-a152-d2b35b79f4ea`.

The persisted work-unit history records specifying at
`2026-10-22T11:21:19.935Z`, testing at `11:24:29.295Z`, implementing at
`11:41:38.986Z`, and validating at `12:18:36.794Z`. The interrupted session left
production changes, scenario tests, coverage sidecars, and temporary RED/GREEN
logs. Recovery verified those artifacts rather than restarting or claiming a
new initial RED run against an already-fixed baseline.

## Original behavioral RED evidence

The following results were recovered from the prior sessions' actual test logs.
These are assertion/decoding failures, not invented failures or missing-import
compilation failures. The temporary paths identify local original evidence;
the failure summaries below are retained in the repository.

| Target | Original result | Failure being exercised |
| --- | --- | --- |
| `codelet-providers --test bug241_metadata_protocol` | 0 passed, 4 failed | Boolean `true` activates buffering; top-level null falls through to nested data; omitted advice retains the previous explicit model; unrelated event types update optional status. |
| `codelet-agent-loop --test bug241_metadata_presentation` | 0 passed, 4 failed | Routine metadata creates notices; waiting chunks are absent; old cleanup clears newer waiting; verification repeats within a turn. |
| `codelet-fspec-tui --test bug241_provider_waiting` | 0 passed, 2 failed | The new state-only `ProviderWaiting` variant cannot be decoded. |
| `codelet-rpc-types --test bug241_provider_waiting` | 0 passed, 1 failed | The state-only variant is absent from the shared wire model. |

Original logs: `/tmp/bug241-provider-protocol-red.log`,
`/tmp/bug241-session-red-final.log`, `/tmp/bug241-tui-red.log`, and
`/tmp/bug241-rpc-types-red.log`.

Additional test-first regression cycles are recorded in
`/tmp/bug241-reentrant-red.log`, `/tmp/bug241-clear-rebind-red.log`,
`/tmp/bug241-lazy-stream-red.log`, `/tmp/bug241-attempt-treatment-red.log`, and
`/tmp/bug241-wire-discriminant-red.log`. Their failures respectively exercise
reentrant observer reset, a lost explicit waiting clear, dropping a never-polled
stream, response-attempt treatment reset, and a moved legacy interruption
discriminant (`10` instead of `9`).

## Acceptance traceability

Five feature files contain 18 scenarios, each linked to exactly one scenario
test with all exact `@step` comments and implementation locations:

- `codex-buffering-wire-compatibility`: four provider protocol scenarios.
- `codex-metadata-presentation`: four real-session presentation scenarios.
- `transient-provider-waiting`: two TUI rendering/lifecycle scenarios.
- `codex-metadata-safety-status`: seven revised PROV-153 regressions.
- `codex-session-metadata-status`: one revised PROV-153 session regression.

Run from the repository root:

```sh
node spec/attachments/BUG-241/coverage-audit.mjs
```

This audit verifies scenario/test/implementation traceability, not instrumented
line or branch coverage. Extra RPC, NAPI, and transport tests are supporting
regressions, not duplicate scenario mappings.

## Validation review

An independent review extended the existing scenario tests before further
production changes for these cases:

- Rebinding a presentation observer synchronizes waiting, including an explicit
  clear, without replaying an already-announced verification in the same turn.
- Initial model hydration and a same-model refresh preserve active waiting;
  selecting a different known model clears it.
- Buffering advice uses physical HTTP response treatment, or top-level
  WebSocket event headers, independently of nested diagnostic-header precedence.
- Waiting set and clear never enter the supervisor broadcast or relay journal.

The work unit returns to testing for this additional RED/GREEN cycle. Its
recorded state history must retain that return rather than bypassing temporal
validation.

Confirmed review RED: `/tmp/bug241-review-red-confirmed.log` contains three
behavioral failures, one per affected target: verification count `2` instead of
`1`, initial hydration rendering ordinary `Thinking` instead of the active
waiting message, and the eight real-transport advice-source cases returning
merged/upgrade advice instead of the transport-owned treatment. An earlier
compile-only run caught a misplaced test variable; that run is not counted as
behavioral RED. Production fixes followed the confirmed run and the recorded
return to implementing.

The transport-source correction is anchored in pinned upstream
`codex-rs/codex-api/src/sse/responses.rs:64` and `:607` (physical HTTP treatment),
and `codex-rs/codex-api/src/endpoint/responses_websocket.rs:721` and `:991`
(default treatment per response, updated only by top-level event headers).
Nested `response.headers` retains its independent precedence for diagnostics.

## Validation boundaries

- All 2068 repository feature files pass Gherkin validation.
- All five affected features pass scoped tag validation. Global tag validation
  reports 526 unrelated feature files with existing violations; those files are
  not changed by this card.
- Package-wide formatting finds unrelated baseline differences. Formatting is
  also checked directly on every changed/new Rust file, without recursively
  formatting unrelated modules.
- Cargo check and Clippy can report existing unrelated warnings; successful
  completion is not a claim that the repository is warning-free.
- Wire compatibility here preserves existing binary enum discriminants and
  verifies same-version embedded/WebSocket delivery plus both JSON codecs. It
  does not introduce protocol negotiation for older clients that do not know
  newly added enum variants.
- Observer synchronization concerns the provider presentation observer. This
  card does not add a separate RPC snapshot API for a newly connected frontend
  to reconstruct transient chrome from transcript history; waiting remains
  deliberately excluded from transcript and relay replay.
- Tests use local synthetic transports and real sessions, not live provider
  credentials or traffic.

After the disk approached capacity, `cargo clean` was run in `rust/` at the
user's request. Subsequent builds use `CARGO_PROFILE_CI_TEST_DEBUG=0` with the
scoped `ci-test` profile and bounded workspace build concurrency.

## Completion acceptance run

`/tmp/bug241-completion-tests.log` records **35 passed, 0 failed across 12
integration-test targets** after the review fixes, including all 18 acceptance
scenarios. The run includes these targets:

| Package | Integration-test targets |
| --- | --- |
| `codelet-providers` | `bug241_metadata_protocol`, `prov153_codex_metadata`, `prov160_reconnect_stream_consistency` |
| `codelet-agent-loop` | `bug241_metadata_presentation`, `prov153_codex_session_status` |
| `codelet-fspec-tui` | `bug241_provider_waiting`, `bug194_mux_thinking_indicator_per_session`, `bug220_interruption_rendering` |
| `codelet-rpc-types` | `bug241_provider_waiting` |
| `codelet-rpc-server` | `bug241_provider_waiting_wire`, `bug220_interruption_wire` |
| `codelet-napi` | `bug241_provider_waiting` |

Reproduce a target from `rust/` using:

```sh
CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test \
  -p <package> --test <integration-test-target>
```

The combined invocation selects the packages and targets above explicitly and
uses `--no-fail-fast`; no unscoped or workspace-wide test invocation is needed.
Waiting tests assert rendering without transcript mutations, cleanup on output
and turn boundaries, session isolation, terminal sanitization, exact set/clear
delivery over both transports and both JSON codecs, and unchanged interruption
binary discriminants. Session tests additionally assert supervisor/live-relay
exclusion, retry behavior, stale/lazy stream ownership, and observer reentrancy.

The lifecycle-requested `prov149_codex_history` provider target and
`prov149_codex_reload` agent-loop target also pass: **5 passed, 0 failed**, in
`/tmp/bug241-completion-history.log`. Together the completion runs cover **40
passing tests across 14 targets**, with no failures or ignored tests.

## Final quality results

| Check | Result | Local execution log |
| --- | --- | --- |
| Cargo check, eight selected packages including standalone `codelet-fspec` | PASS | `/tmp/bug241-completion-check.log` |
| Clippy, six selected packages and the completion integration-test targets | PASS | `/tmp/bug241-completion-clippy.log` |
| Clippy, standalone CLI, CLI library, and session packages | PASS | `/tmp/bug241-completion-clippy-session-cli.log` |
| Direct rustfmt check on every changed/new Rust file, with unrelated child modules excluded | PASS | `/tmp/bug241-completion-format.log` |
| `git diff --check` | PASS | Final working-tree audit |
| Gherkin syntax, all 2068 features | PASS | `/tmp/bug241-completion-features.log` |
| Tag validation, all five affected features | PASS | Scoped `validate-tags` command results |
| `fspec audit-coverage`, all five affected features | PASS | `/tmp/bug241-completion-coverage.log` |
| Exact scenario/test/step/implementation audit | PASS, 18/18 | `node spec/attachments/BUG-241/coverage-audit.mjs` |

The check package set is `codelet-fspec`, `codelet-providers`, `codelet-sessions`,
`codelet-agent-loop`, `codelet-fspec-tui`, `codelet-rpc-types`,
`codelet-rpc-server`, and `codelet-napi`. Clippy covers the six packages in the
acceptance table with those test targets, plus a separate package run for
`codelet-fspec`, `codelet-cli`, and `codelet-sessions`. All use the same
`ci-test` profile and debug-info override described above.

Existing warnings and unrelated global tag/format violations remain explicitly
outside this card; no lint suppression or validation bypass was added. The
working changes are not committed automatically.
