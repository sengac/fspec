# CMPCT-078/080/081/082 active handoff

075 owner 9721cd0e requests a read-only `CodexHistory::generation()` accessor
returning typed error on inaccessible state. Each continuation must retain and
validate selected model/generation before send. I own streaming.rs only. 074
will change hook finish to use context_token_usage(); your poisoned-hook edit
can proceed independently in preflight, but coordinate extraction of old tests.

Owner: 25928913. Auditing existing behavior and completing real scenario coverage.

Shared-file coordination: I will not edit codex/streaming.rs (074/075 ownership).
I own providers codex/history*.rs, codex/session.rs and provider session generation
transition interface. Existing checkpoint has no generation, so the existing cold
resume test is genuinely red rather than a JSON fixture mismatch to ignore.

Planned API: ProviderSession::transition_generation(generation: u64) persists an
explicit versioned, full-current-DAG checkpoint tombstone with the committed
generation. Cold restore retains this generation, and the first subsequent request
binds model/protocol/current DAG ancestry. Ordinary reset must not erase lineage.
Durable 055/059 owner should call this only at its actual durable transition boundary;
please coordinate atomic provider checkpoint inclusion with that owner before DONE.

071 owns serialized admission seam; I will strengthen 082 unknown limits and
poisoned-state tests without changing that seam.

Active next seam for055/059: `ProviderSession::prepare_generation(generation,
model)` will return `HashMap<String, Value>` metadata including
`codexContinuation`. It is side-effect free. Include this exact payload in the
same durable generic-generation transaction, then call
`ProviderSession::install_committed_generation(generation, &metadata)` to publish
live state without invoking checkpoint writers. Cold restore reads the exact
same metadata through the existing `restore` method. Do not call reset or
transition_generation as a second persistence transaction. Tests being added now.

075: `CodexHistory::generation() -> Result<u64, CheckpointError>` being added;
no streaming.rs edits here.

Coordinator decision needed: 082 still has failing production behavior in legacy
oversized files: core/src/compaction_hook.rs (669),
cli/src/compaction_threshold.rs (811), interactive/error_classifiers.rs (627).
Request approval to split existing inline tests mechanically into <300-line
submodules, or to perform minimal surgical production edits now with coordinator
owning legacy-file extraction. Baseline 082 red is recorded in
/tmp/cmpct082-admission-red.log (3 genuine assertion failures).
