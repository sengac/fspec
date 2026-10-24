# BUG-249 / BUG-250 finish evidence

## Implemented owned boundaries

- JournalStore publishes immutable private deltas under an exclusive writer lease.
  Durable session ordinals in filenames, not timestamps or random UUIDs, order
  records. Reopen checks contiguous session and request sequences and fails
  explicitly on a missing interior acknowledged delta.
- BackgroundOutput admits visible text/thinking and tool invocation records
  through BUG-246's shared bounded recovery owner. Identified assistant/result
  persistence preserves journal message UUIDs, including promotion ambiguity.
- The actual CLI tool consumer invokes prepare_tool_dispatch before polling the
  executor again. ProgressiveOutput and Arc forward this gate. A real sentinel
  regression demonstrated that omitting wrapper forwarding executed the effect.
- A caller-owned failed delta remains retained when shared admission is full.
  Retrying preserves its identity/timestamp and never publishes the rejected
  delta or executes a tool. Storage interruption does not claim a user stop.
- Typed cancellation provenance keeps a running tool outcome unknown when CLI
  interruption supplies a synthetic pairing result. Model originals retain their
  existing construction; only the private journal's lifecycle classification is
  changed. Unstarted calls can be genuinely cancelled before execution.
- Replay::parse decodes `_transcript: JournalRecord` into display chunks only.
  It never returns model messages or model envelopes for these records. The
  image/document decoder branch remains unchanged. Terminal text is selected
  from typed public outcome codes, never raw provider errors.
- JournalStore::export returns the scrubbed display projection with original IDs
  and timestamps. JournalStore::delete removes the private lane durably.

## Executed RED

- `/tmp/bug249-integrity-behavior-red.log`: tied/backward request timestamps
  reordered output, and a deleted interior delta silently shortened output;
  two genuine assertion failures against the actual journal source.
- `/tmp/bug249-executor-final-red.log`: the actual streaming executor wrote the
  sentinel despite the missing durability gate forwarding.
- `/tmp/bug249-ownership-red.log`: full recovery admission incorrectly labelled
  persistence interruption as a user cancellation.
- `/tmp/bug249-interruption-red.log`: an interrupted running tool was archived
  as Completed instead of OutcomeUnknown.
- `/tmp/bug250-replay-behavior-red.log`: the shared replay decoder rejected a
  transcript-only record as a missing dialogue message.
- `/tmp/bug250-lifecycle-real-red.log`: actual delete_session left the private
  transcript directory behind. Export identity/timestamp test passed.
- `/tmp/bug250-cold-display-red.log`: actual cold SessionManager restore retained
  the prefix but omitted its durable safe provider error.

## Executed GREEN

- `/tmp/bug249-integrity-red.log`: the queued actual Cargo integrity target ran
  after the fix: 2 passed. The log name describes enqueue time, not its result.
- `/tmp/bug250-replay-red.log`: actual Cargo replay target: 1 passed after fix.
- `/tmp/bug249-process-direct-green.log`: 2 actual process-kill tests passed;
  both preserve exact acknowledged record UUIDs, timestamps and content. The
  child-only test is intentionally ignored by the outer runner and explicitly
  launched by each parent test. The real executor writes the tool sentinel.
- `/tmp/bug249-executor-direct-green.log`: 1 actual executor gate test passed;
  failed ToolStarted admission returns an error and prevents the sentinel.
- `/tmp/bug249-ownership-direct-green.log`: 1 bounded caller ownership test passed.
- `/tmp/bug250-terminal-direct-green.log`: 2 BackgroundOutput integration tests
  passed, covering model-original separation and failed publication/dispatch.

Direct integration runs compile the tracked integration test against matching
existing release-slim Rust libraries, with Rust thin LTO and opt-level 0 for the
test executable. They are real filesystem/executor/process tests, not source scans
or mocked journal admission. Cargo targets are also explicitly invoked below.

## Shared integrations requiring owner action

The parent received these needs after owned execution; rejected busy-worker
messages are not considered delivered.

1. Core manifest deletion owner: call JournalStore::new(data_root, session_id)
   .delete() from SessionStore::delete_owned under its existing session ownership,
   before manifest removal. Derive data_root from the owning sessions_dir rather
   than a different global store. Propagate failure explicitly. The regression
   remains RED until this canonical hook is installed.
2. BUG-246 owner: deletion must also clear pending recovery spool and memory
   ownership, otherwise a later retry can recreate the deleted transcript.
3. Archive/replay owners: include JournalStore::project(original_message_ids)
   in public archival export and cold archival display iteration, with each
   record's original id/timestamp as TranscriptOriginal metadata. Feed only the
   `_transcript` hook, not model replay. Preserve causal original-message anchors
   and durable journal order; do not sort events by wall-clock timestamps.
   The cold SessionManager regression remains RED until this source is connected.

## Scoped Cargo commands

Run from `rust/`, preserving the shared release-slim profile and `-j2`:

```sh
cargo test --profile release-slim -j2 -p codelet-core --test bug249_journal_integrity --test bug249_transcript_journal --test bug250_transcript_outcomes --test bug250_transcript_lifecycle
cargo test --profile release-slim -j2 -p codelet-agent-loop --test bug249_stream_crash --test bug249_executor_gate --test bug249_admission_ownership --test bug250_terminal_archive
cargo test --profile release-slim -j2 -p codelet-agent-loop --test bug249_interrupted_tool_state
cargo test --profile release-slim -j2 -p codelet-sessions --test bug250_transcript_replay --test bug250_cold_terminal_display
cargo check --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib
cargo clippy --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib --no-deps -- -D warnings
```

No commits or done transitions. Feature syntax and registered tags pass for the
owned feature files; the global tag command reports unrelated violations in 526
other files. Coverage is linked scenario-by-scenario through the serialized board
wrapper. Pending Cargo/check/Clippy results must be read before claiming a final
integrated GREEN; known lifecycle/cold-source RED is not waived by unit GREEN.
