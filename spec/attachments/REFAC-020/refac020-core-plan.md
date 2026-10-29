# REFAC-020 core plan — review and causal RED required

## State and authority

This is the parent-approved implementation/test contract, not GREEN evidence.
At initial publication this continuation had made no repository/status changes.
After approval native Example Mapping, specifying/feature validation and testing
preceded new focused tests. No production, migration or Cargo invocation has
been performed by this worker; parent-run causal RED remains the next gate.

Read `/tmp/refac019-approved-coverage-continuation.md` for the exact human yes
at 2026-10-28 21:32:19 UTC. It supersedes the older tooling proposal's pending
authorization/activation language only within its expressly bounded exception.
Parent owns Cargo, review, migration authorization and canonical Done.
Original REFAC-019 behavior/contracts/BUG-283 preservation receipts remain frozen.

Parent subsequently replaced the worker role and approved the core plan/API,
four-root decomposition, private registration and four core help configs.
Feature-first Example Mapping and tests are authorized; production still waits
for parent-run causal RED and explicit production approval. Parent owns Cargo.

## Proposed wire contract for CLI coordination

All four existing `run(args_json: &str, project_root: &Path)` entrypoints stay.
Existing dispatcher routes already reach them; no dispatcher edit is needed.
The CLI must marshal these arguments, never implement coverage mutations.

* `link-coverage`: optional `testMappingIndex: usize` and
  `expectedScenarioHash: string`; indexed mode is implementation-only and still
  requires exact `testFile`, `implFile` and `implLines`. Reject `testLines` with
  an index rather than silently appending or changing a test span.
* `unlink-coverage`: same optional pair. With index and `testFile`, remove that
  one row; with `implFile`, remove the existing first matching implementation
  within that explicitly selected row. No implementation deduplication or new
  implementation-index API. Reject index plus `all`.
* Index and hash must be supplied together. Reject malformed/non-hex/wrong-length
  hashes, negative/fractional/out-of-range indices, file mismatch and stale hash
  without rewriting coverage bytes. Hash syntax: 64 lowercase hex characters.
* Hash is SHA-256 of compact canonical JSON of the entire selected raw scenario:
  recursively sort object keys; retain array order, duplicate occurrences and
  all unknown members. Do not hash typed coverage projections or pretty output.
* Successful mutations add `scenarioHash` to the existing success envelope,
  computed after the mutation. Existing message/warnings semantics remain.
* `show-coverage`: opt-in `mappingJson: true` (`--mapping-json`), requiring
  `featureName`. This explicit mode selects machine readback independently of
  the ordinary format default; absent/false preserves markdown/text/json. Return
  `fileName` and ordered `scenarios`, each with `name`, `scenarioHash` and ordered
  `testMappings: [{"testMappingIndex": 0, "mapping": <complete raw row>}, ...]`.
  Preserve every unknown row member in readback; hash includes unknown scenario
  members even where those are not separately rendered. No project-wide fallback.
* `generate-coverage`: optional `featureName` alongside existing `dryRun`.
  No selector retains documented global scope; selected mode must never fall
  back to a global scan, create unrelated missing sidecars or repair corruption.

These exact argument/readback choices are a proposal for parent/CLI approval,
not a claim that they are supported by the currently running daemon.
Read the CLI worker's newly published `/tmp/refac020-cli-decomposition-plan.md`;
the index/hash/scoped-feature arguments agree. This plan adopts its explicit
`mappingJson` proposal instead of introducing an additional `format` value.
Core help-config ownership is proposed here; no busy-recipient message was
sent or presumed delivered. Parent still must approve the shared contract.

## Transaction and validation boundary

Use a private shared `commands::coverage_mutation` module, registered in the
178-line `commands/mod.rs`. It owns path identity, canonical hash, selector
validation, typed internal errors and a stable per-sidecar transaction guard.
Map typed `thiserror` failures into existing `FspecCoreError` at command boundaries;
do not broaden the public error enum merely for this repair.

Lock a persistent sibling `<feature>.feature.coverage.lock` using fs2, not the
sidecar inode replaced by rename or an atomic writer's temporary inode. Acquire
before reading sidecar state, retain through preconditions, mutation and atomic
replacement; release by RAII. Every affected legacy link/unlink and global or
selected generator path must participate. A stale selector is checked inside
this transaction, not during an earlier independent read. Do not nest guards on
the same sidecar. Keep existing atomic serialization/newline compatibility.

Indexed mode retains normal flag/file/step validation, including story/bug
skip-step-validation rejection. Existing explicit skip flags retain their old
documented behavior; new selector flags do not imply either skip. Migration
uses neither bypass. Preserve unknown fields while updating only required stats.
Do not replace selected raw JSON with lossy typed coverage structs.

### Newly discovered writer scope gap — parent decision needed

The four coverage commands are not every core sidecar writer:

* `update_scenario.rs:144` reads and `:157` rewrites coverage during rename.
* `delete_scenario.rs:245` reads and `:301` rewrites coverage during deletion.
* `delete_scenarios.rs:335` reads and `:382` rewrites coverage during bulk deletion.
* `create_feature.rs:185` creates/writes a sidecar, including a possible existing
  orphan sidecar at a newly created feature's path.
* `show-coverage` can write an arbitrary `output` path; callers must not target a
  live sidecar with a report. This is not a coverage read-modify-write operation.

Parent decided the bounded guarantee covers participating link/unlink/global
and selected generation only. Do not expand to scenario/create/delete writers
or claim repository-wide safety. Those commands, report-to-sidecar writes and
the old daemon are nonparticipants; prohibit concurrent nonparticipant writes
during the exclusive pinned one-time migration. Tests prove participating
writers only. The gap remains documented rather than silently fixed or hidden.

## Selected generation semantics

Resolve an exact `.feature` target using established project feature traversal;
support an exact relative nested feature path and reject ambiguous basenames,
missing/invalid names and escaping paths. Derive the one corresponding sidecar.
Do not use a substring match or infer that a missing requested target means all.

Produce one shared in-memory plan for dry-run and apply. Raw JSON is authoritative.
Preserve retained scenario order, unknown top-level/stats/scenario/row/impl fields,
duplicate mapping/line occurrences and all retained arrays. Append newly declared
scenario names in feature order. Empty obsolete scenarios may be removed; any
mapping lineage blocks removal, even an otherwise incomplete mapping row.
An invalid JSON/shape, unreadable existing file, uncertain duplicate-name
association or mapped removal blocks the entire selected sidecar. Only NotFound
permits creation. Preserve same-name Scenario Outline mapping lineage rather
than multiplying or deduplicating rows by example count. Include Rule scenarios
according to the existing parsed Gherkin model; do not silently omit them.

Proposed selected-mode response is a JSON envelope containing `success`,
`dryRun` and `plan: {featureFile, coverageFile, status, addedScenarios,
removedScenarios, blockedReasons}`. Status is create/update/skip/blocked.
Dry-run returns the blocked plan without a write; actual blocked apply returns
an ordinary command failure with the reasons, also without a write. Successful
apply returns the same plan as preview except `dryRun`. Parent/CLI must approve
rendering before tests freeze this contract. Preview never claims recreation of
corrupt existing lineage. No partial additions on any blocked selected plan.

Dry-run creates no lock artifact when no lock file exists: read-only optimistic
snapshot is advisory; real apply reacquires/recomputes under the writer guard.
Do not promise the preview remains valid after an external writer changes data.
No-op apply should preserve original sidecar bytes. Existing global-mode output
compatibility stays; selected-mode safety must not be silently attributed to old
global behavior without corresponding tests and approval.

## Honest bounded source decomposition

Every maintained touched Rust source must finish strictly below 300 physical
lines. Use ordinary private modules/full functions, not aliases to oversized
sources, macros to hide maintenance, member fragments or formatting compression.
Leave existing step-consistency implementation untouched unless a test proves
the bounded new behavior requires an approved change.

| Root | Proposed meaningful full-item modules | Target root ceiling |
| --- | --- | --- |
| link_coverage.rs (752) | validation.rs, scenario.rs, work_unit.rs, mappings.rs, stats.rs, tests.rs | 250 |
| unlink_coverage.rs (471) | mappings.rs, stats.rs, tests.rs | 240 |
| generate_coverage.rs (386) | legacy.rs, legacy_reconcile.rs, render.rs, selected.rs, selected_plan.rs, tests.rs | 200 |
| show_coverage.rs (743) | status.rs, single.rs, markdown.rs, json.rs, project.rs, project_render.rs, mapping_json.rs, tests.rs | 210 |
| new coverage_mutation/mod.rs | transaction.rs, selector.rs, canonical.rs, target.rs | 120 |

Each extracted module targets 80–250 lines with room for imports and attributes.
Split orchestration from unlink's current 211-line run body before adding fields.
Do not deduplicate the historically different link/unlink stats semantics just
because both functions share a name. Preserve original tests/assertions during
mechanical extraction and compare existing command outputs through regressions.
Help metadata may need four bounded config edits; have parent assign ownership.
Core Cargo.toml needs only the shared test-helper dev dependency if approved.
No dispatch edit, original tests rewrite or CLI business-logic duplication.

## Feature-first ACDD and exact RED targets

After role/plan approval: native Example Mapping, specifying state, then create
`spec/features/precise-lossless-coverage-mutations.feature`. Map each following
scenario 1:1 to one focused test, using Scenario Outlines for validation matrices.
Only after the feature exists and testing state is recorded create these targets:

1. `tests/refac020_selected_coverage.rs`, ordinary includes under
   `tests/refac020_selected_coverage/`: readback.rs, linking.rs, unlinking.rs,
   rejection.rs, validation.rs and support.rs. Each under 300 lines.
   Test/scenario names:
   * `readback_exposes_ordered_rows_and_canonical_scenario_hash`
   * `selected_second_duplicate_gains_complete_two_file_union`
   * `selected_obsolete_duplicate_is_removed_without_losing_other_occurrences`
   * `invalid_or_stale_selector_leaves_exact_sidecar_bytes_unchanged`
   * `selected_link_retains_file_and_story_step_validation`
   * `unselected_mutation_retains_legacy_duplicate_selection_behavior`
2. `tests/refac020_scoped_generation.rs`, ordinary includes under its matching
   directory: scope.rs, preservation.rs, blocked.rs, preview.rs and support.rs.
   Test/scenario names:
   * `selected_generation_does_not_touch_unrelated_sidecars_or_missing_files`
   * `selected_generation_preserves_unknown_fields_order_and_duplicate_lineage`
   * `mapped_scenario_removal_blocks_all_selected_changes`
   * `invalid_or_unreadable_existing_metadata_is_never_recreated`
   * `dry_run_and_apply_report_the_same_actual_selected_plan`
   * `invalid_ambiguous_or_missing_feature_never_falls_back_to_global_generation`
3. `tests/refac020_coverage_transactions.rs`, bounded support/worker files.
   Test/scenario names:
   * `competing_selected_mutations_allow_one_fresh_hash_and_reject_the_stale_one`
   * `legacy_link_and_unlink_wait_for_the_shared_sidecar_transaction`
   * `selected_and_global_generation_wait_then_preserve_latest_participating_write`

Use `codelet_test_helpers::setup_temp_directory`, real feature/coverage files,
valid story Given/When/Then comments, known physical implementation lines and
independently expected full mapping arrays. Assertions must check multiplicity,
unknown fields, unchanged unrelated byte hashes and negative write outcomes,
not just percentages. Compute expected canonical hashes independently of the
production helper. Include reordered-object/same-array and reordered-array
hash cases and reject unknown selector modes; do not normalize duplicates away.

Concurrency tests must coordinate child workers with explicit start/ready/result
handshakes and an externally held stable lock, not race probabilistic sleeps or
run subprocess-global mutations in an unguarded shared fixture. Cross-process
fs2 locking matters. Verify both blocking before release and final state after
release. Bounded timeouts are failure safeguards, not evidence of serialization.
CLI/dispatcher parity acceptance belongs to the CLI owner and must exercise the
same command entrypoints and exact camelCase arguments above.

Parent-run RED commands, once tests exist (working directory `rust/`):

`CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_selected_coverage -j 2`

`CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_scoped_generation -j 2`

`CARGO_NET_OFFLINE=true CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-core --test refac020_coverage_transactions -j 2`

Require causal behavior failures (wrong duplicate, ignored selector, global
side effects, lossy fields, false preview, stale overwrite), not compile errors
or broken fixtures. Existing-validation regression scenarios may already pass;
label them baselines, not newly established RED. Production only after parent
approves plan and reports target-specific RED against frozen pre-change sources.

## Validation and migration gates

After implementation parent reruns all three targets plus existing link_coverage,
unlink_coverage, generate_coverage, show_coverage and coverage_mapping_remediation
integration targets. Parent owns scoped check/Clippy/format, feature/tag validation,
CLI parity and independent review; no workspace tests. Verify exact baseline
assertion retention and all touched physical sources below 300 lines.

Only after GREEN/review may parent authorize precise local CLI migration using
tested binary/source provenance, original-byte backups/hashes, pinned targets,
fresh hashes, expected deltas and complete mapping-multiset/unrelated-byte
readbacks. No install/restart, daemon schema change, direct sidecar mutation,
validation skips, commits, security/history edits or passing-percentage Done.

## Read-only preimplementation fingerprints

* link_coverage.rs: f57790771cf73df585ac2be6f41272ee8667dd4ecf3c6e80f6f5e18519272810
* unlink_coverage.rs: 461f64e20b95e790b4a35e6cc3374e712bdde4cd814bc143492bd4008bf0eda2
* generate_coverage.rs: 9991756bb07e19f6ba1a7dad372d73ddb8c5e276bdf297f84bcee7308ead37ad
* show_coverage.rs: 147af9d62a6a7397eec1bfcec9a6eba8e14ac62aecd301179fafb8f99420639d

These are read-only observed fingerprints, not a replacement for parent-owned
immutable baseline receipts, causal RED, final provenance or canonical statuses.
