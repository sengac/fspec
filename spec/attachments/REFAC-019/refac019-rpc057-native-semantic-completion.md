# REFAC-019 RPC057 native semantic replacement: completed

## Authorization and verification gate

Supervisor explicitly authorized execution after inspecting parent-owned
existing-requirements verification GREEN in
`/tmp/refac019-final-accuracy-rust195.log` (SHA256
`0c825d1d4e3227da04c486e53895c4adc6f3289501673bc98454752e8ffa3a6c`).
The log records `source_shape_rpc057` 9/9 passing, including
`slash_merge_worktree_wiring_declares_required_app_methods`; it also records
RPC050 3/3 and menu004 32/32. These are parent-run results, not worker builds.

All pinned hashes in `/tmp/refac019-rpc057-test-accuracy-repair.md` were refreshed
and matched before mutation: root, both children, production dispatch, feature,
and pre-replacement sidecar. The entire original raw row was also reread and
matched its archived bytes before any write.

This completes an explicitly authorized **semantic test-association replacement**
after adding the actual missing test. It is not mechanical relocation, unchanged
test identity, or evidence that the earlier rejected root-span request succeeded.
The historical safe-29 receipt remains intact.

## Exact native operations and staged readback

1. Native `link-coverage` appended the new actual test:

```json
{"featureName":"rpc057-merge-worktree-source-shape","scenario":"/merge-worktree slash command wiring lives in dispatch_merge_worktree.rs","testFile":"rust/fspec-tui/tests/source_shape_rpc057/slash_wiring.rs","testLines":"2-77","implFile":"rust/fspec-tui/src/app/dispatch_merge_worktree.rs","implLines":"1-252"}
```

2. Readback proved the scenario held exactly `[original_row, new_row]`, with
   both carrying the identical full 252-integer implementation array. The raw
   original object remained present exactly once, byte-for-byte. New test and
   production source hashes were checked again before removal.
3. Native `unlink-coverage` removed only the original root-file occurrence:

```json
{"featureName":"rpc057-merge-worktree-source-shape","scenario":"/merge-worktree slash command wiring lives in dispatch_merge_worktree.rs","testFile":"rust/fspec-tui/tests/source_shape_rpc057.rs"}
```

Final active association: child `slash_wiring.rs:2-77` with exactly the original
single `dispatch_merge_worktree.rs:1-252` implementation mapping. Scenario active
occurrences followed **1 -> 2 -> 1**; final implementation multiplicity is one,
and no original implementation integer was changed or dropped.

## Raw byte and whole-document comparisons

Affected sidecar:
`spec/features/rpc057-merge-worktree-source-shape.feature.coverage`.

- Before hash: `bfd8a0d632c023e5a71ffe43755a2ffc1d6e5f168d7efbb8eeea162cf4ef1349`.
- After: 23,679 bytes, SHA256
  `cc0411d59adda9a89a519f463531bf73c1251ec20b7e6d0a81f0e330181efe69`.
- Archived original raw object: 5,473 bytes, SHA256
  `da6115d97e3e75d4083064402179cbfe7781c3959be44061e6ae120af4b3cdfe`.
- New raw object: 5,483 bytes, SHA256
  `6f0cda818be94a726cb99e3e3d2c690bb2d4e15f5eb64b9f6b6fd8f84b558570`.

The new raw object is exactly the archived old raw object with only its test
file and test range strings substituted. Its implementation array bytes are
identical. Final JSON contains exactly that one new object and no old object.

Raw reverse substitution of the original row, removal of the newly added
`stats.testFiles` child entry, and reversal of the derived
`stats.totalLinesCovered` value reconstruct the entire before document at its
exact recorded SHA256. Thus every other byte is preserved, not merely a selected
mapping multiset or known schema fields.

Expected derived-stat changes are only:

- Add `rust/fspec-tui/tests/source_shape_rpc057/slash_wiring.rs` to testFiles.
- totalLinesCovered: 1334 -> 1385, reflecting 25 old test lines -> 76 actual test
  lines, a delta of 51. This is a metadata statistic, not new business coverage.

Whole-document scenario count, active test-row count, and implementation-entry
count remain **9 / 9 / 12**. Other scenario contents, mapping arrays and order,
unknown fields, and all other statistics compare exactly.

All 2,182 other `spec/features/*.feature.coverage` sidecars have unchanged raw
hashes. The sorted path-plus-SHA256 inventory aggregate is unchanged:
`e7b76ce0872d015e9397fa1a34854f62f4872783e6e076a3f9e33d2a6ebff4a4`.
Inventory serialization is each relative path, NUL, its lowercase SHA256, LF.

## Archival lineage and preservation

The lossless raw original-row archive and physical mixed-slice archive remain
in `/tmp/refac019-rpc057-test-accuracy-repair.md`. Both compressed payloads were
decoded and hash-verified during this completion. No historical evidence was
rewritten or replaced by a shortened summary.

HEAD root 371-395 still equals current root 173-197 byte-for-byte: 1,496 bytes,
SHA256 `82614aca63a230dd7a4326cd1228315df6cd112fde43e5752cfd528dd8f8c32b`.
The new test's trailing include leaves that original physical slice intact.
All eight original test identities and all 47 assertion-token multisets remain
present; they ran as part of the parent's successful nine-test target. The new
test separately proves the missing source-declaration requirements.

All 22 protected-file hashes still match the controller baseline. No production,
feature criteria, test source, other sidecar, work-unit status, audit script,
daemon, history, install, or commit changes occurred in this metadata execution.

## Supported audit and remaining boundaries

Native `audit-coverage` for `rpc057-merge-worktree-source-shape`, without fix,
returns exitCode 0: all 21/21 referenced file occurrences found. This is file
existence verification; parent GREEN and the independent raw/source comparisons
provide the additional test and preservation evidence.

The six selector-dependent rows were not manipulated. Other semantic/tooling
work remains parent-owned. This completes only the specific authorized RPC057
accuracy repair; it does not claim global mapping closure or canonical
REFAC-019/BOARD-024 Done. No builds were run by this worker.
