# Sessions replay finish coordination

The scoped finish audit read BUG-249/250/251/252 repair plans, existing RED/GREEN
evidence, features, shared replay decoder and session transcript paging code.
No production changes have been made during this audit.

The current archival source (`core/src/persistence/archive.rs`) enumerates only
`manifest.messages`. Its record kinds contain no journal record, and
`sessions/src/archival_replay.rs` consumes only that archival source. Therefore
the existing `_transcript` display-only hook is not enough to restore terminal
notices or unpromoted crash prefixes through cold SessionManager restoration.
The existing `bug250_cold_terminal_display` target is queued in the scoped finish
run to verify this integration behaviorally; the source audit is not new RED.

BUG-245 worker `c38f004d-db70-4e9f-b035-906879bae41b` owns archival replay/media.
Parent `bb177085-4cc6-46e3-9c61-f3f68b435749` owns core archival integration.
Neither boundary is patched independently by this audit. Parent should connect
`JournalStore::project(original_message_ids)` to the public archival source and
preserve causal ordering plus journal UUID/timestamp metadata. Do not inject
terminal prose into model envelopes or sort causal events by timestamps.

`replay_transcript`, `transcript_archive` and `transcript_journal` were inspected:
the decoder remains display-only; archive entries retain original metadata;
paging enforces generation/snapshot, UTF-8 offsets and encoded byte budgets.
No independent defect in these owned helpers was established by this read.

The current test command uses the existing release-slim profile and two jobs:

```sh
cargo test -p codelet-sessions --test bug251_multimodal_archives \
  --test bug252_discovery_classification --test bug218_optional_model_discovery \
  --test prov131_local_profile_probe_resilience --test bug250_transcript_replay \
  --test bug250_cold_terminal_display --profile release-slim -j 2 \
  --no-fail-fast -- --nocapture
```

Output: `/tmp/bug251-252-scoped-final-green.log`. Pending tests are not claimed
passing. BUG-249/250 status decisions remain with the parent. Initial messages
to parent and BUG-245 worker were rejected as busy, not delivered or queued.

The first queued invocation was interrupted before compilation to add
`--no-fail-fast`, so a cold-terminal failure cannot prevent the independent
BUG-251/252 targets from executing. This interruption is not behavioral RED.

Parent subsequently approved and assigned the actual production repair to this
owner. The common core archival source now includes pinned journal projection,
which automatically reaches the existing cold-display iterator without editing
BUG-245's archival replay/media files. See `journal-archive-repair.md` for the
implemented contract, executed RED provenance, tests and current validation.
