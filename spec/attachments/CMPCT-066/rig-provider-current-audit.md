# Current Rig provider audit

Owner: d97a39a4. Parent owns board transitions; no staging or commits.

Builtin071 inherited split already preserves eleven scenarios across three
single-target features with real action @step documentation. Current scoped
baseline passes eleven builtin tests, four Anthropic066 tests and six usage065
tests: `/tmp/cmpct-builtin-owner-current-tests.log`.

Anthropic066 audit found a potential explicit-malformed-usage gap: arithmetic
overflow can make `PartialUsage::token_usage` return None while the collector
still publishes a successful FinalResponse. Regression feature is added first;
real SSE behavioral RED must precede any production fix. This owner is editing
`rust/providers/tests/cmpct066_anthropic_terminal_usage.rs` for that regression,
the Anthropic feature/sidecar and Rig provider subtree. Provider runtime owner
042b9b3c should leave that test target untouched during this pass. Busy-message
delivery to parent/provider owner was rejected, so this is the coordination
mailbox. Disk remains 25 GiB; all Cargo commands scoped ci-test -j 2.

Behavioral RED confirmed before production edit:
`/tmp/cmpct066-overflow-behavior-red.log` fails the new actual-SSE regression
because one successful Final response is emitted (expected zero). Initial
occupancy is representable; adding measured terminal output overflows u64.
The collector now explicitly rejects unrepresentable occupancy before returning
merged usage, so the existing streaming error path stops before FinalResponse.

A second actual-boundary gap was found in the Anthropic nonstreaming path:
its returned Usage excludes cache from total_tokens while its raw GetTokenUsage
folds cache into input and drops category detail. This violates usage065's
provider-boundary normalization contract and parity with the terminal066 path.
A sixth feature scenario and real HTTP-adapter test were added before any
nonstreaming implementation edit. RED is queued behind the shared Cargo lock
in `/tmp/cmpct066-nonstreaming-behavior-red.log`; no nonstreaming production edit
will precede that behavioral result. Anthropic test target is 267 lines.

Nonstreaming behavioral RED is now confirmed in
`/tmp/cmpct066-nonstreaming-behavior-red.log`: actual adapter returns total 700,
expected 1150. An earlier mixed suite's missing response `type` was a fixture
error, corrected before this behavioral run; it is not counted as RED evidence.
An explicit raw-usage overflow assertion was also added before the common
normalization fix; its test run is `/tmp/cmpct066-raw-normalization-behavior-red.log`.
No board, runtime-provider, other Rig module, or unrelated production edits.

Recovery resumed. Raw normalization behavioral RED was inspected: four tests
pass and two fail, including overflow panic in the old raw normalizer and
nonstreaming occupancy 700 instead of 1150. The minimal common normalization
fix now retains disjoint categories, computes checked inclusive occupancy and
returns explicit nonstreaming response failure on overflow.

Initial final rerun `/tmp/cmpct-rig-owner-resumed-final-tests.log` is NOT GREEN:
concurrent core065 test edits fail compilation (proptest macro docs parse and
missing Result propagation). No core/runtime-owner files were repaired here;
owned066/071 rerun is isolated in
`/tmp/cmpct-rig-owner-resumed-owned-tests.log`. A prior accidental request for
the nonexistent provider feature `rig` exited 101 before compiling and provides
no behavioral evidence. Parent message remains busy/rejected, not queued.

## Final owned-slice verification — 2026-10-18

The isolated owned rerun is GREEN: **17 passed, zero failed/ignored/filtered**
in `/tmp/cmpct-rig-owner-resumed-owned-tests.log`, explicit exit status 0.
Run from `rust/`:

```sh
cargo test --profile ci-test -j 2 -p codelet-providers \
  --test cmpct066_anthropic_terminal_usage \
  --test cmpct071_builtin_serializers \
  --test cmpct071_builtin_serializers_options \
  --test cmpct071_builtin_serializers_unicode
```

The six Anthropic tests exercise real local HTTP/SSE adapters: terminal cache
retention and finish-hook occupancy, request isolation, measured zero, missing
start, terminal arithmetic overflow with no successful final, and nonstreaming
cache normalization. The overflow scenario also checks raw overflow returns
unknown usage rather than panicking. Eleven builtin tests retain the inherited
physical byte comparisons, generation-aware admission, Unicode property cases,
mandatory documents, API-key/OAuth and Copilot middleware behavior, malformed
options and zero-send negative controls. These are adapter/serializer tests,
not the fresh-process CMPCT-083 lifecycle acceptance gate.

Final quality commands all exited 0, using the shared cache and no new profile:

```sh
cargo check --profile ci-test -j 2 -p rig-core -p codelet-providers --lib
cargo clippy --profile ci-test -j 2 -p rig-core --lib
cargo clippy --profile ci-test -j 2 -p codelet-providers \
  --test cmpct066_anthropic_terminal_usage \
  --test cmpct071_builtin_serializers \
  --test cmpct071_builtin_serializers_options \
  --test cmpct071_builtin_serializers_unicode
```

Evidence logs are `/tmp/cmpct-rig-owner-resumed-check.log`,
`/tmp/cmpct-rig-owner-resumed-rig-clippy.log`, and
`/tmp/cmpct-rig-owner-resumed-test-clippy.log`, each with `EXIT_STATUS=0`.
Two warnings remain outside ownership: provider manager's unused Codex import
and Codex `is_user` dead code. No lint suppression or unrelated repairs.
Scoped rustfmt checks pass for the changed request-usage helper, all four owned
test targets and builtin shared support. Scoped `git diff --check` passes.
The existing oversized Anthropic completion source was changed minimally,
without a broad legacy reformat. New request-usage helper and terminal test
remain under 300 lines (69 and 272 respectively).

## Strict traceability handoff

Four owned specifications map exactly one integration target each:

| Feature basename | Scenarios | Target |
| --- | --- | --- |
| `anthropic-terminal-cache-usage` | 6 | `cmpct066_anthropic_terminal_usage` |
| `builtin-provider-physical-serializer-parity` | 6 | `cmpct071_builtin_serializers` |
| `builtin-provider-physical-serializer-parity-options` | 4 | `cmpct071_builtin_serializers_options` |
| `builtin-provider-physical-serializer-parity-unicode` | 1 | `cmpct071_builtin_serializers_unicode` |

`/tmp/cmpct-rig-owner-resumed-feature-coverage.log` records formatting, syntax,
coverage audit and coverage reporting for all four features, exit 0. Each
scenario has one real test mapping and implementation mappings. Updated line
ranges reflect the current functions; all actual step text occurs as `@step`
documentation around the corresponding setup/action/assertions. Independent
checks verify all ranges and metadata counts (423 Anthropic, 679 main, 161
options, 209 Unicode mapped lines). These counts and 100% reports describe
scenario traceability, not measured execution line or branch coverage.

`/tmp/cmpct_rig_owner_traceability.py` copies the actual four specifications,
sidecars, mapped sources and real tag registry into isolated temporary projects.
The installed `/Users/rquast/.local/bin/fspec` accepts testing-to-implementing
and implementing-to-validating for both card identifiers, exit 0, without any
skip flags. Scoped tag validation also exits 0. Removing the Anthropic action
documentation is a negative control: implementing is rejected, exit 1.
Evidence: `/tmp/cmpct-rig-owner-resumed-traceability.log`.

The isolated boards contain no unrelated dependencies or configured test hook;
these probes prove strict mapping/step gates only, not real project transitions
or hook/dependency completion. Actual behavioral runs are recorded separately
above. No real board, index, commit, branch or unrelated owner files were changed.
The owned 066/071 source/specification slice is stable for coordinator review;
065's other targets, shared provider/core traceability transfers and CMPCT-083
runtime acceptance remain with their owners. Current disk headroom is 24 GiB,
above the coordinator's 20 GiB bounded-validation floor.

## Terminal054 typed-error seam handoff

Loop owner reported authentic physical OpenAI HTTP 400 behavioral RED in
`/tmp/cmpct-loop-owner2-terminal-structured-red.log`: zero passed, one failed,
two filtered, exit 101. The ordinary request returns structured error code
`context_length_exceeded`; no builder invocation occurs. This is not a new
066/071 failure or evidence of completed terminal recovery.

Current OpenAI Chat streaming adapter at
`rust/patches/rig-core/src/providers/openai/completion/streaming.rs:343`
converts the typed HTTP/SSE error into
`CompletionError::ProviderError(error.to_string())`. The original
`http_client::Error::InvalidStatusCodeWithMessage(status, body)` therefore
becomes display text before CLI's strict classifier can inspect it. CLI
`interactive/error_classifiers.rs:57` only parses raw ProviderError JSON.

Existing `CompletionError::HttpError(#[from] http_client::Error)` can carry
the status and original body without introducing a new public error variant.
Recommended coordinated repair: Rig provider owner preserves the original
error in that streaming branch; CLI owner explicitly recognizes structured
HTTP status/body and validates the machine-readable rejection code. Do not
restore broad display-string substring classification or weaken the loop's
physical fixture. The current plan assigns the overall seam to CLI; no new
production change has been made pending that coordination. CLI busy-message
delivery was rejected, not queued. Loop retains its scenario/test ownership.

## Active typed HTTP repair — owner 3ad4c0f4

Loop's latest request confirms this blocks the authentic terminal054 builder.
I am now repairing the Rig Chat adapter with its existing typed
`CompletionError::HttpError(http_client::Error::InvalidStatusCodeWithMessage)`
carrier, preserving BOTH HTTP status and exact original body. This deliberately
does not create raw JSON `ProviderError`, which would discard status provenance.
CLI must classify that typed HTTP error's machine-readable body and preserve
the original error chain at final return; it must not match display substrings.
Messages to CLI8ec86f7e and loop820af668 were busy/rejected, not queued.

New sibling `builtin-provider-structured-stream-rejection.feature` was formatted
and validated before adding target `cmpct071_builtin_stream_rejections`.
It physically posts to local HTTP endpoints returning structured overflow,
authentication with misleading message text, and plain text; exact status/body
and absence of a successful final are required. Behavioral RED is running at
`/tmp/cmpct071-typed-http-behavior-red.log`; no production repair precedes that
result. Existing 066 and eleven serializer assertions are unchanged.

Behavioral RED is confirmed in `/tmp/cmpct071-typed-http-behavior-red2.log`:
zero passed, one failed, zero ignored/filtered; Cargo reports test failure
(101), specifically lost typed HTTP status/body. The earlier log without `2`
is a test diagnostic compilation failure, not behavioral evidence.
After that RED, the adapter now closes its SSE source, yields the original
`CompletionError::HttpError(error)` and returns instead of falling through to
successful FinalResponse. CLI/loop may now use the typed HTTP status/body seam.
The five-target owned regression run is pending in
`/tmp/cmpct-rig-owner3-owned-green.log`; a pending run is not a pass.

## Final resumed provider result — 3ad4c0f4

The final five-target run is GREEN, **18 passed, zero failed/ignored/filtered**,
with explicit `EXIT_STATUS=0` in `/tmp/cmpct-rig-owner3-owned-final.log`:

```sh
cargo test --profile ci-test -j 2 -p codelet-providers \
  --test cmpct066_anthropic_terminal_usage \
  --test cmpct071_builtin_serializers \
  --test cmpct071_builtin_serializers_options \
  --test cmpct071_builtin_serializers_unicode \
  --test cmpct071_builtin_stream_rejections
```

Counts by target are **6 + 6 + 4 + 1 + 1**. The new one-scenario test makes
three real HTTP requests, checking structured HTTP400 overflow, HTTP401
authentication with a misleading overflow message, and an unstructured HTTP400
body. Every result retains exact typed status/body and terminates after one
error, without a successful final or synthetic usage. The inherited seventeen
usage/serializer regressions remain unchanged and pass.

Quality commands run from `rust/`, each with explicit `EXIT_STATUS=0`:

| Command | Log |
| --- | --- |
| `cargo check --profile ci-test -j 2 -p rig-core -p codelet-providers --lib` | `/tmp/cmpct-rig-owner3-check.log` |
| `cargo clippy --profile ci-test -j 2 -p rig-core --lib` | `/tmp/cmpct-rig-owner3-rig-clippy.log` |
| `cargo clippy --profile ci-test -j 2 -p codelet-providers` with the five explicit test targets above | `/tmp/cmpct-rig-owner3-tests-clippy.log` |

The provider dependency emits two existing warnings: unused `codex` import and
unused `CodexItem::is_user`. Rig Clippy emits zero warnings; no warnings or lint
suppression were introduced by this change. The final test file is 65 lines;
`rustfmt --edition 2021 --check` and scoped `git diff --check` exit 0. The legacy
Rig streaming file was already over 300 lines; no major refactor or unrelated
legacy reformat is included.

New feature formatting, Gherkin validation, coverage audit/report, and test
formatting exit 0 in `/tmp/cmpct-rig-owner3-feature.log`. One scenario maps one
actual test target, test lines 11–65, all four literal `@step` descriptions,
and adapter implementation lines 341–346: **61 mapped lines**, not measured
execution coverage. Existing four feature mappings remain unchanged.
The supported `fspec list-tags` confirms the four non-card feature tags are
registered; the work-unit tag is feature-level. Global `fspec validate-tags`
still exits 1 on the same **526 legacy violating files**, with **1404 passed**,
and no violation for this new feature (`/tmp/cmpct-rig-owner3-global-tags.log`).
An attempted temporary tag-registry copy was denied and not retried; this pass
does not claim a new isolated scoped-tag gate execution.

Remaining cross-owner blocker: CLI's current classifier still only handles
raw JSON `ProviderError`, not typed HTTP body/status. It must recognize the
existing `InvalidStatusCodeWithMessage` source and preserve error provenance
through the final stream-loop return. Loop's authentic terminal054 test must
then prove a distinct target builder POST. Provider evidence alone does not
complete terminal054 or the fresh-process083 gate. Busy messages to CLI and
loop were rejected, not queued; this attachment is the durable handoff.

Disk headroom stays above 20 GiB (final observed 25 GiB). No real board changes,
staging, commits, new profile/cache deletion, or cross-owner production edits.

## Active ZAI output-limit repair — 3ad4c0f4

Gate owner reported genuine selected-ZAI normal/plan RED in
`/tmp/cmpct083-zai-selection-current2.log`: both physical requests omit
`max_completion_tokens` despite ZAI setting core `max_tokens` to8192.
The shared OpenAI Chat conversion drops that core field, affecting preparation,
streaming, nonstreaming and direct completion alike. Exact byte parity alone
did not expose the semantic loss. Gate assertions and source remain unchanged.

New sibling `builtin-provider-output-limit-serialization.feature` was formatted
and validated before target `cmpct071_builtin_output_limits` was written.
Three scenarios exercise configured8192/4096/u64::MAX and absent limits,
unrelated optional parameters and physical adapters. Streaming/nonstreaming
retain the shared exact physical-byte, generation, source preservation and
zero-preparation-send checks. Owned behavioral RED is pending in
`/tmp/cmpct071-output-limits-red.log`; no new production edit precedes it.
Free disk headroom is25GiB. Only the Rig provider conversion will be repaired.

Owned behavioral RED is confirmed: all three physical-adapter tests fail with
missing `max_completion_tokens` (null instead of8192), zero compile failures.
The core conversion now retains `max_tokens` as optional
`max_completion_tokens`, omitted for None. No endpoint, ZAI production,
additional-parameter workaround or gate assertion was changed.
`/tmp/cmpct071-output-limits-green.log` is GREEN:3 passed, zero failed,
ignored or filtered, explicit `EXIT_STATUS=0`.

SOURCE READY for gate083 re-execution: the repaired conversion is in source.
The gate matrix `/tmp/cmpct083-gate-latest-matrix.log` was observed starting
before this edit; if it built before the repair, its ZAI result is stale.
Gate-owner notification was busy/rejected, not queued. Final six-target21-test
run is pending `/tmp/cmpct-rig-output-limit-final-tests.log`.

## Current-source final handoff — output reservation repair

The last test-helper edit is included in the final execution, not just the
earlier green run. `/tmp/cmpct-rig-output-limit-current-final.log` finishes
after that edit with **21 passed, zero failed/ignored/filtered**, explicit
`EXIT_STATUS=0`. Counts by integration target are:

| Target | Passed |
| --- | ---: |
| `cmpct066_anthropic_terminal_usage` | 6 |
| `cmpct071_builtin_output_limits` | 3 |
| `cmpct071_builtin_serializers` | 6 |
| `cmpct071_builtin_serializers_options` | 4 |
| `cmpct071_builtin_serializers_unicode` | 1 |
| `cmpct071_builtin_stream_rejections` | 1 |

All six targets ran together with `cargo test --profile ci-test -j 2 -p
codelet-providers` and explicit `--test` arguments, retaining physical HTTP/SSE
assertions and all preparation identity/generation/source/byte checks. The
three new tests exercise twelve actual POSTs across configured8192,4096,
u64::MAX and absent limits. Preparation sends no requests. This verifies the
adapter serialization contract, not fresh-process lifecycle acceptance.

The output-limit helper was restored after the earlier combined green run;
the earlier run alone is therefore not final-source evidence. This current
rerun and the following test Clippy include the restored helper assertions.
All final quality logs explicitly finish with `EXIT_STATUS=0`:

- `/tmp/cmpct-rig-output-limit-check.log`: Rig and providers library check.
- `/tmp/cmpct-rig-output-limit-rig-clippy.log`: Rig library Clippy.
- `/tmp/cmpct-rig-output-limit-tests-clippy.log`: all six explicit test targets.

These use the same bounded `ci-test -j 2` profile. Only the two previously
reported outside-scope provider warnings remain; Rig emits no warnings.
New test and rejection-test rustfmt checks and scoped `git diff --check` pass.
No broad legacy Rig formatting/refactor is included.

The new output-limit feature maps three scenarios to the three actual functions
in one integration target, with implementation field/conversion mappings.
`/tmp/cmpct-rig-output-limit-feature-final.log` records scoped feature format,
Gherkin validation, coverage audit/report and rustfmt, exit0: three scenarios,
one target, 68 mapped lines (not measured line coverage). Strict `link-coverage`
accepts each real mapping and step documentation without skip flags, exit0 in
`/tmp/cmpct-rig-output-limit-step-validation.log`.
That command appended duplicate mappings rather than updating existing entries;
the duplicates were removed, preserving exactly one real function per scenario.
`/tmp/cmpct-rig-output-limit-mapping-final.log` independently verifies three
single mappings, 68 mapped lines and the final coverage audit/report, exit0.

`/tmp/cmpct-rig-output-limit-scoped-tags.log` records a read-only registry and
feature-placement check over all six owned features: all ordinary tags are
registered and work-unit tags are feature-level. This is a scoped registry
check, not a new isolated `fspec validate-tags` execution or real board gate.
No denied temporary-registry-copy action was retried. A tool invocation with
the incorrect `files` JSON key unintentionally selected global format/validate
and reporting; the subsequent recorded CLI commands use exact feature paths.
The global formatter introduced no new unrelated dirty feature paths; the two
add-diagram features were already staged before this pass.

Readiness is exact, not transitive:

- **CMPCT-066 owned slice ready:** six real adapter tests and mappings green.
- **CMPCT-071 builtin slice ready:** fifteen tests across five targets, complete
  per-target specifications and implementation mappings; other071 layers remain
  with their owners.
- **CMPCT-054 typed-error provider seam ready:** original HTTP status/body kept;
  CLI classification and actual outer-terminal builder execution are external.
- **CMPCT-083 not claimed complete here:** selected-ZAI normal/plan and full
  cold-process chronology/no-fallback matrix require gate-owner current-source
  execution. Gate assertions, physical order, endpoint, model and ZAI production
  remain untouched. Gate notification was paused/busy and rejected, not queued;
  this attachment is the source-ready mailbox.

Disk headroom is25GiB. No board, index, stage, commit, branch, validation bypass,
or production files outside the Rig provider subtree were changed by this owner.

## Active Gemini native tool-pairing repair — 3ad4c0f4

Gate083's latest actual capture reports functionCall.name `lookup` followed by
functionResponse.name `call_after_snapshot`. The latter is a generic source ID,
not the native function name required by Gemini. Gate semantics remain intact.

New sibling `builtin-provider-gemini-tool-pairing.feature` was formatted and
validated first; `cmpct071_builtin_gemini_tool_pairing` then exercised two
distinct named parallel calls and reversed response order through streaming
preparation/admission and direct HTTP completion. Behavioral RED is confirmed
in `/tmp/cmpct071-gemini-tool-pairing-red.log`: zero passed, two failed,
Cargo101, actual response name `call_second` instead of `inspect`.

The shared Gemini request conversion now resolves responses from preceding
generic call identities (including call_id aliases) to native function names.
It leaves the generic source history untouched and preserves native argument,
response content and ordering. The new helper is within the owned Rig provider
subtree; no CLI, provider-runtime or gate production source is edited.
Focused GREEN is pending `/tmp/cmpct071-gemini-tool-pairing-green.log`.
No pass or overall gate readiness is claimed from a pending execution.

## Final Gemini source-ready handoff — 2026-10-18 17:50 AEST

The pending focused run above completed **2 passed, zero failed**, exit0.
Final test source additionally exercises call_id aliases, including a result
whose primary ID differs but whose call_id identifies the original call.
`/tmp/cmpct-rig-gemini-final-tests.log` includes that final source and is GREEN:
**23 passed, zero failed/ignored/filtered**, explicit `EXIT_STATUS=0`.

```sh
cargo test --profile ci-test -j 2 -p codelet-providers \
  --test cmpct066_anthropic_terminal_usage \
  --test cmpct071_builtin_gemini_tool_pairing \
  --test cmpct071_builtin_output_limits \
  --test cmpct071_builtin_serializers \
  --test cmpct071_builtin_serializers_options \
  --test cmpct071_builtin_serializers_unicode \
  --test cmpct071_builtin_stream_rejections
```

Counts are **6 + 2 + 3 + 6 + 4 + 1 + 1**. The Gemini target performs four actual
POSTs: streaming and direct completion, each with primary-ID and alias variants.
Two different native functions, reverse result order, full result tail, original
arguments and source-history preservation are asserted. Streaming additionally
retains admission generation/byte identity, zero preparation sends and mandatory
context. No runtime check, gate assertion or fixture semantic is weakened.

Final quality logs all explicitly end `EXIT_STATUS=0`:

- `/tmp/cmpct-rig-gemini-check.log`: `cargo check --profile ci-test -j 2 -p
  rig-core -p codelet-providers --lib`.
- `/tmp/cmpct-rig-gemini-rig-clippy.log`: Rig library Clippy, same profile/jobs.
- `/tmp/cmpct-rig-gemini-tests-clippy.log`: providers Clippy with all seven
  explicit test targets above, same profile/jobs.

Only the inherited providers unused Codex import and `is_user` warnings remain;
Rig Clippy emits no warnings. New helper/test files are58/143 lines. Scoped
rustfmt, feature format/validation and all seven coverage audits/reports pass
in `/tmp/cmpct-rig-gemini-feature-final.log`, exit0. Scoped diff whitespace
checks pass. The oversized legacy Gemini completion source receives only the
shared conversion call, not an unapproved broad refactor or format.

Strict `link-coverage` validates both new actual test mappings and literal step
documentation without bypass flags in `/tmp/cmpct-rig-gemini-traceability.log`,
exit0: two scenarios, one target,139 mapped lines. Existing Gemini helper and
completion ranges plus the shifted OpenAI preparation mapping are refreshed.
An initial integrity probe expected scenario order and exactly one helper
function; these diagnostic script assumptions were corrected without removing
or weakening any real test mapping. Final independent read-only integrity and
registered-tag checks pass for all seven features in
`/tmp/cmpct-rig-gemini-scoped-integrity-tags.log`, exit0. Counts are423,671,161,
211,61,68,139 mapped lines respectively, not measured execution line coverage.
This tag check is not a new isolated/global `fspec validate-tags` execution;
the previously denied registry-copy action is not retried.

Exact readiness:

- **CMPCT-066 owned source/spec slice ready:** six physical usage regressions.
- **CMPCT-071 builtin source/spec slice ready:** seventeen tests across six
  targets, including output reservation and Gemini semantic pairing. Other071
  provider/core/CLI layers remain externally owned.
- **CMPCT-054 provider seam ready only:** typed HTTP status/body preserved;
  actual outer-terminal recovery remains CLI/loop's acceptance responsibility.
- **CMPCT-083 source ready, NOT complete:** gate owner must execute its complete
  current-source cold matrix, real negative controls, both selected ZAI modes,
  immediate promotion probe and live-then-fresh physical chronology. Other
  cross-owner blockers are not repaired or certified by this slice.

The initial Gemini source-ready message reached gate owner5ce54e0f; the final
GREEN update was busy/rejected, not queued. This is the durable final mailbox.
Disk remains25GiB, above20GiB. No board/index/staging/commit/branch/bypass or
production edits outside the Rig provider subtree.
