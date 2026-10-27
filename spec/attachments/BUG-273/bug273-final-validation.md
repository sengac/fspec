# BUG-273 final implementation and validation

Final validation passed. CLI status transition and verification are recorded separately in the work-unit history.

Codex managed this work through `fspec bootstrap` and standalone CLI commands for specification, status, coverage and attachments. No native agent/MCP orchestration was used. Bootstrap's unsupported `--sync-version 0.9.3` option is documented; actual CLI workflow gates were followed and their reported coverage issues corrected.

## Implemented behavior

1. A small optional `messages.verified` completeness cache binds strict full-scan results to exact data/index file generations. Current peer/cold-reopen appends reuse verified completeness instead of recovering from byte zero for each fresh identity. Missing, corrupt, replaced or unsuccessfully published caches retain conservative recovery.
2. Readers accept the byte-identical V1 and V2 offset layouts; publications use the legacy-readable V1 header. Message framing, UUID and content-hash checks remain in the current scanner/reader. The historical permissive JSONL scanner is fixture-only.
3. Native SessionSearch handler I/O and serialization run on Tokio's existing blocking pool. A denied pre-tool hook never dispatches storage work.

Same-version peer publications changed from `[2,2,2,2]` to `[1,1,1,1]`. Alternating current/historical readers changed from four rebuilds per codec to zero. Verified missing-ID lookups no longer republish merely because a current peer advanced the generation. A legacy append explicitly invalidates completeness and requires conservative recovery before fresh admission.

## Red/green and final checks

Pre-fix failures reproduced all three diagnosed mechanisms. Original diagnostic sources, specifications and red logs are preserved in this task's `validation/bug273-pre-fix-diagnostics/` and `validation/bug273-*-red.*` / `indexing-diagnostics-run-red.*`.

**82 distinct passing tests across 16 targets.** Nested subprocess invocations are not counted again.

| Target | Tests passed |
|---|---:|
| `bug247_message_durability` | 5 |
| `bug247_stable_message_identity` | 4 |
| `bug248_reference_evidence` | 2 |
| `bug248_tail_recovery` | 7 |
| `bug273_shared_index` | 9 |
| `concurrent_message_index_persistence` | 1 |
| `indexing_diagnostics` | 14 |
| `message_index_recovery` | 4 |
| `message_store_lifted_test` | 4 |
| `shared_message_store_cleanup_safety` | 3 |
| `shared_message_store_index_coherence` | 5 |
| `shared_message_store_live_reads` | 3 |
| `bug273_search_responsiveness` | 2 |
| `pre_tool_hook_native_test` | 14 |
| `prov149_codex_history` | 2 |
| `prov149_codex_reload` | 3 |

Required successful commands (offline, one job, isolated fixture HOME/TMPDIR/storage, six-GiB free-space guard, debug symbols disabled):

- `cargo test --offline -p codelet-core --test indexing_diagnostics --test concurrent_message_index_persistence --test message_store_lifted_test --test message_index_recovery --test shared_message_store_cleanup_safety --test shared_message_store_live_reads --test shared_message_store_index_coherence -j 1 -- --nocapture --test-threads=1`
- `cargo test --offline -p codelet-tools --test bug273_search_responsiveness --test pre_tool_hook_native_test -j 1 -- --nocapture --test-threads=1`
- `cargo test --offline -p codelet-providers --test prov149_codex_history -j 1 -- --nocapture --test-threads=1`
- `cargo test --offline -p codelet-agent-loop --test prov149_codex_reload -j 1 -- --nocapture --test-threads=1`
- `cargo test --offline -p codelet-core --test bug273_shared_index -j 1 -- --nocapture --test-threads=1`
- `cargo check --offline -p codelet-core -p codelet-tools -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j 1`
- `cargo clippy --offline -p codelet-core -p codelet-tools -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib --no-deps -j 1`

The configured history/reload targets were run with the existing no-debug test profile instead of creating a second ci-test cache. All selected consumer libraries compile and pass scoped Clippy. Scoped rustfmt, `git diff --check`, syntax/tag validation for all three scoped features and both acceptance coverage audits pass. Per-command receipts and full logs remain under this task's `validation/bug273-*` paths. Clippy warnings, if any, remain verbatim in the log; there are no failed required checks.

The initial broad run had one layout assertion expecting two files. Its corrected control asserts exactly `messages.idx`, `messages.jsonl`, `messages.verified`, including rejection of leaked staging files, and passed. The Unicode-tail property controls passed every exercised byte boundary. Independent stores and real child processes passed coherence and exactly-once/identity safeguards. Cache write/sync/rename failure controls preserve recoverable committed messages.

## Review and preservation

The completeness cache is derived validation state, not content or identity authority. It is published only after strict scanning, or carried forward by verified append/cleanup under the existing stable sidecar lease. Exact data/index generation mismatch invalidates it. Omitted originals, incorrect offsets, conflicting identities, corrupt hashes, durable admission, metadata updates, compaction and cleanup/reference evidence controls all pass.

All eight changed production files remain under 300 lines. Their exact SHA-256 values and the unchanged verbatim historical codec checksum are recorded in `bug273-final-results.json`. All 47 unrelated baseline source/spec files remain byte-identical; the board file changed only via CLI. Earlier unpublished BUG-271/272 work was preserved.

Cargo cleanup removed 12,375 generated files, reporting 8.1 GiB; measured free-space gain 8,561,905,664 bytes. Sources/evidence/installed tools were preserved. The first cleanup was denied before execution; a single retry succeeded after the current explicit rule was verified. A later rebuild failed because its target directory disappeared; attribution is unknown and the failure log is preserved. The subsequent single rebuild passed.

## Limits and deployment

No full-workspace test run, Windows execution or live-store benchmark was performed. The first admission into an unverified history still requires one strict full scan; a missing/stale proof conservatively repeats that recovery. The existing global storage lease, synchronous core APIs and whole-index publication per append remain. This fixes redundant whole-history rescans and native SessionSearch worker starvation; it does not claim to remove all storage latency or every synchronous caller. Concurrent mutation correctness is tested among current cooperating writers; old binaries that ignore the lease cannot be made safe merely by another reader's format compatibility. Repository-wide tag debt is outside this card; scoped tags pass.

Changes remain uncommitted and are not deployed. No daemon restart, live-store read/reindex/migration, BPD operation, credential change, commit or push occurred. User data and native Mac Valheim were untouched.
