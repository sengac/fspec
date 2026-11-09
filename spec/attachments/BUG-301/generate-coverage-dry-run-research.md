# Research: global `generate-coverage --dry-run` under-reports what a real run changes

Observed 2026-10-30 in this repository while preparing REFAC-020.

## Reproduction
```
$ fspec generate-coverage --dry-run
Would create 4 coverage files (DRY RUN)
  - client-mcp-full-rpc.feature.coverage
  - client-mcp-read-only.feature.coverage
  - indexing-lock-diagnostics.feature.coverage
  - precise-lossless-coverage-mutations.feature.coverage
Would skip 2174 existing files
```
A real `fspec generate-coverage` then **also rewrote 5 existing sidecars** that the preview
reported as "skip":
`agent-loop-interrupt-cascade`, `board-card-details-additive-api-contract`,
`board-card-details-canonical-backend`, `structured-generation-aware-session-replay`,
`typed-durable-summary-identity` (`.feature.coverage`). These had to be reverted with git to
avoid unrelated lineage changes.

## Cause
`rust/fspec-core/src/commands/generate_coverage.rs` dry-run mirrors the TS reference, which
only distinguishes create / recreate / skip and never computes updates ("TS dry-run does NOT
detect updates"). The global update path also drops unknown top-level fields
(TS `{ scenarios, stats }` parity) and removes stale scenarios without checking whether they
carry mapped lineage — exactly the hazards REFAC-020's selected mode (`--feature-name`)
blocks.

## Suggested fix
Make global dry-run report the same plan the real run executes (created / updated with
added+removed scenario names / recreated / skipped), reusing REFAC-020's
`generate_coverage::plan` so preview and apply cannot diverge; and block (or require an
explicit flag for) removal of scenarios that still have test mappings, like selected mode.
Keep the TS-parity text format behind the existing output if byte parity is still required.
