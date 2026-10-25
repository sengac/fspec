# BUG-258: ACDD execution and validation

## Authorization and lifecycle

The human approved the concrete bounded extraction on 2026-10-25, conditional
on native Fspec card scope/status updates and new Markdown attachments before
starting. The approval is recorded in the card description and architecture
note; `extraction-review-plan.md` explicitly supersedes the earlier hold.
Native Fspec attached that plan and the original `red.log` before implementation.
The card returned to specifying for the applied-compaction budget criterion,
then testing for its real-path regression, then implementing. No gate override
or manual `work-units.json` edit was used.

## Acceptance evidence before production changes

`red.log`: three cold-replay failures and two passing controls. The ignored
child entry point is explicitly invoked by parent tests in fresh OS processes.

`applied-control-red.log`: the sixth scenario adds a fourth behavioral red;
the original three reds and two greens remain. Four successful builder calls
and four inference calls reach the applied-compaction budget limit. The final
request has zero terminal records. Inspection of
`interactive/compaction_handoff.rs` and `interactive/stream_loop.rs` confirms
successful summary handoff breaks the stream before inner Done. This case
therefore needs no immutable-terminal rewriting, replacement request identity
or deferred-completion redesign.

At that pre-implementation stage, all six scenarios mapped 1:1 to tests in
`bug258_outer_terminal.rs`; the final refined specification has seven. Support code
reuses the existing automatic-compaction harness and shared temp-directory
helpers. The transport fixtures bind to loopback and use deterministic offline
HTTP responses. Cold replay uses real SessionManagers, restores twice, checks
stable ordering/identity and immutable archive evidence, and rejects terminal
prose or transcript metadata in model originals/history.

## Extraction review

The existing entry point remains the caller-facing function. Extracted phases
preserve user/supervisor/MCP priority and channel-close behavior, cancellation
epochs and notify handles, fresh-user-only counter reset, selected provider and
thinking resolution, request-owned output/journal, the inner mutex guard's
turn scope, registration order, ordinary-handler cleanup order, compactor
registration drop after settlement, and loop-abort retry scheduling after drop.
No new execution engine, publication hook or background recovery identity is
introduced. The oversized `background_output.rs` parent is not edited.

The existing MCP structural regression pins the guarded select in the
orchestrator. Review retained that small selection block inline rather than
change unrelated scenarios/tests. All five unchanged RPC-089 source contracts
pass. The orchestrator is 233 lines; helper modules and the 55-line recovery
child remain below 300 lines. This is the smaller-grouping option permitted by
the approved extraction plan.

The terminal adapter is `BackgroundOutput::finish_outer_failure` in
`background_recovery.rs`, called by final settlement for abandoned errors and
applied-compaction budget exhaustion. It emits Error and Done through the same
existing output/journal owner, setting Idle before Done. Existing journal
admission provides safe typed failure metadata and deduplication; successful
recovery does not call this method. `background_output.rs` remains
byte-identical to HEAD. The bounded follow-up journal distinction and recovery
owner correction are described below.

### Seventh-case refinement and owner correction

The first broader run found a real compatibility regression: the existing
repeated-builder-failure test expects an actionable Error after the builder's
CompactionFailed warning. Blanket deduplication suppressed that Error. Native
Fspec returned to specifying/testing, recorded the refined rule/example and
architecture note, mapped a seventh scenario, and attached `diagnostic-red.log`
(zero actionable Error chunks). Only then did implementation resume.

`StreamJournal::terminal` now permits one live ProviderError diagnostic after
an acknowledged CompactionFailed terminal, without appending or replacing a
terminal. A per-owner flag suppresses repetitions. ProviderError duplicates and
immutable Completed outcomes remain suppressed. This uses existing admission,
not a global publication hook.

Stronger failure counting then exposed an existing second output allocation in
`try_terminal_overflow_recovery_owned`: recovery had its own request/journal,
separate from the outer owner. The owned helper now borrows the original turn's
output. The public locking wrapper retains its existing standalone output and
unchanged public signature. This fulfills the approved same-owner invariant;
no new identity conceals duplication.

`green.log` was reused during iteration and now preserves the intermediate
two-failure run that exposed those duplicate safe outcomes; it is **not** the
final green evidence. A subsequent test-helper correction counts the documented
CompactionFailed cold warning notification, not just ProviderError Error chunks.
Native testing recorded that fixture correction separately. No production replay
mapping or safe message was changed to satisfy the test. `final-green.log` is
the completed definitive result: **7 passed, 0 failed, 1 ignored child entry**,
with that child explicitly run by all seven parents and each archive restored
twice. Both failure types are checked against model-history pollution.

## Interruption and rebuild

At the human's request the supervising caller interrupted this session and
ran `cargo clean` in the verified `rust/target` directory after checking that
no build or patch process remained. It reported 38,548 files / 25.1 GiB removed;
actual free space rose from 6.52 to 30.62 GiB. Source, evidence, other caches and
uncommitted changes were preserved. Resume inspection finds the native card
still implementing and `git diff --check` clean. The partially interrupted
baseline is not accepted as completed test evidence.

## Validation ledger

Commands run from `rust/` use `--offline --profile ci-test`, explicit packages
and test targets, and `-j2`. No workspace/unscoped test command is permitted.
The 6 GiB disk floor is monitored between builds. The approved command matrix
is in `extraction-review-plan.md`.

- Post-clean extraction baseline: completed, four behavioral reds and two greens.
  `extraction-red.log` preserves the full offline rebuild and run.
- Reviewed smaller-grouping baseline: completed, same four reds and two greens,
  plus five passing unchanged RPC-089 tests (`extraction-reviewed-red.log`).
- Initial minimal terminal adapter: six passing acceptance tests observed before
  the seventh-case refinement; final complete evidence is `final-green.log`.
- Refined same-owner implementation: seven passing acceptance tests, zero
  failures; the child entry explicitly runs in seven fresh OS processes.
- Scoped runtime regressions: executed as summarized below, not all green.
- Library check and Clippy for core/providers/cli/sessions/agent-loop: pass.
- BUG-258 test Clippy: pass; reused fixture unused-member warnings only.
- Gated `test-support` agent-loop library check: pass.
- Native feature/tag validation and targeted rustfmt/diff checks: pass.
- Native coverage audit: all 15 mapped file references found and valid. Additional
  read-only verification confirms seven 1:1 test mappings, exact step comments,
  nonempty implementation mappings and every mapped line in bounds.
- Native validating and done transitions: completed after acceptance green,
  implementation mappings, executed quality checks and direct review. Feature
  tag is `@done`; final scoped native validation and coverage audit pass.
  The card was subsequently reopened to validating solely to classify the five
  broader failures. The executed isolated baseline and full current-code
  comparison below complete that classification; final native closure uses
  these existing results without rerunning validation or changing code.

### Executed test matrix

All entries use the offline ci-test profile and `-j2`. These counts exclude
explicitly invoked child-process helper repetitions and repeated reruns.

| Scoped test targets | Observed result | Evidence log |
| --- | --- | --- |
| `bug258_outer_terminal` | 7 acceptance passes; ignored child explicitly run by all 7 parents | `final-green.log`, `regression-lifecycle-final.log` |
| `bug210_automatic_continuation`, `cmpct053_terminal_lock`, three `cmpct054` targets in plan | 15 passes, 1 missing-invoking-context fixture failure | `regression-lifecycle-final.log` |
| `bug211_compaction_budget_recovery` | Initially 3 passes/5 failures; final full suite 4 passes/4 baseline-reproduced token-fixture failures; repaired diagnostic case passes | `regression-recovery.log`, `baseline-classification-current.log`, `bug258-baseline-tests.log` |
| `bug211_explicit_retry_safety`, `rpc087_dispatch_routes_through_recovery_stream_shape`, `rpc089_mcp_injection_drain` | 8 passes on final rerun | `regression-routing-final.log` |
| Three `bug246` and four `bug250` agent-loop targets in plan | 19 passes; recovery child entry explicitly exercised | `regression-persistence.log` |
| Sessions `bug245_cold_empty_failures`, `bug250_transcript_replay` | 2 passes, including repeated fresh-process cold restore | `regression-cold.log` |
| `bug253_gemini_invocations`, `bug257_fork_continuation`, `prov149_codex_reload` | 23 passes | `regression-continuation.log` |
| Providers `prov149_codex_history` | 2 passes | `regression-provider-history.log` |

**80 distinct observed passing tests; 5 broader failures proved preexisting.**
This is not a claim of a fully green repository/workspace test suite. No
unscoped/workspace test command was run and no unrelated test was modified,
ignored, or filtered out of the original broader runs. The earlier diagnostic-only
rerun is supplemented by the completed full current-code comparison below.

Successful quality commands (from `rust/`):

```text
cargo check --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-agent-loop --test bug258_outer_terminal -j2
cargo check --offline --profile ci-test -p codelet-agent-loop --features test-support --lib -j2
```

Logs: `check.log`, `clippy.log`, `clippy-test.log`, `check-stub-feature.log`.

### Baseline-reproduced token-fixture failures

Four budget scenarios fail in `support/bug210_completion_metrics.rs:110` before
the exercised compactor path: the fixture computes 11,371 tokens but hardcodes
90,336. The affected scenarios are streaming synthesis, exhausted exploration,
invalid synthesized summaries, and provider-failure classification. That fixture
and its token-preparation implementation are unmodified by BUG-258. The executed
isolated baseline reproduces all four failures with the same 11,371 versus
90,336 assertion. The final full current-code suite has the same four failures.
The fifth initial budget-suite failure was the related live diagnostic regression
and is fixed: its unchanged repeated-builder-failure test passes on both the
baseline and final current code, and the new cold-replay case passes.

### Executed baseline and full current-code comparison

The isolated copy preserved earlier-card working changes and restored only
`rust/agent-loop/src/{agent_loop.rs,background_recovery.rs,stream_journal.rs,terminal_overflow_recovery.rs}`
from pre-BUG-258 HEAD `2699693b74124cf12b323997fd68dd491adf7e8f`.
The original working tree was not altered. `baseline-manifest.json` records
the restored paths and baseline/current SHA-256 values; `bug258-baseline-run.json`
records the executed command, isolated working directory, shared target path,
expected exit 101 and free-space measurements. `bug258-baseline-tests.log`
preserves its full output: budget suite **4 passed/4 failed**, dependency suite
**2 passed/1 failed**.

`baseline-classification-current.log` preserves the completed full current-code
comparison: budget suite **4 passed/4 same failures**, explicit-retry safety
**2 passed**, dependency suite **2 passed/1 same failure**, expected exit **101**.
All five failure names and substantive diagnostics match the baseline; invoking
context UUIDs and execution details naturally differ. No introduced failures
remain in these compared suites. `baseline-comparison.md` maps each failure and
the repaired diagnostic control. These existing logs were read and attached
during authorized finalization; no test, check, Clippy or build was rerun.

### Repository-wide specification check caveat

Native `fspec check` inspected 2,120 feature files: Gherkin syntax passes, but
repository-wide tag and format checks fail on unrelated existing specifications
(including unregistered `@wire-format` / `@fanout` and formatting in
`rlcd-blocklist-security.feature`). BUG-258's scoped native feature and tag
validation passes. No unrelated feature is rewritten to hide global failures.
The tag-mutation command's legacy advisory also uses hardcoded component/group
lists that omit this feature's registered `@agent-core` / `@session-management`
tags. Both are present in their required categories in `spec/tags.json`; they
are not replaced with unrelated tags to appease that advisory. No registry or
checker implementation is changed as part of BUG-258.

### Baseline-reproduced dependency-lifetime fixture failure

The lifecycle regression run passes eight automatic-continuation tests, three
terminal-lock tests, bridge-clear and cancellation-cleanup controls, and two
dependency tests. The unmodified
`scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup` fails with
`invalid work_unit_id attribution: unknown invoking context` before the expected
HTTP builder rejection. Its shared `terminal_recovery::fixture` constructs a
BackgroundSession directly without an auxiliary invoking-context registration;
the BUG-258 automatic-compaction harness does register that scope and its real
outer-builder controls pass. These existing test/fixture/admission sources are
not changed by BUG-258. The isolated pre-BUG-258 baseline and final current-code
comparison both reproduce this failure at the same assertion with the same
unknown-invoking-context diagnostic. It remains a reported preexisting failure,
not a passing dependency test.

BUG-253/256/257 production/tests/evidence remain uncommitted and preserved.
BUG-254's native-Windows blocker and BUG-255's RLCD hold, red tests and
instrumentation remain unchanged. The unrelated pre-existing
`persist_token_state_signature_carries_reasoning` source-shape failure documented
on BUG-257 is outside this work and was not rerun or repaired here. No commit,
push, manual stash, checkpoint deletion, daemon restart, credential access or
external provider request occurred during implementation. The evidence-only
replacement-session authorization is recorded below.

## Final direct review

Review was performed in this same codex/gpt-6.1-sol session, without subagents or
model substitution. No outstanding BUG-258 acceptance or implementation defect
was found after the seven-case green run and quality checks. The orchestrator
and all edited/new Rust files are below 300 lines (largest new test: 249 lines).
The 739-line `background_output.rs` parent remains byte-identical to HEAD.
No lint was disabled to pass the checks. Required cancellation/Arc/tool-handle
clones remain; redundant last-use clones introduced by extraction boundaries
were removed and the acceptance/lifecycle matrix was rerun.

The original direct implementation review remains preserved above. The human
authorized one replacement codex/gpt-6.1-sol session solely to attach completed
baseline evidence and close the reopened card. This finalization preserves the
old session/history and does not retry compaction or repeat implementation review.

The completed current-code log records **15,076,065,280 free bytes** after its
run (approximately 14 GiB), above the 6 GiB floor. The five baseline-reproduced
failures and global specification failures remain explicit warnings, never
reclassified as passing tests. All source/evidence changes remain uncommitted.

## Final native closure

The authorized replacement session attached all three exact baseline files,
the completed current-code log and `baseline-comparison.md` through native
`add-attachment`. Byte comparisons confirmed the baseline copies are identical.
Native `update-work-unit-status` accepted **validating → done** without bypass
flags, recording the seven green acceptance cases, 80 scoped passes, successful
quality checks, five baseline-reproduced failures and global-spec warnings in
the completion reason. The existing feature tag was already `@done`, not `@wip`.
