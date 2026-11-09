# BUG-284 isolated real-history replay

This uses preserved real failure data locally. It does not contact a provider,
execute tools, or repair a live session. Keep raw evidence private; share only
the analysis, scripts, hashes and bounded results.

## Evidence

- `/tmp/compaction-investigation/private/`: latest two failed manifests,
  checkpoints, ordered original records, checked blobs and derived wire inputs.
- `/tmp/compaction-investigation/earlier-private/`: earlier two Linux failures.
- Preservation manifests identify original source paths, copied artifacts and
  SHA256 hashes. `evidence-verification.json` confirms the latest eight source
  artifacts and their copies remained identical. There are 44 checked blobs
  for those two sessions.
- `timeline/` contains filtered failure/cancellation metadata and receipt hashes.
- `replay-source.sha256` identifies the harness, dependency libraries and relevant
  source files. The replay provider library is a retained ci-test artifact; its
  identity is distinct from the separately examined live release executable.

## Run the final replay

From `/home/rquast/projects/fspec`:

```bash
rustc +1.95.0 --edition=2021 /tmp/compaction-investigation/replay_production.rs \
  -o /tmp/compaction-investigation/replay-production -C debuginfo=0 \
  -L dependency=rust/target/ci-test/deps \
  --extern codelet_providers=rust/target/ci-test/deps/libcodelet_providers-24f47bf118f70df8.rlib \
  --extern serde_json=rust/target/ci-test/deps/libserde_json-18d8e11bc1cb5ec6.rlib
/tmp/compaction-investigation/replay-production
```

The harness rejects all four unchanged histories with the exact lineage error.
It then checks explicitly counted local counterfactual differences and strict
tool-tamper refusals, returning nonzero on an unexpected result. The final
`production-replay.log` records four successful assertions and exit zero.

## Conversion and limits

`replay_lineage.py` restores original blob markers with hash validation and
reconstructs the observed text/tool-only wire subset following the repository's
Anthropic-envelope-to-rig and rig-to-Responses conversion. It also mirrors
projection to report the first mismatch without printing prompt contents.
The actual pass/fail authority is the production Rust `CodexHistory` API, not
the Python comparator. No image/multimodal correctness is inferred.

The selected admission path uses this same lineage preparer, but the harness
does not execute an entire automatic compactor, model call, process restore or
daemon lifecycle. Those remain required validation for any eventual repair.
Diagnostic omission/replacement happens only in an in-memory cloned request;
it is neither a migration nor an acceptable live bypass.
