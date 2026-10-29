# REFAC-020 compiling RED and pending bootstrap decision

This is an evidence checkpoint, not completion or new approval. REFAC-020 remains
testing, REFAC-019 validating, and BOARD-024 testing. The native root session
`2701b174-ea28-4ca6-a8ba-a46034ecb847` is Paused on a human decision and was read
back as `codex/gpt-6.1-sol`. The unchanged daemon is on port 36373.

## Actual causal RED

Five scoped Cargo test targets compiled and executed against unchanged production:

| Target | Passed controls | Causal failures | Other |
| --- | ---: | ---: | --- |
| refac020_cli_contracts | 3 | 1 | Existing main exceeds the approved physical ceiling |
| refac020_cli_coverage | 1 | 6 | Missing selected/scoped CLI capabilities |
| refac020_selected_coverage | 2 | 5 | Incorrect selection, stale/ambiguous safety absent |
| refac020_scoped_generation | 0 | 7 | Scope and lossless planning absent |
| refac020_coverage_transactions | 0 | 3 | Participating operations do not wait for held lock |

Totals: 6 preservation controls passed and 22 meaningful runtime assertions
failed. One ignored transaction entry is a subprocess-only harness invoked by
the acceptance tests, not an ignored acceptance criterion. Evidence logs are
`/tmp/refac020-cli-causal-red.log` and `/tmp/refac020-core-causal-red.log`.
Earlier fixture-compilation failures are excluded from causal RED.

The original compiled Clap tree remains frozen at
`/tmp/refac020-original-clap-tree.json`, SHA256
`80c4b8b7ed267e4c4afec8610fda703a0d965d9a03bf83599c99e15f1297d4b4`.
Independent core and CLI follow-up reviews found their enumerated proof gaps
resolved; they do not claim feature GREEN.

## Exact blocked action and required decision

The normal native `update-work-unit-status REFAC-020 implementing` operation
was rejected because required coverage metadata is missing/incomplete. The core
sidecar is absent; CLI metadata contains only one of eleven scenarios and no
test mappings. The installed supported commands have no selective initializer
for existing feature files. Global generation risks unrelated lineage and is
outside the approved workflow. No unchanged retry, status override or production
implementation occurred after this rejection.

Native interaction `1d42d125-79fc-43b8-84b2-23acfefc248b`, question
`coverage_bootstrap`, is pending. Its original two-sidecar wording is incomplete:
the gate also requires one actual physical test file per feature. The concrete
review-only plan at `/tmp/refac020-feature-layout-proposal/` regroups the existing
28 scenario/test pairs into 19 cohesive feature files without moving or weakening
any test. Every original scenario block is preserved exactly once, byte-for-byte;
both scenario-union hashes equal
`6e187cc35c93557010625b74f3c770433a30e4b6402c6e974ba3f27285218ad1`.

Required decision: permit one bounded preimplementation bootstrap of those 19
exact REFAC-020 sidecars, preserving the existing CLI entry/unknown fields, with
backups, input hashes and exact readback. Then native commands must link the real
tests and pass the unchanged normal workflow gates. No implementation mappings
are invented. Reconcile the original header's post-GREEN bootstrap timing only
under that explicit decision. This is separate from the already authorized
post-GREEN validated local CLI migration; it is not general metadata editing,
global generation, installation, daemon restart or gate bypass permission.

The earlier `/tmp/refac020-bootstrap-proposal/` is a preserved two-sidecar draft
and must not be applied as the final 19-file plan. The migration plan at
`/tmp/refac020-pinned-migration-plan.md` is also pre-bootstrap evidence: its G1/G2
counts and input hashes require explicit reviewed revision after any approved
feature regrouping; they must not be silently refreshed or reused as current.

## Independent progress and coordination

The settled pre-migration audit reports 152/160 strict implementation records,
100/100 moved identities found, 93 fully mapped moved identities, six pending
selector scenarios, and zero provenance/assertion errors. All 22 protected hashes
and six frozen contract snapshots match. Six RPC037 definitions share one
remaining duplicate mapping; one RPC054 definition was inherited unmapped.
Historical unrelated metadata problems are recorded, not silently repaired.

BUG-284 has ownership release for the 13 requested durability/error-propagation
targets. Their original bytes, hashes and passing refactor evidence are preserved
at `/tmp/refac019-pre-bug284-shared-boundary/manifest.json`. No REFAC-020 edit
conflicts. BOARD production must wait for the shared-file handoff. Later BUG-284
signature changes are separate authorized behavior deltas, not replacement
refactor baselines.

No production work, real coverage migration, install, daemon restart, commit,
push, failed-history modification or canonical Done transition is claimed here.
