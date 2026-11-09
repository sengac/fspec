# COV-058: coverage mappings changed in the last six commits

## Scope

The six commits before this work (`fc06d080`..`76cb29f6`: BUG-325 and its spec
follow-up, BUG-326, BUG-327, BUG-328, RPC-450) changed 20 `.coverage` files. Comparing
each file at `fc06d080~1` and `76cb29f6` gives 67 test mappings that were added or
changed (test file, range or impl mappings).

## Check

For each of the 67 mappings: the test range must contain an `@step` comment for every
step of its scenario (the test fn that implements the scenario). Checked with a script
that parses the feature's scenarios and the test file's `@step` lines.

## Findings

37 mappings are correct. 30 are wrong:

- `custom-model-persistence` (18): BUG-325 (`fc06d080`) split
  `rust/sessions/src/profile_sections.rs` into modules and relocated the coverage by
  line arithmetic, so test mappings point into `profile_sections.rs`,
  `profile_sections/tests.rs` and `profile_sections/config_io.rs` (implementation code
  or other features' tests) or at the wrong lines of `persistence_tests.rs`. Every
  scenario's test fn is in `rust/sessions/src/profile_sections/persistence_tests.rs`.
- `model-selector-profile-server` (4) and `model-selector-cloud-section-assembly` (3):
  same split; the test fns are in `profile_sections/tests.rs` at other lines.
- Tests that gained lines in BUG-325/BUG-328/RPC-450 while their ranges were not
  extended: `custom-model-rpc-cross-transport-parity` (1, range ends 36 lines early),
  `custom-model-rpc-surface` (1), `rpc-073-list-providers-wiring` (1),
  `rpc017-cross-transport-parity` (1).
- `model-selector-cloud-sections` (1): the mapping points at
  `e2e/prov-126-cloud-sections.test.ts`, which no longer exists in the repository (the
  e2e TypeScript suite was removed with the TypeScript client, commit 103a7307); no Rust test carries
  that scenario's steps.

## Fix

Each wrong mapping is re-pointed to the range of the test fn that holds all of its
scenario's steps with `fspec unlink-coverage --test-mapping-index ... --expected-scenario-hash ...`
and `fspec link-coverage` (step validation on), and its implementation mappings are
re-linked to the new test mapping. Where the scenario already had a correct mapping to
the same fn, only the wrong duplicate is removed (its impl mappings are added to the
correct one). The `model-selector-cloud-sections` mapping to the deleted e2e file is
reported, not invented.

## Result

After the 30 changed mappings, every other mapping in the same 20 coverage files was
checked the same way (93 test mappings in all). 12 more were wrong (ranges left behind
when tests gained lines: `rpc-073-list-providers-wiring` 4, `rpc017-cross-transport-parity`
5, `custom-model-rpc-surface` 1, `per-profile-max-images-persistence` 1,
`implement-sessionmanagerhandle-for-codelet-sessions-sessionmanager` 1) and were
re-pointed the same way. For the last one fspec's step validation also showed the test's
`@step` comments for the two quoted-`SessionId` steps were not byte-identical to the
feature (the feature escapes the inner quotes); the comments in
`rust/sessions/tests/handle_impl/broadcasts_round_trip.rs` now match. Every
implementation mapping present before is still present (checked as per-file line sets).

Left as reported: `model-selector-cloud-sections` → `e2e/prov-126-cloud-sections.test.ts`
(file deleted with the TypeScript client) and `rpc017-cross-transport-parity` "napi::
move_work_unit_up is wired through the same shared helper" (its steps describe
`rust/napi`, which REFAC-027 removed; no test carries them). Both need a spec decision
(retire or rewrite the scenario), not a range fix.
