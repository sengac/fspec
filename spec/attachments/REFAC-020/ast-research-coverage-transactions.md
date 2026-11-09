# REFAC-020 coverage transaction discovery

## Actual AST query

Used the AstGrep tool with language `rust`, path
`rust/fspec-core/src/commands`, pattern `write_json_atomic($PATH, $VALUE)`.
Relevant observed matches before production changes:

- `link_coverage.rs:171`: `write_json_atomic(&coverage_path, &coverage)`.
- `unlink_coverage.rs:271`: `write_json_atomic(&coverage_path, &coverage)`.
- `generate_coverage.rs:198`: `write_json_atomic(coverage_path, &coverage)`.
- `generate_coverage.rs:288`: `write_json_atomic(coverage_path, &output)`.

This syntax query also found work-unit/foundation writers; those are not
coverage transaction participants and are outside this repair.

## Cross-check and architectural decision

Source inspection established that current coverage commands read separately
before atomic replacement. `io/locked_file.rs` locks the temporary replacement
file during writes, not a stable sidecar transaction identity across the whole
read-modify-write. Atomic replacement alone cannot reject stale scenario state.
The bounded shared helper must hold a persistent sibling sidecar lock across
read, hash preconditions, mutation and replacement for every participating path.
Both front doors already dispatch to the existing four core `run` functions.

Additional text/source inspection identified independent sidecar writes in
scenario rename, single/bulk deletion and feature creation; report output can
also target arbitrary paths. Parent explicitly excluded integration of these
writers from this bounded repair. They and the unchanged old daemon remain
nonparticipants; prohibit their concurrent sidecar writes during pinned exclusive
migration. Do not claim repository-wide transactions.

## Approved implementation/test boundary

See `/tmp/refac020-core-plan.md` and its parent-approved precise camelCase API.
The four oversized command roots require honest private full-item modules below
300 lines. Tests use raw JSON with unknown members and duplicate occurrences,
independent canonical hashes and cross-process held-lock coordination. Capture
causal RED before production. Parent owns Cargo, review and real migration.
No production changes, build, install, restart, history/security edit or migration
occurred while preparing this discovery evidence.
