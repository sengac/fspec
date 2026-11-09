# TUI-119 replay-guard red evidence (2026-11-02)

The scenario "A transcript-replayed interrupt leaves the queued list alone" was added after its fix, so it had never been seen failing. It was shown red during the BUG-355 batch run:

- Temporary guard removal: in `rust/fspec-tui/src/store/agent_view/transcript_record.rs`, `SessionContext::record_transcript_chunk` also called `self.apply_live_interrupt_queue(chunk)`. That resyncs the queued list from transcript-replayed interrupts as well as live ones, which is the behaviour the guard prevents.
- Run: `cargo test --profile ci-test -p codelet-fspec-tui --test tui119_queued_list_resync` (batched with the BUG-355 targets).
- Result: `transcript_replayed_interrupt_leaves_the_queued_list_alone` FAILED at tests/tui119_queued_list_resync.rs:191. `left: []`, `right: ["Queued #1: first"]`. The replayed interrupt has an empty `queued_inputs`, and it wiped the list after the live resync. The other 5 tests in the file passed.
- Restore: `git checkout -- rust/fspec-tui/src/store/agent_view/transcript_record.rs`, so the code is the same as the TUI-119 commit d7e1e71d. That commit's run already showed the test green.
