# PROV-162 validation and direct review

## Approved scope

This explicitly reconciles PROV-148's deliberate text-only policy with PROV-149
typed lineage. It does not relabel the original restriction as an accidental bug.
The native answer to `prov162_bounded_typed_continuation`, request
`f3874604-9758-4382-80db-93bdfe49e7d3`, approves the attached bounded design.
The supervisor's delegated approval and the refusal/outer-loop review remain
canonical in native history and their controller receipts. No new approval,
worker, model selection, live provider test or unrelated compaction repair occurs.

## Causal evidence and honest chronology

The original feature files, native rules/examples and tests precede production.
Preserve `typed-protocol-initial-red.log`,
`typed-and-native-loop-causal-red.log` and
`native-composition-valid-fixture-red.log`: these contain genuine text-only
replay and actual native CLI composition failures. Earlier attribution/catalog
fixture failures are not product RED. The original outer-refusal compiler failure
and output-reservation admission failure are likewise not runtime refusal proof.

The replacement repairs only the normal fixture's explicit positive output
reservation through `override_model_limits`. It removes a duplicate override.
`outer-refusal-admitted-runtime-red.log` then compiles and demonstrates TWO
physical responses instead of one: suppressing protocol replay alone allowed an
outer automatic nudge. The final Codex response now carries the existing Final
stop-reason field as `refusal`, which both ordinary and goal continuation already
recognize as non-clean. No outer budget/decision policy is rewritten.

`malformed-shape-runtime-red.log` demonstrates malformed annotations reaching a
published checkpoint. The narrow validator now validates nested arrays/strings,
known citation/logprob fields, typed summaries, opaque strings, optional IDs and
statuses, hosted actions and exact family/field allowlists. Unknown or malformed
items fail without diagnostics exposing encrypted content. The first shape
matrix still failed an explicit null encrypted field; the final validator checks
that field on reasoning, not on the message branch.

The first control matrix catches a pre-send generation guard missing from the
inherited partial implementation. It is restored before every physical admission
and attempt notification; send-time AttemptAuthority remains unchanged.
`in-flight-generation-billing-valid-red.log` separately demonstrates lost
trustworthy usage when a response races generation replacement. Its preceding
similarly named log ran a filtered test before the helper was in that test, so is
NOT causal proof of the new helper. Billing now settles validated physical usage
before rejecting stale output, without publishing text, tools or checkpoints.
The failed `settled-runtime-and-controls.log` retains both guard failures; it is
not final GREEN. The patch was corrected using unambiguous contiguous blocks.

That matrix also identifies a stale ordinary Responses diagnostic expectation
from before BUG-264. The existing test and lifecycle scenario now explicitly
expect missing terminal evidence. All prior text, timing, request-count,
rollback, error-count, no-retry and no-final assertions remain. No shared
Responses production behavior changes here.

## Final-source runtime and quality

`verified-final-source-runtime.log` contains FIFTEEN complete targets:
**51 passed, zero failed/ignored/filtered**. These comprise nine acceptance tests
and 42 affected controls: actual native CLI tool/result and nudge composition,
visible refusal without either continuation, exact typed wire replay through
streaming and completion, all three hosted actions, malformed shapes, unsafe
terminals, protocol cap across transport replacement, full-body admission growth,
cancellation, generation/account replacement, genuine ancestry/model/history
controls, request usage, terminal lifecycle, native capability, checkpoint
authority, BUG-260/261 and all seven BUG-265 genuine-user/recovery controls.

`final-reload-and-traceability-controls.log` adds THREE distinct session-reload
controls and repeats all TEN lifecycle controls after the final step-comment
update. Total distinct runtime tests: **54 = nine acceptance + 45 controls**.
Repeated tests are not double counted. Every selected target completes; the
earlier failed matrices and filtered causal runs are not counted as GREEN.

`settled-shipping-check.log` and `final-shipping-clippy.log` check default shipping
libraries for providers, CLI, agent-loop, sessions and NAPI. `final-test-clippy.log`
checks both new test targets and the changed lifecycle control. The seven
previously disclosed NAPI warnings and reused-fixture dead-code warnings remain;
these commands succeed without disabling lints. Focused rustfmt and whitespace
checks pass in `settled-format-check.log` and `final-whitespace-check.log`.

All Cargo commands are offline, explicit package/target, ci-test, jobs two, with
command-scoped `CARGO_PROFILE_CI_TEST_DEBUG=0`; runtime debug assertions remain.
Final full matrices additionally isolate HOME/config/auth and inherited proxies.
Acceptance fixtures use temporary synthetic credentials, mock keyring and numeric
loopback. No installed scripts, real credentials, remote inference or issuer
services are used. Available disk remains about 16 GiB, above the 6 GiB reserve;
this replacement performs no cleanup. Local protocol tests do not establish live
provider support, workspace-wide success or other-platform compatibility.

## Traceability and direct review

All nine new scenarios retain their original step assertions and one primary test
mapping each. Native link commands append rather than update mappings; earlier
line-rebase attempts created duplicates. Native unlink/link now reconciles only
the nine owned scenarios and the one changed lifecycle control, with original
native artifacts copied to pre-reconciliation evidence. No coverage/work-unit
JSON is hand-edited, no scenario is deleted and no gate is weakened.
`reconciled-native-coverage.log` and `reconciled-coverage-audit.log` record the
successful exact mappings; `settled-spec-validation.log` records scoped format
and Gherkin validation. Native file-scoped tag checks pass. The known 522 unrelated
global tag violations remain outside scope and are not claimed green.

`settled-native-review-context.log` finds no native issues but summarizes only the
two-scenario composition feature, not all nine. Direct review additionally covers
all seven provider scenarios and their helpers: exact current-response append
order and opaque bytes; existing negotiated native capability enforcement;
initial/full-DAG ancestry; per-attempt generation/account admission; same-account
refresh; once-only physical usage and final aggregate/request identities;
refusal through both continuation layers; no tools on unsafe/incomplete output;
three protocol follow-ups independent of finite reconnect and task budgets; and
success-only checkpoint publication. New source/test helpers stay below 300 lines.
No durable/delta registry, new retry/UI policy or Copilot/shared Responses policy
is introduced. Unpublished batch and user changes remain intact.
