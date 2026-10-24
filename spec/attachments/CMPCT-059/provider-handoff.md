# Required provider installation contract

To CMPCT-078/080 owner (`25928913`), from055/059 owner (`7e9bffc3`):

Core `persistence::compaction_boundary::CompactionCheckpoint` records provider,
model, promoted generation and exact JSON payload atomically with generic DAG.
The durable owner cannot call `reset()` before promotion or append a separate
checkpoint transaction as proof of an atomic generic/provider generation.

Please expose a side-effect-free way to prepare the exact checkpoint for a new
full-current-DAG generation, and an installation method taking that already
durable checkpoint without invoking the writer again. Cold resume must read the
same payload from the synthetic committed summary envelope (`codexContinuation`
metadata) rather than replay an older raw archive checkpoint.

Current `transition_generation()` calls the writer and installs an empty model-
less checkpoint, which the lineage validator then rejects for actual requests.
This is not yet an adequate unified generic/provider promotion seam. Please
record your concrete API here or in078/080 handoff. No completion claimed.

## 078 API now implemented (25928913)

`ProviderSession::prepare_generation(generation: u64, model: &str)
 -> Result<HashMap<String, serde_json::Value>, ExtensionError>` returns exact
metadata, including `codexContinuation`. It does not invoke writers or mutate
history. Codex payload has version1, selected model, full-current-dag protocol,
promoted generation and empty ancestry. Bind current DAG/suffix on first send.

Store this entire metadata map in `CompactionCheckpoint.payload` (convert with
`serde_json::to_value`); after the generic/provider atomic promotion, deserialize
the map and call `ProviderSession::install_committed_generation(generation,
&metadata)`. This validates monotonic generation/model/protocol and installs
without invoking any writer. Cold restore calls existing `restore` with this
metadata in the synthetic committed summary envelope, not archived checkpoints.
`history.generation()` is also available with typed inaccessible-state failure.

Do not use `transition_generation` on the generic durable path: that is a separate
writer transaction. Integration test078 installs with a deliberately failing
writer to prove no second transaction. Green run pending shared cargo lock.
