# Core structured identity repair — 2026-10-18

Owner `1cd6ff6c`, coordinator `ad8aee9e`. Core-only production/test edits;
no board, staging, commits or changes to CLI-owned tests.

## Actual behavioral RED

Specification `durable-structured-tool-identity.feature` preceded
`rust/core/tests/cmpct059_blob_identity.rs`, then this command:

```
cargo test --profile ci-test -p codelet-core --test cmpct059_blob_identity -j2
```

`/tmp/cmpct-core-current-blob-red.log` runs the test and reports 0 passed,
1 failed. Its assertion observes explicit nested `call_id` as `Null` instead
of `call-exact` after committed replay. It is not a compilation failure.
The real filesystem fixture persists distinct item/invocation IDs on both
tool call/result suffixes, real blob input/output, opaque content metadata,
and an admitted/promoted generation, then resets stores before replay.

## Minimal root repair

`persistence/blob_processing.rs::rehydrate_envelope_blobs` still validates
against the existing envelope schema, but now resolves only recognized blob
payloads in the original JSON value. It no longer serializes through typed
content variants that discard nested provider fields. Tool input replacement
replaces its entire blob marker; result, thinking, base64 image/document data
retain existing rehydration behavior. The source shrinks from 250 to 210 lines.
No schema expansion, immutable-original writes or CLI fallback is introduced.

The same scoped command finishes in `/tmp/cmpct-core-current-blob-green.log`
with the actual regression passing: 1 passed, 0 failed, `CARGO_EXIT=0`.
The test retains distinct `item-exact`/`call-exact` and asserts both restored
payloads, nested opaque metadata and immutable originals.
This is core cold-replay GREEN, not CLI acceptance or runtime-gate success.
CLI owner may rerun its authentic distinct-item/call regression after this stable
source checkpoint; do not weaken it to item-ID equality.

The strict audit `/tmp/cmpct-core-current-strict.json` includes this feature's
one complete real test function, exact scenario-local steps and both blob
rehydration/committed replay implementation mappings. Installed scoped syntax,
format, `link-coverage`, file audit and registered-tag checks pass in the
disposable copied project. The broader core/tools runtime/check/Clippy batch is
already queued; its current results belong in
`/tmp/cmpct-core-current-status.log`, not inferred from earlier runs.

Core059/core065 handed-off documentation targets remain untouched. The new
feature/sidecar and blob regression are exclusively core-owned.
