# REFAC-019 safe-30 native correction receipt

## Authority and bounded outcome

Read `/tmp/refac019-approved-coverage-continuation.md` first, then
`/tmp/refac019-safe-remaining-review.md`, as instructed by the supervisor.
The trusted authorization records the human's actual 2026-10-28 21:32:19 UTC
approval of the bounded repair and validated migration exception. Earlier
receipt wording attributing Prepare Tooling Proposal to the human was inaccurate:
that was a controller decision. This assignment used only native Fspec metadata
commands; the local CLI exception remains unused and parent-owned.

Final result: **29 of 30 corrections completed**, comprising all six
implementation relocations and 23 of 24 same-file test-range repairs. One
RPC057 native validation rejection is retained honestly below. No attempt was
made to force it, retry unchanged, change comments/assertions, substitute a
different test, or bypass validation.

- 13 affected feature documents; 108 scenarios and 108 test rows before/after.
- Implementation entries: 132 before, 133 after. The increase is the required
  RPC011 two-file union; all 24 test-range candidates retain their live arrays.
- Exactly 29 rows match their complete expected replacements; the rejected
  30th row remains identical to its original row. All 78 nontarget rows and all
  other document fields match the initial snapshot, apart from recalculated stats.
- 30 successful native link calls, 28 successful native unlink calls, one
  rejected link call. No failed request was retried unchanged.
- All six selector-dependent scenarios remain exactly equal to the snapshot,
  including the separate duplicate RPC037 row and its existing first union row.
- All 18 scoped source fingerprints and all 22 protected hashes still match.

No repository source edits, builds/tests, work-unit status changes, global
generation, shell coverage writes, CLI migration, daemon/history/install/commit
changes, or BUG284 changes were made. Temporary read-only verification code and
this receipt are the only additional files created outside coverage metadata.

## Before/after evidence

Before mutation, every affected sidecar's actual byte hash matched its captured
entry in `/tmp/refac019-safe-review-snapshot/current-coverage.json`. That immutable
snapshot supplies the complete original JSON content and byte hashes used for
comparison. No new baseline was synthesized from partially mutated state.

All paths below are under `spec/features/` with suffix `.feature.coverage`.

| Feature | Before SHA256 | After SHA256 |
|---|---|---|
| rpc011-regression-invariants | d94e876168c4d2ab1e7cb0be5daef65fb49b884c56b227a845cfec5335b1b48a | b1fe00f761d8e442ece8169a4f5ca0ca8b30755d19b833647e44c7b7e7696212 |
| rpc012-action-variants | 4cecd09541b38080871b9ca8c7062c0cc1d3e34e0ba2ab99ec7b45b84f0cb3ff | 08e6540355e11152b18305cd8ca3dc73edd529654847eb3441026bacbee24aaf |
| rpc015-source-shape | 7dd5007ffe3f83a2b35d8045af23db49f379bfbeaf756192722d909eee9cc737 | b56e1704a36df5fc3462e6acf0d61ba06cd805c107f7004ae93adc8b96568cb2 |
| rpc017-source-shape | 8231f7c870f53cb1dc07648063cf968b2b08dace3f0354aa3ae19d5cd35c2781 | c66695e945d0b5a445c95731629a64c96251ac3fe41691a08b5f55a4f38cb644 |
| rpc018-source-shape | 99fd3551cf042ba46739296adfb71c117b9b9b4e9a11664f609dc5b4bc1a8c01 | 3866d2061afc4285cbd9781811d969d34c9e84491bfd8873b6fac1a7f52b33ae |
| rpc019-source-shape | 83414f10c7395220b1b3d45da4347b14f250dd06af2c6f5f227f7ab5bf3eeb51 | fdc7c6c5dc9d97c797b027a11e6095d97706e466044a4e6559c08ac4590ecd03 |
| rpc020-source-shape | eac5defef35efc8ab6bc836009c4e4c49bf841e6e019dbdb661d0c52ef10a85d | a33dd35c36264b020178333fa25e7e7fca68eb4afc035f8043a9a399300bc193 |
| rpc022-source-shape | e1bffa19a2a18a8bbd1fd854e66f057b5d6bdefca6a682f79f9ad16ac56da6bd | 49ed3cc954cc2bf54bd89e1d66f081b88b9207444032ac2f6e70aff96ada0e12 |
| rpc025-rpc-methods | c1fa0edebd7fe9e4ade0d8d85b0d76246f8d5580d4b48273b23e6a0036976d05 | 5c8d3ea12806bef26e23358589503932142ca7e25155547cf2b316c8afdd3be2 |
| rpc054-provider-settings-source-shape | c9b41aa59663832eb2c2c61c757494a60ccf2322ba7e2318fbccfcca924215f5 | 7ea06eb813087d896fe9971c5f7f3104ec64c8492770cdc46acc24d218a1a494 |
| rpc055-slash-debug-source-shape | 3cd957772351316c0e3988e707e6c9f274edb0a094b65fb6dbed16d5fddb2e8b | 895f4206586b87556ac2a1999273e9f8ed70a37ac729fc9b29e19b03473b9655 |
| rpc057-merge-worktree-source-shape | 62fa09ec8b8b32b3f42d4569423a69771eadd34f0208af3ca15e7d5dda28f954 | bfd8a0d632c023e5a71ffe43755a2ffc1d6e5f168d7efbb8eeea162cf4ef1349 |
| rpc059-loop-source-shape | 3ad568d85d7b602eb1ede7b21bf7d90de1fadc743387acec75cda8525325dda6 | 5eb7dc1c5600cf0614b7c92b9933712f970ce54d944ef43dc0fb4b066b75d275 |

## Provenance and supported operation checks

Read-only verifier: `/tmp/refac019-safe30-native-check.py`.

- It derives the six implementation changes and 15 extra-RPC test repairs from
  `/tmp/refac019-extra-rpc-plan.json`, adds the explicitly approved parent RPC018
  mixed range, and includes exactly the eight reviewed additional partial spans.
- Source fingerprints are checked before each verifier run. All six original
  implementation spans match the physical successor sequences with indentation
  trimmed; the reviewed sole removed reference is the RPC011 blank separator.
- All three mixed spans and all eight additional partial spans compare exactly
  against their original HEAD physical slices. Assertion token multisets match.
  Other test repairs use the reviewed original same-function spans, rather than
  treating stale/out-of-bounds metadata ranges as valid original source slices.
- For implementation changes, all new physical mappings were added and read back
  alongside their unchanged original before old mappings were removed. RPC011's
  root was updated to 60-64, not removed, after construction.rs:1-7 was verified.
- For 23 accepted test repairs, complete replacement rows were appended with
  the entire single live implementation. Readback proved each exact old row was
  first and its complete replacement second before any original was unlinked.
  Final readback proves exactly one corrected row per accepted scenario.
- The completed mixed ranges are RPC018 193-227, RPC020 popup-fields 103-114,
  and RPC020 size 131-152. Historical cross-assigned provenance is preserved,
  not narrowed to scenario-named tests or reinterpreted as new semantic proof.
- Final complete-document comparisons match exactly after substituting the
  preserved rejected row for its unapplied target; implementation order is
  canonicalized only for comparison, not rewritten on disk. Unknown fields,
  scenario order, row multiplicity, and unrelated mapping arrays remain intact.

## Exact new native validation rejection

Feature: `rpc057-merge-worktree-source-shape`.
Scenario: `/merge-worktree slash command wiring lives in dispatch_merge_worktree.rs`.

Rejected native link arguments:

```json
{"featureName":"rpc057-merge-worktree-source-shape","scenario":"/merge-worktree slash command wiring lives in dispatch_merge_worktree.rs","testFile":"rust/fspec-tui/tests/source_shape_rpc057.rs","testLines":"173-197","implFile":"rust/fspec-tui/src/app/dispatch_merge_worktree.rs","implLines":"1-252"}
```

Native validation reported these missing scenario steps:

- Given the file rust/fspec-tui/src/app/dispatch_merge_worktree.rs exists
- Then it declares a method named "handle_slash_merge_worktree"
- And it declares a method named "handle_inspect_changes_loaded"
- And it declares a method named "handle_merge_confirmed"
- And it declares a method named "handle_discard_confirmed"
- And it declares a method named "handle_cancel_merge_dialog"
- And it declares a method named "try_dispatch_merge_worktree"

The original row remains `source_shape_rpc057.rs:371-395` with its entire
`dispatch_merge_worktree.rs:1-252` implementation array. No new row was appended
and no old row removed. Physical-source inspection confirms that the approved
successor 173-197 is exactly the historical dialog-test tail plus Default-test
documentation/start, not slash-wiring assertions. The root and its included
service-contract source do not supply the missing slash-handler step comments.
The byte-identical relocation proof does not authorize inventing those steps or
retargeting the historical row. Parent must classify/resolve this additional
semantic gate separately; this receipt does not claim 30/30 completion.

## Native supported audit

Ran native `audit-coverage` without `fix` for all 13 affected features. Twelve
return exitCode 0. `rpc019-source-shape` returns exitCode 1 for one pre-existing
missing implementation file: `src/tui/components/MultiLineInput.tsx`, lines 1-50,
in scenario `Existing TS AgentView input + scrollback files are untouched`.
Its row and implementation are identical to the before snapshot and were not
among the authorized corrections. No deletion or repair was attempted.

Native audit totals: 240 of 241 referenced file occurrences exist. Per-feature
checked occurrences, in hash-table order above: 9, 2, 18, 16, 30, 19, 36, 21,
20, 22, 10, 21, 17. Only RPC019's 19 includes the one missing path.
This audit validates file existence, not source spans or behavioral correctness;
the separate physical-span/document checks provide the relocation evidence.

The six selector-dependent scenarios and RPC026/RPC050/Mux semantic ownership
were not touched. No conclusion about other workers' concurrent owned files,
global coverage closure, canonical REFAC-019 Done, or BOARD-024 Done is implied.
