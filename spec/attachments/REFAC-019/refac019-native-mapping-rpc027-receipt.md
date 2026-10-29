# REFAC-019 native RPC-027 mapping relocation receipt

## Assignment and boundaries

Fresh continuation parent: `2701b174-ea28-4ca6-a8ba-a46034ecb847`.
Owned feature only: `rpc027-help-disconnect-thinking-dialogs`.
Tool attribution: `REFAC-019` on every call.

All repository metadata mutations used supported native `fspec link-coverage`
and `fspec unlink-coverage` calls, with no validation skips. No source changes,
direct sidecar edits, status resets, commits, builds, tests, installs or restarts.
This receipt is the only file written via apply_patch by this assignment.

## Verified source and provenance

The three remaining moved test ranges were checked against their actual function
names, leading `#[test]` attributes and closing braces in:
`rust/fspec-tui/tests/rpc027_dialog_parity_bcd/thinking_level.rs`.

| Scenario | Old root test range | New physical test range | Preserved/relocated implementation |
| --- | --- | --- | --- |
| Pressing D in ThinkingLevelDialog emits Action::SetThinkingLevelDefault and keeps the dialog open | rpc027_dialog_parity_bcd.rs:331-367 | thinking_level.rs:111-145 | components/thinking_level_dialog.rs:144-152 |
| Pressing uppercase D in ThinkingLevelDialog behaves identically to lowercase d | rpc027_dialog_parity_bcd.rs:368-391 | thinking_level.rs:148-169 | components/thinking_level_dialog.rs:144-152 |
| SetThinkingLevelDefault is wired through the backend trait stack | rpc027_dialog_parity_bcd.rs:392-426 | thinking_level.rs:172-230 | transport/backend/core.rs:109-149 |

Paths in the table are relative to `rust/fspec-tui/tests/` for old tests,
`rust/fspec-tui/tests/rpc027_dialog_parity_bcd/` for new tests, and
`rust/fspec-tui/src/` for implementations.

The D/uppercase original implementation arrays remain exactly
`[144,145,146,147,148,149,150,151,152]`. Their production source is byte-identical
to HEAD.

For wiring, `/tmp/refac019-transport-impl-relocation-plan.json` mapping audit index
82 / feature F20 specifies B:180-220 -> P00:109-149 with no removed separators.
P00 resolves to `rust/fspec-tui/src/transport/backend/core.rs`. Verified baseline
and current source hashes against the plan and compared all 41 original physical
lines to the 41 relocated lines after trimming indentation. The exact new array:

```text
109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,
126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,
143,144,145,146,147,148,149
```

This is source-provenance relocation, not a semantic retarget to a differently
located method suggested by the scenario's name.

## Successful native mutations

Each link below used `featureName=rpc027-help-disconnect-thinking-dialogs`, the
exact scenario name in the table, no skip flags, and returned `success: true`.
Each link was followed by unlinking only that scenario's obsolete root test
mapping, also returning `success: true`:

1. Link D test `thinking_level.rs:111-145` with unchanged implementation 144-152.
2. Unlink D test `rust/fspec-tui/tests/rpc027_dialog_parity_bcd.rs`.
3. Link uppercase D test `thinking_level.rs:148-169` with unchanged implementation 144-152.
4. Unlink uppercase D test `rust/fspec-tui/tests/rpc027_dialog_parity_bcd.rs`.
5. Link wiring test `thinking_level.rs:172-230` with `transport/backend/core.rs:109-149`.
6. Unlink wiring test `rust/fspec-tui/tests/rpc027_dialog_parity_bcd.rs`.

Linking first preserved coverage during migration; unlinking removed the obsolete
test mapping and its implementation link only after its replacement existed.
No all-scenario reset or blanket unlink was used.

## Native readback and preservation

Supported `fspec show-coverage` readback reports:

```text
Coverage: 100% (11/11 scenarios)
Covered: 11; Uncovered: 0
Test Files: 2; Implementation Files: 4
Test Lines: 367; Implementation Lines: 1594; Total Lines: 1961
```

Independent read-only JSON checks confirm all 11 scenarios retain exactly one
test mapping and exactly one implementation mapping. All three replaced test
paths/ranges and exact implementation arrays match the verified targets. The
three previously migrated ThinkingLevel scenarios remain at 10-26, 29-74 and
77-108 with their full original implementation array 1-208; the parent-strengthened
footer mapping was not altered.

Help/Disconnect physical source lines 1-249 in the root are byte-identical to
HEAD. Their five functions are actually at 99-139, 142-167, 170-193, 200-228 and
231-249. Existing coverage ranges remain 90-134, 135-159, 160-189, 190-220 and
221-245, respectively. Those inherited range differences predate this extraction;
they were not guessed away or broadened under this moved-test assignment.

Both test sources retain their pre-mutation hashes. All 22 protected baseline
file hashes match, including BUG-283 sources, specs, mappings and tests.

## Evidence fingerprints

- Coverage sidecar before this assignment:
  `edd2008433114f905e55174897cbc4d70f7f80021cf6bd83e167e764cb91ffec`.
- Coverage sidecar after native mutations:
  `c4110116bf1db2d3eef3e6d71b111ac46ba9b4f84d5f6705030995dc5099205e`.
- Root test source, unchanged:
  `7db2d7f0592757d9a1e0702c10c9127164a75b0ac40a4162df65152f9c3f91a5`.
- Extracted thinking-level test source, unchanged:
  `fba786df75d80fb837372af0c695051a10c85e24ef2f3d432270cb3ba9b8a0b2`.
- Parent-owned `/tmp/refac019-footer-proof.log`:
  `1acb6658b79e04e25c07e6ea5509aad4e98d71791515637aa6237b53e65d1db0`.

The parent log records 11 passed / 0 failed, including the actual centered-footer
assertion. This assignment inspected that existing receipt and did not rerun or
claim a new test execution. Mapping readback is not itself behavioral evidence.
