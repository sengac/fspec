# REFAC-019 native mapping receipt: rpc onward

## Scope and status

Owned feature basenames lexicographically >= `rpc`, excluding parent-owned
`rpc018-source-shape` and `rpc027-help-disconnect-thinking-dialogs`.
This is a partial-completion receipt, not canonical Done or a passing build.

Current readback covers 40 feature sidecars and 257 scenarios. Comparing each
scenario against git HEAD plus the exact approved physical-source relocations:

- 37 feature sidecars match their complete expected scenario contents.
- 254 scenarios match exactly; three scenarios each retain one original row.
- All 276 original test-mapping rows remain represented, with multiplicity.
- 152 of 155 affected test rows match their complete expected replacements.
- 56 of 57 moved test-mapping rows have reached their destination.
- 127 of 130 original invalid implementation entries are relocated.
- Original invalid-entry scope: RPC 32, transport 89, Action 9.
- Remaining entries: RPC 1, transport 1, Action 1.
- Implementation mapping count is 389 (original 339; fully relocated target 394).
- All unrelated rows/implementation mappings match the original comparison.

The 155 affected rows originally carried 205 implementation mappings; complete
relocation would produce 260 mappings on those rows. Nonaffected mappings remain
unchanged. These counts include the preceding successful native operations, not
just calls made after this continuation.

## Verification

Read-only command:

`python /tmp/refac019-rpc-writer-prepare.py verify`

It verifies all Action/RPC/transport plan source fingerprints, checks each moved
test's function declaration at its recorded physical lines, constructs expected
relocations from original HEAD sidecars, and compares complete scenario row
multisets including every implementation file and line array. An independent
Counter-based readback found exactly the three pending old/new row differences
listed below, with no other missing or additional rows.

All 22 hashes in `/tmp/board024-controller-baseline/protected.json` match.
The 20 owned RPC production sources remain below 300 lines (maximum 265).
No production/test source, frozen snapshot, protected file, build state, or
work-unit status was changed during this mapping continuation. No builds,
commits, installs, or restarts were run. All coverage mutations used native
`functions.fspec` link/unlink commands, never direct sidecar writes.

## Blocker 1: RPC026 fixture contradicts the scenario

Feature: `rpc026-cross-transport-parity`

Scenario: `persistence_delete_session round-trips identically across both transports`

Unchanged original test row:
`rust/fspec-tui/tests/rpc026_cross_transport_parity.rs:1-300`

Unchanged implementation mapping: `rust/rpc/src/lib.rs:1-700`.

Required provenance union:

- `rust/rpc/src/lib.rs:1-62`
- `rust/rpc/src/service_declaration/finish.rs:3-8`
- `rust/rpc/src/service_declaration/workspace.rs:5-171`
- `rust/rpc/src/service_declaration/providers_sessions.rs:5-224`
- `rust/rpc/src/service_declaration/supervision_credentials.rs:5-247`

The first link was rejected for missing the scenario's second independent
SharedFspecService setup. No replacement was added and no old mapping removed.
Source investigation confirms this is substantive, not a missing cosmetic
comment: feature line 40 requires another independently seeded service; the
existing test shares one service and deletes s-2 through embedded, then deletes
s-1 through WebSocket while the scenario requests s-2 on each isolated service.
The original test discards both list results instead of comparing them. Do not
add a false setup comment, bypass validation, or retry unchanged. Parent owns
any separately authorized test/spec correction.

## Blocker 2: historical RPC050 detach assertion disagreement

Feature: `slash-command-detach-source-shape`

Scenario: `components::Action declares the new RPC-050 variants`

Unchanged test row: `rust/fspec-tui/tests/source_shape_rpc050.rs:139-162`.
Unchanged implementation mapping: `rust/fspec-tui/src/components/mod.rs:414-432`.
Required replacement:
`rust/fspec-tui/src/components/action_agent_navigation.rs:106-124`.

The earlier native rejection remains unresolved; it was not retried unchanged.
Feature line 54 requires WorkUnitDetached to exist, whereas the existing test
lines 162-166 explicitly assert its absence after BUG-205 removed /detach.
The positive attach assertions remain intact. No false step comment, feature
rewrite, validation bypass, or old mapping removal was performed.

## Blocker 3: native duplicate-row implementation selector

Feature:
`widen-sessionmanagerhandle-fspecservice-both-backends-stub-with-cross-transport-parity-tests`

Scenario: `every new method has a tarpc service peer and a FspecBackend trait peer`

The first original row has been fully relocated to:
`rust/fspec-tui/tests/rpc037_cross_transport_parity/source_contracts.rs`
with test lines `73-86,88-111,113-132,134-154,156-176,178-190` and the union:

- `rust/fspec-tui/src/transport/backend/models.rs:5-96`
- `rust/fspec-tui/src/transport/backend/workspace.rs:8-72`

The second original row remains intact:
`rust/fspec-tui/tests/rpc037_cross_transport_parity.rs:1111-1238`
with implementation `rust/fspec-tui/src/transport/mod.rs:232-389`.
Its required destination and two-file union are identical to the first row.
Both rows must remain as distinct original lineages; do not merge/deduplicate.

The supported native link operation appends a row carrying only one mapping
when testLines is supplied. Subsequent implementation-only links select the
first row by testFile, not testLines or row identity:
`rust/fspec-core/src/commands/link_coverage.rs:459` and `:520`.
Therefore it cannot complete the second two-file union without updating the
first row instead. This is a demonstrated API-selector limitation, not a
rejected tool request. The second old row was deliberately not removed.
No direct sidecar write, fabricated path alias, duplicate flattening, or
temporary loss of source provenance was used to force completion.

All other RPC037 scenarios now match their complete expected row multisets,
including distinct duplicate implementation ranges and multi-file unions.

## Delegated 9c receipt incorporation

Incorporated final delegated receipt:
`/tmp/refac019-native-mapping-rpc022-through-rpc061-receipt.md`.
SHA256: `7de6962dcc4ea82b0946525a45229a84be494fa37a6ace7e0e8cf6150437d150`.

The delegate's affected-feature inventory is a subset of this receipt, not an
additional disjoint region. Its 16 feature documents, 108 scenarios, 108 test
rows, and 145 implementation entries were independently reread and match its
reported totals. These are the affected documents in its interval, not all
repository features whose names fall in that lexical interval.

Delegated results already included in the aggregate counts above:

- 79 audited original implementation entries; 78 resolved to 90 physical
  replacement mappings, with RPC026 audit index 81 retained as blocked.
- 54 unrelated implementation entries preserved, with multiplicity.
- 1,426 original line-reference occurrences became 1,416 physical references;
  the ten removed blank-reference occurrences were explicitly accounted for.
- 11 moved-test definitions across five files correspond to ten named scenario
  rows; the unassociated RPC054 wire-types definition received no invented row.
- Native coverage readbacks and scoped whitespace validation are delegated
  evidence, not new test/build executions or newly repeated mutations here.

The delegated RPC026 rejection is the same Blocker 1 described above; it must
not be counted a second time. Inherited successful relocations were not replayed.

## Final read-only readback and tooling decision

After the supervisor's instruction to stop all metadata/source changes, the
read-only resolver comparison and independent row-multiset comparison were
repeated. Final actual counts are unchanged: 40 affected feature documents,
257 scenarios, 276 test rows, and 389 implementation entries; 37 documents and
254 scenarios match fully. Exactly three expected rows are missing and exactly
their three preserved original rows remain instead. Fully relocated target:
394 implementation entries. All 22 protected hashes still match.

The user selected **Prepare Tooling Proposal**, not a validation bypass or Done
claim. No further coverage metadata, source, production, status, daemon, or
tooling changes were made. This receipt update is documentation only. Parent
owns attachment of the tooling proposal and validation evidence.

No legitimate alternative to the duplicate selector limitation was proved.
For the exact RPC037 peer scenario in Blocker 3, row 2 still needs the same
two-file models/workspace union as row 1. Current native implementation-only
linking selects row 1, while combined linking appends only one implementation.
No attempted workaround, forged path alias, duplicate manipulation, deduplication,
or direct sidecar change was used. The original row 2 remains intact.

This receipt does not claim to resolve the parent's separately identified
global generator limitation, activate proposed tooling, close the mappings,
or establish canonical REFAC-019/BOARD-024 Done. Existing statuses are preserved.
