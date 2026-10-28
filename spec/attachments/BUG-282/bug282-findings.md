# BUG-282 — native compaction ancestry investigation

## Preservation and scope

The separate native card is BUG-282. No extra agent/session was created, no model
was changed, and failed session `7c55a508-ff74-4a68-895f-c01e9bdbb03c` was not
resumed, edited, recovered, cleared or compacted. No protected on-disk history,
credentials, personal context or taken-over Mac task was inspected. Evidence is
the supplied bounded RPC capture, repository cards, attachments, specifications
and executable synthetic fixtures only.

Preserved originals and registered attachment copies have these SHA256 values:

- `native-compaction-failure.json`: `669fb95222e0a58a4cad1d14e62b052355ef1bdac5adc9c669c5b771ac43d5dd`
- `failed-compaction-session-evidence.json`: `b5e7bdaf1a49e491dc14bd6aa9c2dfbeb510a4af240417d5981a8f772fae3086`

BUG-280 and BUG-281 remain validating. Their initial Rust manifest hash is
`720c6378ba6debb4363c77f0b17f9f36eada287aa4155c243c66ac7c763ffdb0`.
The manifest verified clean from `rust/` before this task's test/production work.
An earlier verification from the repository root used the wrong relative base;
its missing-file reports are not source corruption evidence. Similarly, the
first evidence hash command used unsupported shell brace expansion; explicit
paths subsequently verified both original files. No denial was involved.

## Observed failure versus causal proof

RPC pages 25269–25284 show a genuine tool result, request-attempt start,
Compacting, builder progress, Idle, warning, the exact terminal failure, and Done.
Metadata reports codex/gpt-6.1-sol, Idle and no active compaction progress.
The failure is:

```
terminal compaction failed: compaction commit failed: compaction candidate rejected: selected request preparation unavailable: RequestError: Codex checkpoint lineage mismatch: assistant, tool result or DAG ancestry
```

The exception originates in `providers/src/codex/history_lineage.rs::assemble`.
`cli/src/session/selected_compaction.rs::validate_current_compaction_history`
serializes durable replay against the current selected checkpoint before the
manifest transaction may promote its replacement. That refusal is necessary.
The bounded supplied capture does not contain the entire checkpoint or original
conversation. It cannot identify every historical divergent item or prove this
was the only defect in the protected session. No live repair is claimed.

The isolated executable reproduction establishes a concrete cause for this exact
failure path: `handle_tool_result` first appends the genuine rig ToolResult to
runtime history, but manually strips quotes and replaces literal `\\n`, `\\t`,
`\\r`, escaped quotes and backslashes in the emitted StreamEvent. BackgroundOutput
uses that event for its journal and durable ToolResult persistence. The
checkpoint and runtime keep the canonical text; the archive receives changed
text. On selected compaction the same ancestry guard correctly rejects it.

This defect occurs after canonical rig tool-result parsing, so another unescape
is not a parsing fix. JSON results, literal escape sequences and Windows paths
can all change meaning. The repair must preserve the authoritative event text,
not relax the lineage checker or regenerate missing history.

## Related history, not interchangeable causes

- BUG-278 (done): provider and durable generation domains were conflated. Its
  causal refusal was `generation must advance with a selected model`, not the
  present ancestry error. Keep its explicit binding and atomic checks intact.
- BUG-272 (done): terminal-error tail surgery deleted accepted calls/results.
  Preserving those messages does not prevent a later persistence encoding bug.
- CMPCT-078 and CMPCT-080 (done): strengthened complete lineage and removed
  silent repair/fabricated results. These are safety controls, not regressions
  to bypass because compaction is refused.
- BUG-242 (done): pending native tool settlement at a compactor timeout is a
  separate ownership boundary. Initial failure-only evidence is explicitly
  superseded by its later full-repair attachments; it is not final success proof.
  Its completion-validation attachment establishes later 4/4 settlement and 5/5
  provider replay GREEN. Its truthful request-local unknown-outcome handoff is
  not a license to fabricate archived outcomes. The present failure occurs
  during selected parent history validation, not timeout synthesis settlement.
- BUG-257 (done): forked checkpoint provenance and child authority, not evidence
  of a cause in this non-forked preserved capture.
- BUG-212 (done): inactive Codex state was incorrectly consulted for a selected
  OpenAI transport. This incident's selected transport is Codex; its stricter
  refusal remains required. All four selected-provider regression controls pass.

All seven related cards are linked with non-blocking `relatesTo`. No active card
with the same concrete encoding fix was found; generic backlog/provider work is
not treated as duplicate active ownership.

Additional repository card evidence distinguishes other recurrent terminal
compaction symptoms: BUG-211 is a bounded sub-agent depth exhaustion; BUG-229 is
complete-request capacity exhaustion; BUG-232 is missing/unclosed DAG syntax;
BUG-233 is overlapping same-depth ranges; BUG-234 is a shared operation deadline
and synthesis runway investigation. Those are not this ancestry exception.
Their card descriptions sometimes cite historical log locations; none of those
protected logs was opened. Their initiating historical causes are not promoted
from likely to proven merely because their completed cards describe fixes.

## ACDD chronology and honest RED

Rules, examples and architecture notes precede the new three-scenario feature.
The feature was formatted and validated before tests. Native transitions are
backlog → specifying → testing; no production edit precedes executable RED.

Fixtures use real rig streaming, loopback HTTP Codex responses, real once-only
filesystem effects, BackgroundOutput, real persistence, selected request
serialization/admission, durable commit, publication and cold reconstruction.
Only the remote provider is scripted; no success path or lineage guard is mocked.
The existing production replay decoder source is reused in the integration test;
two test-only dependencies support that decoder, with no public API changes.

- `red.log`: missing test-only base64/url dependencies; compiler error, NOT RED.
- `red-executable.log`: three fixture timeouts, NOT causal RED.
- `fixture-diagnostic.log`: proves both HTTP requests and the genuinely different
  emitted outcome. Empty terminal responses caused the harness to request more
  inference; fixed the fixture to return real final assistant text, not to extend
  timeouts or bypass protocol completion.
- `causal-red.log` and `confirmed-red.log`: compiled positive cases fail on
  changed exact event text and the exact ancestry error above. A negative missing
  outcome fixture first indexed non-conversational metadata, then removed an
  original after checkpoint anchoring; corrected setup to construct the explicit
  invalid checkpoint after removal. Neither setup issue is credited as RED.
- `canonical-red.log`: positive causal failures persist; the attempted removal
  also correctly violates provider-root anchoring. This is not credited as a
  valid missing-ancestry fixture. A later attempt to alter final assistant text
  used the preceding request rather than the completed checkpoint, which does
  not yet contain that assistant output (`final-red.log`).
- `accepted-red.log`: the final fixture uses the genuine completed checkpoint
  and tests missing assistant ancestry without deleting anchored originals.
  **1 passing strict-refusal control, 2 behavioral failures**, compiled before
  any production edits. The failures are exact event-byte divergence and the
  identical selected-compaction ancestry error. The accepted test and helper
  source hashes are unchanged in the first GREEN run.

No assertion is weakened. Negative cases retain exact ancestry refusal,
unchanged manifest/checkpoint/originals, no publication and blocked inference.
The five negative variations are changed outcome, changed invocation identity,
changed existing assistant output, changed DAG ancestry, and checkpoint
assistant output missing from conversational originals. Existing BUG-278
controls additionally exercise genuinely absent outcomes/originals.

## Minimal repair and mechanical extraction

The only production behavior change removes the second quote/escape decoding
in `cli/src/interactive/stream_handlers.rs::handle_tool_result`. The existing
event now carries canonical parsed ToolResult text. The native journal, archive,
hooks and presentation receive the same authentic text as runtime/checkpoint
history. No provider lineage, admission, generation, commit or publication
implementation changes. Images/multipart outcome redesign is outside this
textual-integrity scope.

The existing 451-line file exceeded the mandated limit. Before the behavioral
edit, its existing test module was mechanically extracted through a cfg(test)
include into `stream_handler_tests.rs`. A byte comparison against HEAD proves
the complete existing module is unchanged, including every assertion. The main
file is now 259 lines and the extracted file 180. One initial extraction patch
failed on the exact timeout-string context; read-back proved no partial edit,
then the corrected patch applied and was compared byte-for-byte. No major
refactor, lint suppression or unrelated cleanup occurred.

`green.log` executes all three scenarios successfully. It includes actual
first/successive admitted promotions, observed committed identities, generation
binding checks, cold provider restoration, real loopback continuation and
once-only filesystem effects. It also retains all five altered-ancestry vetoes.

Strict Clippy initially found a cloned single-element expected array and a
synchronous request-capture guard held across a later async runtime lock in the
new test. The final test uses `slice::from_ref` and a lexical capture scope.
Every assertion is retained unchanged in meaning; no allow/suppression is added.
`accepted-red-test.rs` and `accepted-red-fixture.rs` preserve the exact RED/GREEN
source hashes, and `test-lint-only.diff` records these final test-only edits.
`clippy-final.log` passes with `-D warnings` for both owned libraries and the new
integration target.

## Final verified results

There are **81 distinct passing scoped tests** across 19 integration targets
and the 23-test existing handler unit module. Child-process repetitions in two
atomicity targets and repeated acceptance executions are not counted twice.

| Scope | Distinct passes |
| --- | ---: |
| New canonical-outcome acceptance | 3 |
| CLI: BUG-278, BUG-272, BUG-212 | 17 |
| Provider: full lineage, strict replay/preflight, streamed/parallel ancestry | 17 |
| Core: checkpoint preservation, generations, publication ownership, lost ack, promotion validation | 9 |
| Agent-loop: BUG-242, native reload, persistence recovery, transcript archive | 12 |
| Existing unchanged handler unit module | 23 |

`final-check.log`, `clippy-final.log` (`-D warnings`), `library-build.log`,
`formatting.log`, `final-acceptance.log` and `final-preservation.log` all exit 0.
Only the selected CLI/agent-loop libraries were built, never the denied
standalone executable. New test source lint changes are covered by the final
check, strict Clippy and 3/3 final acceptance execution. All production sources
are unchanged since the earlier 81-test regression batch.

The feature validates after its @done tag; scoped tag validation has zero
errors and coverage audit reports all 6 mapped files found. There is exactly
one executable test per scenario with all literal step annotations. Global
spec/tag rewrites and the denied legacy feature are deliberately not invoked.

Final preservation verifies every original Rust-manifest entry except the three
explicitly owned changed files: CLI handler, agent-loop test dependency manifest
and generated Cargo.lock. Every other original Rust file, all BUG-280/281 handoff
specification hashes, the extracted unit module and both failure attachments
verify. BUG-280 and BUG-281 remain validating. No denial or approval is pending;
all failed attempts are preserved and classified, never relabeled as GREEN.

## Validation and provenance

Final results are recorded separately after implementation. Every Cargo command
is package/target scoped, offline, ci-test/debug0, -j2, with an isolated temporary
HOME, FSPEC_USER_DIR, FSPEC_HOME, CODEX_HOME, XDG config/data/runtime and TMPDIR.
No real credentials/history are available to the fixtures. Commands, exit codes,
raw logs and per-run full Rust source hashes are retained under
`/tmp/fspec-bug282-evidence/`. The target cache belongs to this same original
workspace path (`/tmp/fspec-bug280281-target`); no copied pre-fix/final tree shares
it, no sources are replaced or timestamp-touched, and no cache is cleaned.
Source HEAD at task start: `042d3974b4dbdf208fa006d7474edd504fd4ea5f`.

No standalone fspec executable build, installation, restart, commit, push,
security change, legacy enter-key architecture edit or denied issue-document
edit is part of this investigation.
