# CMPCT-077 AST research

2026-10-17, before test and production changes.

AstGrep Rust pattern `impl NativeCapabilities { $$$BODY }` identifies
`rust/providers/src/codex/native.rs:11`. Its `apply` method appends a
`compaction_trigger` when the public `compaction` capability is enabled.
`model.rs` exposes this through `with_native_capabilities` and calls it after
history preparation. Hosted search shares this type but has an independent tool
contract and must remain available.

AstGrep Rust pattern
`pub fn commit($$$ARGS) -> Result<(), CompletionError> { $$$BODY }` identifies
`rust/providers/src/codex/history.rs:141`. This method currently accepts negotiated
compaction output, clears the replay prefix, and saves native compacted users.
Remove that alternate commit algorithm. Reject unsupported input/output before
mutating checkpoint state, including restored legacy native state.

Streaming validates native output in `NativeOutput` before checkpoint commit, but
direct completion bypasses that validation. Both boundaries need the same typed
native-memory rejection policy. `CodexItem::Compaction` is only needed to recognize
persisted incompatible checkpoints; it must never authorize continuation or commit.

Existing PROV-158 tests assert the superseded opt-in behavior. Replace those owning
slice assertions with explicit rejection, retaining hosted-search and reasoning
regressions; preserve historical feature files and work-unit records unchanged.
