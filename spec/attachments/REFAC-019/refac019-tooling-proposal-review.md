# REFAC-019 tooling proposal — read-only review

Reviewed /tmp/refac019-scoped-coverage-tooling-proposal.md only, against the
native constraints already established during mapping work. No further source
research, source/metadata changes, tool activation or builds performed.

## Verdict

Sound proposal-only boundary: no aliases, sidecar imports, skipped validation,
status weakening, replacement shell executable or unsupported live activation.
The same-name BUG181 outline is accurately distinguished from global generation.
Parent reports its post-comment filtered pair GREEN at
/tmp/refac019-bug181-outline-green.log (2 pass, 0 fail); this is supplied parent
evidence, not a test run by this reviewer.

## Clarifications before implementation approval

1. **Shared mutation locking must cover every writer.** Explicitly require
   selected and legacy/append link modes, unlink modes, and generation to use
   the same per-sidecar read/check/write lock. An indexed mutation's lock/hash
   cannot prevent lost updates from an unlocked legacy or generator writer.
   Do not assume the existing atomic JSON replacement is already a transaction
   lock; introducing/reusing a suitable lock needs an explicit implementation
   and concurrency-test scope. Add a stale-read competing-writer test.
2. **Selected generation must fail closed on invalid existing JSON.** The
   proposed recreation delta needs qualification: an unreadable/invalid sidecar
   has unknown lineage and cannot be safely auto-recreated under a preservation
   guarantee. Report a blocked preview and make no write unless a separate
   explicit repair contract is approved. Add a corrupt-sidecar preservation test.
3. **Add concrete evidence receipt references.** The mechanism descriptions are
   accurate, but bare link_coverage.rs/generate_coverage.rs names do not identify
   the empirical native failures or captured unrelated mapped-lineage loss.
   Link the parent's existing duplicate-union native receipt and generation-loss
   evidence by their actual paths, plus the settled audit when available. The
   existing /tmp/refac019-bug181-lineage-correction.md is sufficient evidence
   for that case's original global-scope blocker and supported outline repair.
   Do not label a historical transient audit as a current settled readback.

## Constraint checks

- The duplicate-union limitation is real, not merely an inconvenient index:
  append creates a last row with at most one impl, whereas impl-only mutation
  targets the first same-file row. Rotating originals cannot leave two retained
  same-file rows each with its required multi-file union without a precise
  selector or another separately approved native capability. Preserving the
  blocked original is correct; do not deduplicate or merge occurrences.
- Selected generation and truthful update/removal previews are justified. The
  current global dry run does not establish that a real run is lossless.
  Rejecting mapped-row removal, with no silent global fallback, is appropriate.
- Selector/hash checks do not replace normal link file/step validation. Keep
  legacy unlink semantics explicit; this proposal must not invent a validation
  bypass or demand false comments to delete obsolete coverage.
- Repository implementation/tests do not activate the running native tool.
  Activation remains independently unauthorized and unsupported under the
  present no-install/no-restart constraint. Canonical Done is future conditional
  closure, not authorized by preparing or accepting this proposal.
- The accepted safe-extra receipt remains
  /tmp/refac019-native-additional-before-rpc-receipt.md. Its held row 16 is a
  separate semantic discrepancy, not resolved by a selector or scoped generator.

No objection to attaching this as a pending tooling proposal after the three
clarifications; no claim that it unblocks the running daemon or completes either
card today.
