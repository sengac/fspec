# BUG-213: misleading compaction counter and determinate progress bar

## Scope and evidence

- Investigation date: 2026-10-21.
- Source reviewed: `d70f583179894f60a4f68937139f09f7d0fedb39`.
- User-provided screenshot:
  `/Users/rquast/Desktop/Screenshot 2026-10-21 at 11.21.31 am.png`.
- Screenshot footer: `[compacting: Compaction builder working 0/153]`
  followed by ten empty bar cells; input area: `Compacting... (Esc to stop)`.
- This is a source review and screenshot inspection, not a new live-provider
  reproduction. The screenshot predates some recent lifecycle changes; findings
  below describe the current source, including BUG-210/BUG-211/BUG-212 changes.
- No production code was changed or runtime tests executed for this investigation.

## Conclusion

The display is **reachable and wired up**, but the displayed fraction is not a
measurement of completed compaction work. It is an inherited determinate-progress
presentation fed by a one-time placeholder. Calling the whole path dead code
would be incorrect; the misleading counter/bar semantics should be replaced.

The screenshot's `153` is consistent with an active in-memory **message count**,
not a verified number of conversation turns, captured persisted originals, builder
iterations, or completed DAG coverage. Its exact runtime source cannot be proven
from the screenshot alone, but the current producer emits the exact phase text.

## End-to-end wiring

1. `rust/cli/src/compactor_sub_agent.rs:103` emits CompactionStarted and one
   CompactionProgress event with phase `Compaction builder working`, `current=0`
   and `total=max(session.messages.len(), 1)`. There is no increment in this run.
2. `rust/agent-loop/src/background_output.rs:553` sets the session Compacting;
   `rust/agent-loop/src/background_output.rs:561` forwards progress into the
   BackgroundSession store.
3. `rust/sessions/src/background_session.rs:1567` saves progress and broadcasts
   `StreamChunk::CompactionProgressUpdate`, including clears (`None`).
4. `rust/fspec-tui/src/app/dispatch_stream_chunks.rs:52` reduces those updates
   into the original session's progress slot.
5. `rust/fspec-tui/src/views/agent/chrome_paint.rs:105` reads the viewed session's
   slot and supplies it to SessionFooter.
6. `rust/fspec-tui/src/views/agent/footer.rs:112` unconditionally renders the
   phase and `current/total` fraction. At `:134` it derives filled bar cells from
   `round(current / total * 10)`. A fixed zero therefore stays empty throughout.
7. Separately, `rust/fspec-tui/src/views/agent/animation.rs:35` derives the busy
   display from SessionStatus, and
   `rust/fspec-tui/src/views/agent/input_transition.rs:173` paints
   `Compacting... (Esc to stop)`. The spinner and footer have distinct inputs;
   neither proves that the fraction is live progress.

BUG-210 added the push delivery path; it did not make the production builder
produce meaningful intermediate completed-work counts. The progress query in
the backend/transport remains available to snapshot consumers, but the live TUI
does not depend on polling it. Absence of a polling caller is not a broken wire.

## Why the numbers do not make sense

| Producer | Denominator | Numerator |
| --- | --- | --- |
| `rust/cli/src/compactor_sub_agent.rs:104` | Active messages, minimum one | Zero |
| `rust/cli/src/interactive/stream_loop.rs:323` | Approximate `messages.len()/2`, minimum one | Zero |
| `rust/cli/src/interactive/recovery_compaction.rs:381` | Approximate `messages.len()/2`, minimum one | Zero |
| `rust/agent-loop/src/turn_compaction.rs:192` | Synthetic one-operation total | Zero |

These units are incompatible even before considering repeated compaction, tool
messages, reminders, retained suffixes, or a persisted source prefix different
from active history. The pre-prompt path also emits `Analyzing context` and then
the builder's `Compaction builder working`, potentially changing the denominator
without accomplishing any measured work.

`rust/agent-loop/src/compaction_builder.rs:56` captures authoritative evidence and
builds a validated DAG. `rust/agent-loop/src/compaction_execution.rs:8` executes
multi-turn exploration, with possible final synthesis and candidate correction.
Neither exposes a completed-source-record progress contract. Reading a record,
spending a tool turn, and validating/committing a DAG are not interchangeable
units of percentage complete. Incrementing the counter after each tool call or
SessionSearch page would invent a different misleading percentage.

Authoritative **final coverage** is a separate concept:
`rust/fspec-tui/src/app/dispatch_committed_compaction.rs:42` reports covered turns
and token reduction from a committed snapshot. Preserve that distinction.

## History and existing coverage

- `git blame` attributes the footer fraction and bar implementation to
  `3affacdd6` (2026-05-31, original `codelet/` path); comments explicitly mirror
  the older TypeScript/Ink footer. Legacy presentation, not unreachable code.
- RPC-047 (`spec/features/slash-command-compact.feature`) owns the original
  compaction footer contract; its tests in
  `rust/fspec-tui/tests/slash_compact_rpc047.rs:347` feed handcrafted `5/10`
  progress and verify the chip/bar. That proves rendering, not real progress.
- `rust/fspec-tui/src/views/agent/footer_tests.rs:63` likewise tests arithmetic
  and handcrafted rendering.
- BUG-210's `spec/features/automatic-compaction-progress.feature` requires live,
  original-session progress and cleanup.
  `rust/fspec-tui/tests/automatic_compaction_progress_bug210.rs:11` starts a
  pending builder but injects `Summarising 1/4` and `2/4` from the test itself.
  This exercises real embedded/WebSocket delivery, not builder instrumentation.
- The cleanup test in that file uses task abort for cancellation. Keep its
  cleanup invariant, but do not mistake it for an Escape integration test.
- Related cards: BUG-210 (lifecycle delivery), CMPCT-068 (committed occupancy and
  honest final coverage), BUG-214 (Escape cancellation). No unfinished card with
  this specific placeholder-counter defect was found in the work-unit review.

## Recommended fix boundary

1. For the current DAG builder, replace the fraction and determinate bar with
   an indeterminate activity/phase display. Reuse or consolidate the existing
   spinner rather than adding a second fake progress indicator.
2. Use real lifecycle transitions only: for example building, validation and
   commit/publication if those stages actually have instrumented boundaries.
   Do not fabricate percentage, ETA, completed turns or heartbeats as progress.
3. If source size is useful, label it explicitly as an input quantity with a
   defined authoritative basis, not as an advancing fraction.
4. Preserve original-session ownership, focus isolation, cleanup on all terminal
   outcomes, and final authoritative coverage/token reporting.
5. Review all producers, RPC schema consumers, JSON serialization and snapshot
   clients before removing obsolete fields. Avoid leaving unused bar helpers or
   tests that enforce fake counts. Do not broadly delete functioning transport.
6. Reconcile RPC-047 and BUG-210 feature text/coverage with the new honest UI;
   specifications first, failing tests second, production implementation last.

## Proposed acceptance scenarios / regression plan

- A real automatic builder is held pending with 153 active messages: the original
  session shows indeterminate compaction activity, never `0/153`, a percentage or
  an empty determinate bar implying measured progress.
- Pre-prompt, hook/error recovery, manual compaction and GenerateCompaction use
  the same explicit activity semantics; unknown units never become `0/1`.
- Test the actual producer, not a test-only call supplying advancing fractions.
  Use a deterministic local builder/provider and shared temporary-directory
  fixtures; no live provider is needed.
- Through both embedded and WebSocket transports, switching session focus never
  transfers activity to another session. Completion, failure and cancellation
  clear only the operation's session activity; refocusing cannot resurrect it.
- Successful durable commit reports measured/estimated token basis and true
  available final coverage separately; a mere activity clear is not success.
- Preserve footer workspace/branch and existing supervisor notification priority.

Suggested validation targets when implementing: package `codelet-fspec-tui`,
integration targets `automatic_compaction_progress_bug210`,
`slash_compact_rpc047`, `cmpct068_committed_projection`; add producer-level
coverage in `codelet-cli`/`codelet-agent-loop` and update feature sidecars.
Scope Cargo commands by package and target; do not run workspace-wide tests.

## Implementation and ACDD verification

The fix retains the existing `phase/current/total` wire and snapshot fields.
Unknown work is published as `0/0`; the footer renders the phase alone, including
when receiving legacy nonzero counts. This removes the fabricated fraction and
determinate bar without removing the working activity transport or changing
authoritative completion statistics.

Four focused features cover actual builder activity, footer compatibility,
entry routes and generated-compaction targets. Each feature maps to one
integration test file. The historical automatic-progress, slash-command and
recovery specifications were reconciled as well. Specifications and failing
tests preceded production changes; the card entered implementing only after
the red gates and feature/test split were complete.

Scoped green verification covers 20 TUI tests, five recovery-helper scenarios,
the entry-route outline and the generated-target outline. Property tests cover
arbitrary legacy count combinations; the footer snapshot preserves workspace
chrome and supervisor priority. Library checks and targeted Clippy completed;
the existing token-state `collapsible_match` warning remains unrelated. Coverage
audits validate all seven affected features and 23 scenarios. Global tag
validation still reports pre-existing registry debt; it is not a clean global
validation result. No live-provider reproduction is claimed.
