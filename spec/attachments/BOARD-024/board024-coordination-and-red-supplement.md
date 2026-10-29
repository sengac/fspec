# BOARD-024: coordination resolution and independent specification/RED supplement

This supplements, rather than replaces, the original requirements, discovery
evidence and refactor proposal. Earlier evidence describes the state at its own
timestamp. No original authorization is broadened by this supplement.

## Coordination resolution

The parent's follow-up cleared exactly one shared hunk:

- BOARD-024 owns `try_dispatch_card_dialog(action)` registration in
  `rust/fspec-tui/src/app/dispatch_capability.rs`, plus its independent dialog
  modules, specifications and tests.
- BUG-283 retains navigator.rs, auxiliary routing/helper, its own specifications,
  tests and provider/model regressions. BOARD-024 made no edits to these files.
- This does not authorize oversized-file additions or broader refactoring.

Supported metadata records this resolution in architecture notes and the answer
to discovery question 1. Question 0's answer records the known current scope:
neither production option is authorized; only independent specs and RED tests
are authorized now. Answering that question is not production approval.

## Workflow and specification

The first ordinary transition to testing was rejected for missing AST research.
Actual AstGrep queries and their raw output were then attached as
`ast-research-board024-enter-and-event-routing.md`. The next ordinary transition
to testing succeeded. No skip flag, gate bypass, implementing or Done transition
was attempted during this follow-up.

Feature:
`spec/features/board-card-enter-chooser-and-complete-details.feature`

Seventeen scenarios cover Board/Mux choice and cancellation, captured intent,
normal/isolated/cancel agent creation, empty-board control, complete daemon-owned
canonical information through both transports, strict read errors, same-card
request generations, dismissed/cross-card responses, Unicode wrapping and
scrolling/resize, linked cycles/back history, exact attachment paths and contextual
errors, terminal sanitization, modal key/paste isolation, mouse hitboxes and hints.

The acceptance docstring incorporates `/tmp/board024-independent-review.md`:
user story, state/legacy histories, legacy updated fields, blocked reason,
event storm, virtual hooks, iteration/token counts, answered questions, deleted
discovery state and unknown fields must not be dropped. Paths and identifiers
remain original for actions; display sanitization cannot rewrite them. Stale
generation handling includes same-ID reopen and must cover errors as well as
successes. Attachment action routing is not proof of browser launch. Existing
lexical traversal behavior is preserved without a stronger symlink claim.

Final supported checks:

```text
format {"file":"spec/features/board-card-enter-chooser-and-complete-details.feature"}
{"formattedCount":1}

validate {"file":"spec/features/board-card-enter-chooser-and-complete-details.feature"}
{"success":true,"exitCode":0,"invalidCount":0}

validate-tags {"file":"spec/features/board-card-enter-chooser-and-complete-details.feature"}
{"validCount":1,"invalidCount":0}
```

An initial format invocation mistakenly supplied `files` rather than `file`,
which ran formatting globally. Four unrelated whitespace-only feature diffs
were restored immediately; subsequent status/diff checks confirm no retained
unrelated formatting changes. All subsequent format calls use the single-file
argument. Coverage generation has no file-scoping argument: a dry run revealed
three unrelated missing sidecars, so actual generation used a temporary staging
project containing only BOARD-024's feature. Only its generated sidecar was
copied back, then populated using supported `link-coverage` commands. After test
review/formatting, mappings were reset to that generated empty sidecar and
relinked with final ranges, avoiding duplicate old-range mappings.

## Current-API behavioral tests

Integration target: `rust/fspec-tui/tests/board024_enter_chooser.rs`.
Four scenario modules contain 17 tests with exact Gherkin `@step` comments.
All scenarios have one test mapping and zero implementation mappings. Helpers
are split into small modules; every new Rust file is below 300 lines.

Tests send actual crossterm key events through App::handle_event, dispatch the
existing action bus, and inspect actual App render buffers/stores. They do not
add proposed Action variants, backend methods or production stubs merely to
compile. Empty-column Enter is a nontrivial passing control. Fixtures reuse
existing intent/isolation support, shared test-helper temp directories, real
stores, real embedded RPC and a real WebSocket setup in the transport branch.

Canonical fixture data differs deliberately from board summaries. The detail
harvester paints compositor layers alone to avoid mistaking the underlying board
summary for canonical dialog content. Buffer extraction accounts for wide Unicode
cells. Focus helpers retain requested dimensions; mouse coordinates target the
label rather than the selection marker. Delayed completion tests await actual
pending tasks while withholding their actions, rather than relying on sleeps.

## Exact RED command and receipts

Working directory: `/home/rquast/projects/fspec/rust`.

```bash
CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-tui --test board024_enter_chooser -j 2 -- --test-threads=1
```

The command ran under `/bin/bash -c` with pipefail and tee. The final run captured
the actual Cargo pipeline exit code using PIPESTATUS, not tee's status.

Original raw receipt: `board024-red.log` (before review refinements).
Intermediate receipt: `board024-red-reviewed.log` (two test-authoring compile
errors, corrected; not behavioral RED evidence).
Final raw receipt: `board024-red-final.log`.

Final meaningful raw lines:

```text
Finished `ci-test` profile [unoptimized] target(s) in 6.77s
running 17 tests
test result: FAILED. 1 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
error: test failed, to rerun pass `-p codelet-fspec-tui --test board024_enter_chooser`
Cargo exit status: 101
```

The empty-board control passed. Fifteen tests fail at the assertion that actual
Enter offers both Open in agent and Show full details. Existing behavior instead
enters the associated agent or opens the existing story-specific creation dialog;
the Mux test observes the same missing chooser via the real Mux event path.
The remaining failure is the missing chooser hint in the board footer.

**Evidence boundary:** this establishes the missing chooser/footer behavior.
It does not independently demonstrate stale-load, attachment, Unicode or canonical
projection defects: those test bodies stop at the earlier chooser precondition.
The WebSocket iteration is prepared but not reached because embedded chooser
setup fails first. These deeper assertions are executable future acceptance
checks, not claimed GREEN results or separately observed causal RED failures.

Compilation reports eight dead-code warnings for unused functions in the reused
existing intent fixture. No lint suppression or edits to that fixture were added.
The initial unused WorkUnitFocus import in the new root was removed.

## Independent review and refinements

Read-only review verified current-API compilation and identified assertion risks.
The revised tests now assert the chooser before sending modal shortcuts, establish
Board/Mux preconditions, include multiple associated Mux sessions, refresh card
selection before unassociated entry, assert exactly one confirmed assignment,
exercise the visible Cancel option, distinguish neutral retired-rule text from
the historical/deleted indicator, use distinct relationship IDs, avoid attachment
filenames that satisfy error text themselves, and do not conflate attachment
action emission with browser launch. Same-ID stale responses are delivered both
while the new generation is loading and after it completes. The stale-hitbox
negative check is unconditional and exercises an old point still inside the
terminal after manual scrolling. Narrow focus helpers no longer force 120x36.

Remaining review work when implementation is authorized: bind held-response
assertions to the new typed completion kind/card/generation once that API exists;
test a real launch failure via an appropriate existing/new test seam rather than
claiming unavailable-viewer coverage proves browser error handling; verify readable
field grouping and exact display contracts against the implemented renderer.
No production change is justified solely by an unexecuted deeper assertion.

## Formatting receipt

```bash
rustfmt --edition 2021 --config skip_children=true --check rust/fspec-tui/tests/board024_enter_chooser.rs rust/fspec-tui/tests/board024/support.rs rust/fspec-tui/tests/board024/canonical.rs rust/fspec-tui/tests/board024/chooser.rs rust/fspec-tui/tests/board024/loading.rs rust/fspec-tui/tests/board024/details.rs rust/fspec-tui/tests/board024/attachments.rs
```

```text
rustfmt exit status: 0
```

Final mapped test-module lengths: chooser 229, loading 229, details 245,
attachments 147; helper lengths: canonical 96, support 203.
Coverage report: 17/17 test mappings, four scenario files, zero implementation
files. The tool's 100% label means mapping coverage, not passed behavior or Done.

## Handoff boundary

Canonical state is testing, not Done. Production edits remain paused. The parent
has resolved shared-file coordination only. The outstanding decision is explicit
approval of an oversized-file exception or a separately bounded refactor, as
described in `board024-narrow-integration-proposal.md`. No installation, daemon
restart, commit, push or security/credential change occurred.
