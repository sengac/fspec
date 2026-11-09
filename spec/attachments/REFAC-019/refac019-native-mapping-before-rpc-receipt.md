# REFAC-019 native mapping receipt: basenames before rpc

UPDATE: the original BUG181 blocker is now resolved by the separately approved
same-name outline correction. See /tmp/refac019-bug181-lineage-correction.md.
Original-plan totals are now 25/25 implementation entries → 33 physical entries;
all 46/46 owned moved definitions and 53/53 mapping occurrences are reconciled.
The counts/hashes below retain the accepted first-phase checkpoint, before that
semantic correction. Additional in-bounds repairs have their own final receipt
/tmp/refac019-native-additional-before-rpc-receipt.md; only its row 16 remains held.

First-phase final, except its then-held BUG181 semantic blocker. All repository
mutations used native functions.fspec link-coverage/unlink-coverage only.
No repository source/status edits, builds, commits or restarts.
All 60 plan source fingerprints and all 22 protected hashes verified before writes.
Owned plans: 25 original implementation entries across 13 features; test inventory
46 definitions across 6 features. BUG181 deletion guard (audit impl 17/test 95)
remains blocked pending parent's semantic resolution; no rejected request retried.

Early native gate: linking Action replacement to the old inline HITL root test
was rejected because its steps now live in options_and_source.rs. Read that
actual relocated definition (all five required existing comments/assertions).
Linked the new test and exact two Action targets there successfully, then
removed the old test mapping. No test/spec edits or validation bypass occurred;
the rejected old-root request was not repeated.

## Final counts and readback

- 24 original implementation entries relocated to 32 exact physical locations
  across 13 feature sidecars: Action 2→3, RPC 5→5, transport 17→24.
- 882 original implementation line-reference occurrences become 876. The six
  omitted occurrences are documented removed blank separators only: transport
  audit entries 0/1/5 drop old WebSocket line 352; entry 6 drops 226; entries
  7/8 drop 65. All remaining references and unrelated implementation arrays
  retain their full multiplicity.
- 40 moved definitions newly reconciled, covering 47 test-mapping occurrences:
  inline HITL 13 definitions/13 rows, inline pause 20/27, live checkpoints 7/7.
  The 7 extra pause rows preserve existing same-function implementation lineage.
- 5 previously relocated definitions/rows remain unchanged (bootstrap, tracing,
  and the 3 pause/HITL error/source-shape scenarios). Final relocated inventory:
  45/46 definitions and 52/53 test-mapping occurrences. No staged copies remain.
- All 17 scoped feature scenario-content hashes below match the pre-write
  expected values exactly on final readback (0 mismatches). Canonicalization
  normalizes collection order, never duplicates or line arrays. This verifies
  scenario names/counts, test multiplicity and every unrelated implementation.
- All 45 relocated ranges contain the exact audited function identity.
- Final recheck: 60/60 source fingerprints match; 22/22 protected hashes match;
  frozen implementation-audit SHA256 is
  64813343dd6da0fd1c044248e910582d30e51dfd22db7e8c2c6e37e55f9758b8.
- No protected target overlap, false step comments, skips, status changes or
  direct sidecar edits. Existing Done statuses remain untouched.

## Parent-owned remaining blocker

Feature live-checkpoint-counts-in-the-board-header; scenario
"Embedded transport delivers a live checkpoint-count frame end-to-end into the
BoardStore": the original second row remains byte-for-byte semantically intact:
test rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs:406-440,
implementation rust/rpc/src/lib.rs:[819,820,821,822,823,824,825,826,827].
Its physically moved function is
embedded_transport_delivers_a_decremented_frame_after_a_checkpoint_delete at
rust/fspec-tui/tests/bug181_live_checkpoint_counts/polling_and_surface.rs:147-182.
The parent's investigator owns correcting its deletion-vs-creation scenario
semantics. This worker made no attempt to repeat the previously rejected link.
The original first creation row remains separate at old test range 176-209,
with its valid relocated embedded implementation worktrees_scheduling.rs:71-77.
Native impl operations select the first same-test-file row, so applying the
second RPC relocation before resolving this lineage would target the wrong row.

The live-checkpoint hash below intentionally includes this unchanged blocked row.
Ownership is released for the parent to perform the coordinated semantic repair.

## Pre-write expected canonical scenario-content readback

Hashes include every scenario and test/implementation occurrence (multiplicity
preserved); only array order within a scenario's test/implementation collections
is normalized. Planned relocations applied in memory; BUG181 deletion excluded.
Read-only verifier: /tmp/refac019-before-rpc-read.py.

```text
auto-reconnect-supervisor ef878bae0eb27d3a01ede3a49edb367968a03b4c0e8166cc40be3d2293883a4b
compaction-reduction-display-contract 87d2d9a9e365069c4adc8cf5ade020f07364a81cc00e2157f78f4bbf6f3a5eed
custom-model-rpc-cross-transport-parity ba79a5578b998d5efc35732273e0ad75172ba3768432c203931a83d084c39730
disconnect-dialog-cr1-baseline 63ebd5aee03fd927bdfa897dcaedc53915ba7e914feed8704a4c006fe9e953c1
fspec-tui-app-bootstrap-rpc009 af760fd40eb14ed790b09e98ec377a514d2e2ad6ba4adc0b024c546736d224e1
fspec-tui-ws-backend fe714620adc2c44b82795039dbf54324f383e730deecfcb93deb5cf019a3a6cb
git-changed-files-transport 15a5a64aeecdaf8bb6df4f85b7c765a2790cfa901557dd42f8175f80c8447345
gitstate-capture-on-the-production-service-wiring-runs-off-the-async-pool 274022a9fd3ff3d22b8fc8586239d28891a6617eaaa3d673869ae22262fdc328
health-rpc-serverstats 82dd44de88441ece2fb6286869a53c30bd423ef29997300386115262d90a01ac
inline-hitl-prompt 32700ee9494811d4013f182c100203db193745e3331bc7633ecd32858b2d92b3
inline-tool-approval-pause-prompt b3e5516ec393e66113f9d63496f74300890b125456509fc90154cebaaaffee7c
live-checkpoint-counts-in-the-board-header b5d4c58fb55e86e08562b343bb7664246479fe19c5aaa07ca6be7b82ab5b805e
multi-client-and-tracing d92f3a419b609e760deb4f1a0b2107f1d1be6bac1c9932273e2809d1c309f6a3
pause-and-hitl-dialogs cff37f4fc2b143478e0afe91b53e329fd6975d5e6f4ea633dab1033867d1e396
prov159-rpc-lifecycle abc829ee03f07e575a10316f7e4def6a27e14d243ca45c9793a96828bf6ba37b
provider-config-profile-dispatch adc8f5270a3fc5a4af9225639cbd80e712f2041108e01a32b7c5886d50ea4a66
provider-settings-oauth-disconnect 82fbc0669c8ccc4ef2ee4639e0d3f590bf695fb9b1d30a1d05ae4bc0db38f076
```
