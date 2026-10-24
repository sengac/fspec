# BUG-250 canonical completion audit

## Verified canonical results

`/tmp/bug246-250-final-integrated-cargo.log` completed at 18:06 on 2026-10-23.
The BUG-250 targets all pass:

| Package | Target | Passed |
| --- | --- | --- |
| codelet-agent-loop | bug250_deleted_recovery | 1 |
| codelet-agent-loop | bug250_recovery_retirement | 7 |
| codelet-agent-loop | bug250_request_anchors | 1 |
| codelet-agent-loop | bug250_terminal_archive | 2 |
| codelet-core | bug250_archival_journal | 2 |
| codelet-core | bug250_request_boundaries | 3 |
| codelet-core | bug250_transcript_lifecycle | 3 |

The retirement target's ignored test is its explicitly invoked cold-subprocess
entry point. These are 19 passing acceptance scenarios, including canonical
deletion, retained memory, stale durable recovery, active ownership, precommit
failure, postcommit cleanup, causal boundaries and pinned archival snapshots.

The historical source findings in recovery-deletion-audit.md and
unanchored-request-audit.md are addressed by these canonical passing targets;
they are not still-open source findings. Historical direct-runner results alone
are not used to close the remaining canonical execution gaps.

## Remaining canonical invocation

From `rust/`, `/tmp/bug249-250-completion-cargo.log` executes:

```sh
cargo test --profile release-slim -j2 -p codelet-core --test bug249_transcript_journal --test bug250_transcript_outcomes
cargo test --profile release-slim -j2 -p codelet-sessions --test bug250_transcript_replay --test bug250_cold_terminal_display
cargo check --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib
cargo clippy --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib --no-deps -- -D warnings
```

No duplicate successful integrated target is rerun. The supplied cold-session
log `/tmp/bug245-final-green.log`, last modified at 18:11, actually ends in
missing LTO output compilation failures, including bug250_cold_terminal_display.
Its name is not passing evidence. The new invocation preserves the existing
profile/target and is rebuilding missing artifacts, without cleanup or a direct
compiler fallback. A supervisor coordination message about these discrepancies
was rejected as busy and was not delivered; this attachment records them.

## Traceability and status

`/tmp/bug249-250-completion-coverage.log` records successful fspec path audits,
100% scenario mappings and scoped syntax validation for all 16 combined features.
BUG-250 has 25/25 scenarios across ten features and one test mapping per scenario.
Source-span checks confirm existing test functions and in-bounds implementation
ranges. This is traceability rather than measured Rust branch coverage.

Scoped formatting passes with `FORMAT_EXIT=0` in
`/tmp/bug249-250-completion-format.log`. All 16 owned features pass registered-tag
and feature-level work-unit placement checks. Global `fspec validate-tags` is
still nonzero for 526 other features; its exact output is
`/tmp/bug249-250-completion-tags.log` and is not waived or called GREEN.

No production edits, commits or manual work-unit changes are made by this audit.
BUG-250 remains implementing until canonical tests and scoped quality checks
complete; transitions and attachment registration use fspec commands only.

## Missing core outcome target executed

`/tmp/bug249-250-completion-cargo.log` now records canonical Cargo GREEN for
`bug250_transcript_outcomes`: **4 passed, 0 failed**. This verifies **23/25**
BUG-250 acceptance scenarios through Cargo. The cold-display and shared-decoder
targets and scoped quality commands remain queued behind the shared target lock.
No new production repair was required by the core execution.

## Canonical cold-test RED and fixture correction

The fresh canonical sessions run completed with one actual failure:
`/tmp/bug250-cold-model-count-red.log` preserves the full original
`/tmp/bug245-completion-sessions.log`, `CARGO_EXIT=101`. The shared transcript
decoder passes. Cold display passes its exact prefix, safe failure, identity and
timestamp checks, then the fixture's aggregate model-message count fails:
actual 4, expected 3 at the former test line 76.

The extra message is the existing runtime environment/focus supersession reminder
installed by `restore_activity_focus`, not a journal outcome or dialogue original.
The fresh-creation baseline has two runtime reminders; cold restoration validly
adds the focus reminder before installing the one authentic assistant original.
System reminders intentionally retain superseded versions for prompt caching.
Changing production restoration to discard one would violate that established
contract and is not a transcript repair.

After this executed RED, only the owned cold test is corrected: assert exactly
one non-system-reminder message, equal to `Message::assistant("cold authentic
prefix")`, while still checking the entire model input for both safe terminal
prose and the private credential sentinel. Existing acceptance already requires
display/model separation and predates the failing test; no new production repair
or weaker expected-count adjustment is introduced. No shared source is edited.

The queued canonical cold/replay command in
`/tmp/bug249-250-completion-cargo.log` will compile this corrected tracked test.
Its pending execution is not GREEN. Scoped rustfmt passes. Coverage is refreshed
through fspec unlink/link/audit commands in
`/tmp/bug250-cold-completion-coverage.log`, retaining exactly one scenario mapping,
the corrected full test span 14-90 and all six production implementation links.
BUG-250 is now validating via fspec, not done.

## Canonical cold GREEN

The queued actual Cargo command now completes both corrected cold/display and
shared decoder targets in `/tmp/bug249-250-completion-cargo.log`:

- `bug250_cold_terminal_display`: **1 passed, 0 failed**, including both repeated
  restores, exact dialogue equality, safe failure order and original metadata.
- `bug250_transcript_replay`: **1 passed, 0 failed**, with no model messages or
  model envelopes returned for transcript-only records.

All **25/25** BUG-250 acceptance scenarios now have canonical Cargo GREEN.
The cold RED is also preserved as registered attachment
`spec/attachments/BUG-250/bug250-cold-model-count-red.log`. The complete scoped
fspec audit was repeated after the fixture correction and confirms 18 BUG-249
plus 25 BUG-250 unique, in-bounds test mappings and valid syntax.

The sessions test compilation reports existing test-only dead-code warnings
from the shared cmpct060 fixture and source-included replay error variant.
They are not hidden or represented as strict all-target Clippy success. The
requested library check/strict Clippy commands remain separately queued.

## Final quality GREEN and completion decision

`/tmp/bug249-250-completion-cargo.log` now ends with
**`CARGO_COMPLETION_EXIT=0`**. The scoped three-package library `cargo check` and
strict `cargo clippy --lib --no-deps -- -D warnings` commands both actually pass,
using `release-slim -j2` and the existing target. The full output is registered as
`spec/attachments/BUG-250/bug249-250-completion-cargo.log`.

The preceding integrated canonical output is also preserved as
`spec/attachments/BUG-249/bug246-250-final-integrated-cargo.log`. These two logs
prove **25/25 BUG-250** plus **18/18 BUG-249** unique acceptance scenarios pass.
The ignored crash/retirement subprocess helpers are executed explicitly by their
passing parent cases and are not missing acceptance coverage.

After the cold fixture correction, the final scoped fspec audit confirms all
**43 scenarios**, one test mapping each, existing test functions, in-bounds source
ranges, complete implementation links and valid syntax across all 16 features.
Formatting and scoped registered-tag/placement checks pass. The actual cold
runtime RED, narrow fixture correction and actual canonical GREEN are recorded
above; no production change or shared-source edit is made by this final audit.
Existing staged changes are preserved, with no commit or manual work-unit edit.
There is no remaining owned behavioral, coverage-mapping or library-quality gap.
The done transition is performed through fspec only after these results.

## Explicit scope limitations

- This is scoped library Clippy, not `--all-targets` Clippy or measured Rust
  branch/line coverage. Existing test-only dead-code warnings remain visible.
- Global `fspec validate-tags` still reports **526 unrelated violating features**;
  all 16 owned features pass. No global tag GREEN is claimed.
- The latest broader TUI quality log at this audit,
  `/tmp/bug244-245-completion-quality.log`, exits 101 for
  `clippy::collapsible_match` at
  `rust/fspec-tui/src/store/agent_view/token_state.rs:100`. That source is
  supervisor/BUG-244-owned and is not changed here.
- `/tmp/bug242-252-follow-goal-canonical.log` exits 101 while compiling
  `bug242_checkpoint_settlement` and `bug234_elapsed_runway`: LTO output paths
  disappear with `No such file or directory`. This is not behavioral RED and is
  not passing evidence for those supervisor-owned cards. No cleanup, alternate
  target or direct compiler workaround is performed by this audit.

These broader gaps do not invalidate the completed BUG-249/250 scoped commands,
but they must not be omitted from the supervisor's whole-goal assessment.

## Board completion

The normal fspec transitions **implementing -> validating -> done** succeed.
`fspec show-work-unit BUG-250` independently confirms **Status: done** in
`/tmp/bug250-completion-status.log`. No temporal/workflow bypass is used. All three
registered canonical/RED log copies compare byte-for-byte with their executed
`/tmp` originals. The final changed Rust test passes rustfmt and diff whitespace
checks, and the staged pre-existing content remains preserved without a commit.

## Final cross-card specification boundary audit

The coordinator subsequently identified and corrected a BUG-249-only feature/file
boundary gap: five semantic-admission scenarios move unchanged into
`transcript-semantic-admission.feature`, leaving two ordering/integrity scenarios
in the original feature. BUG-250 acceptance, tests and production are untouched
by this factoring. The verified combined total is now **17 features**, each with
exactly one test file and matching feature header, and still **43** unique
scenarios (**18 BUG-249**, **25 BUG-250**). The exact original criteria and all
mapping objects are preserved. Evidence is
`/tmp/bug249-factored-coverage-verification.log` and BUG-249's appended coordinator
standards-correction section. This supersedes the earlier 16-feature count only;
all registered actual Cargo, RED and quality evidence remains intact.
