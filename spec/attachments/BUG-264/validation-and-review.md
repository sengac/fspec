# BUG-264: ordinary Responses terminal semantics

## ACDD and evidence

Rules/examples and five Gherkin scenarios preceded real loopback tests.
`red-terminal.log`: 1 control passed / 4 causal failures. Failed backend details
became missing usage, incomplete emitted queued calls without final metadata,
actual attributed native Done concealed invalid usage, and valid accepted Done
was not physically billed. No live Codex cause is claimed from this latent
ordinary Responses adapter defect.

The oversized 481-line streaming module was mechanically separated into public
schema/response types and a transport module before repair. Public type paths,
request body, trace fields, incremental text/reasoning and native call IDs were
preserved. The old unwrap_err was replaced by equivalent fallible matching.
`mechanical-control.log` passes the actual local explicit-error/completed control
on that split, before terminal changes.

The repair adds a typed ResponsesTerminalFailure preserving backend code/message
through the error source chain. Failed responses cannot release tools. Incomplete
max_output_tokens and content_filter retain exact usage with normalized stop
metadata and cannot execute pending tools; other incomplete causes fail explicitly.
Status/identity mismatches, failed-without-error and missing/inconsistent usage
remain failures. An authoritative Responses terminal closes parsing without EOF.
Validated usage and physical Final precede executable calls, so the existing
settlement hook bills before accepted Done drops consumption. Explicit type:error
continues to fail; unsupported nonterminal frame handling remains compatible,
not a new unconditional rejection policy. No executable work is drained after Done.

`green-terminal-matrix.log`: new 5 + BUG-263 5 + BUG-262 3 = 13 passed.
`green-codex-controls.log`: terminal 7 + completed answers 1 + typed history 2 =
10 passed. Dedicated Codex receives only stop_reason: None in its shared-final
initializer; its existing terminal/replay policy is unchanged. Two excluded-rig
usage-test struct literals receive the same compatibility default. Their direct
unit/integration runner remains unavailable as recorded by BUG-262; no pass is
claimed for those excluded targets.

After preserving the original nonterminal skip policy in review,
`green-final-source.log` passes all five new tests on final source.
`clippy.log` and `check.log` pass the scoped CLI target/default compilation.
Only dead-code warnings from reused test helpers remain; no suppression was added.
Affected matrix totals 23 passed, with no failure. Scoped formatting, whitespace,
native feature/tag validation and five-scenario coverage audit precede native Done.

## Direct review

Reviewed streaming schema, transport, terminal evaluator, real request fixture,
five tests, native Done/attribution and shared stream-loop settlement, the one-line
dedicated Codex compatibility addition and both excluded test literals. Actual
HTTP requests use numeric loopback and synthetic keys only. Native Done is armed
through normal disposable admission; tests reject attribution errors, assert no
trailing filesystem effect/nudge, exactly one request and exact persisted billing
20 input / 30 output, with distinct item-0/call-0 preserved in tool results/history.
All touched files are below 300 lines. There are no new credentials, external
requests, model/session changes, downloads, commits or publication.
