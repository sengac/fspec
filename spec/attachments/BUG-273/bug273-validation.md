# BUG-273 implementation and validation checkpoint

Status: implementation complete; validation blocked on space cleanup approval. Not Done.

## Workflow

Codex ran `fspec bootstrap`, read the complete 118,763-byte output and managed BUG-273 exclusively through Fspec CLI commands: backlog -> specifying -> testing -> implementing -> validating. The installed binary rejects bootstrap's `--sync-version 0.9.3` flag; ordinary card, scenario, coverage and status commands work. No native agent session or MCP orchestration was used.

The CLI requires one test file per feature, so storage and async-search acceptance criteria have separate features. Coverage mappings were validated through CLI commands; both audits pass. CLI-generated coverage and scoped retag output were installed without bulk changes to unrelated features.

## Change and correctness review

- A small optional completeness cache (`messages.verified`) binds strictly scanned completeness to the exact data and index generations already used by MessageStore, including device/inode and ctime on this Mac. Only a full strict scan establishes completeness. Appends/cleanup carry it forward under the existing shared sidecar lease.
- A missing, malformed, stale, replaced or unsuccessfully published cache takes the existing recovery path. The cache is not message content, credential material or authority to accept an identity conflict. Hash checks, omitted-original recovery, durable message/index barriers and cleanup safeguards remain intact.
- V1 and V2 indexes have the same offset-entry layout. Readers accept both layouts and new indexes use the legacy-readable V1 header. Current code still validates framing, UUID and content hashes when scanning/reading originals; it never imports the historical permissive JSONL validator.
- SessionSearch runs its synchronous handler and serialization on Tokio's existing blocking pool. Pre-tool denial remains before dispatch; no new runtime or blanket approval is introduced.
- All newly changed implementation files remain below 300 lines. New completeness module: 58 lines. Current-worker concurrency is covered by independent stores and actual subprocesses.

## Genuine red and green evidence

Before production changes, BUG-273 tests reproduced peer append index publications=2 (expected1), mixed-version cache replacement, and a single-worker search wait preventing heartbeat progress. Original diagnostic sources and red logs are preserved in `validation/bug273-pre-fix-diagnostics/` and `validation/indexing-diagnostics-run-red.*`.

Current final-source tests: 78 distinct tests across15 targets passed. The separate child-process invocations are controls within a parent test, not additional distinct test cases.

- Core: 60 tests across12 targets. Includes BUG-2738, diagnostic14, message-index recovery4, coherence5, live reads3, cleanup safety3, concurrent index1, lifted store4, stable identities4, durable publication5, committed-reference evidence2 and tail recovery7. Unicode truncation property controls completed in292s.
- Tools: async responsiveness/denial2 plus native pre-tool hook14.
- Configured provider history target:2 passed using synthetic local transport. Proptest printed its existing source-location persistence warning.

The first broad run reported a directory-layout assertion (expected2 files, got3). The updated control asserts the exact sorted names `messages.idx`, `messages.jsonl`, `messages.verified`, rejecting leaked stage files, and its rerun passed. Old diagnostic expected-bad behavior was converted to fixed-behavior regression checks; originals are archived, not erased.

Logs and machine-readable command receipts are under this task's `validation/bug273-*.log` / `.json`.

## Checks and limits

Both acceptance-feature coverage audits pass. All three scoped feature syntax and tag checks pass. Global tag validation has unrelated repository violations; scoped checks use the unchanged registry and actual work-unit data. `git diff --check` passes; scoped rustfmt check is being finalized.

The repository-configured session reload target and six-package library Clippy were stopped by the six-GiB disk guard before completion. They are NOT counted as passes. No whole-workspace suite or live-store performance benchmark was run. Existing synchronous core APIs, global storage lease and whole-index rewrite per append remain; this fixes the redundant whole-history scans and the native SessionSearch async boundary, not every possible blocking caller. Old unmodified writers that ignore the current sidecar lease cannot be made cross-process-safe solely by a new reader; compatible cache reading is tested, while mutation correctness is verified among current cooperating writers. No BPD process or binary was changed.

No daemon restart, live storage access/reindex, deployed binary replacement, commit or push occurred. Tests isolate HOME/TMPDIR/FSPEC storage and use synthetic fixtures only.

## Preservation and current blocker

47 of48 preexisting source/spec file hashes remain identical. The sole changed baseline file is `spec/work-units.json`, updated only through the CLI for BUG-273. The historical V1 codec fixture remains verbatim. Evidence is preserved outside Cargo target.

Disk guard stopped build groups22349 and22500; both are empty and `lsof +D rust/target` found no open files. Cargo metadata verified target=/Users/rquast/projects/fspec/rust/target.

The authorized `cargo clean` attempt was rejected BEFORE execution by automatic approval review, which cited the historical cache-removal refusal rather than the later scoped cleanup authorization. Nothing was deleted. No retry or alternate cleanup path was attempted. Exact blocked action: `cargo clean` with cwd=/Users/rquast/projects/fspec/rust. Fresh approval resolving this review conflict is needed before retry; then resume the two unfinished checks, attach final results and complete CLI Done.
