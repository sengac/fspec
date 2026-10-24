# CLI, agent-loop and tools finisher evidence

Owner: session `62c08be1-96b1-4718-89dd-21275b895d5c`.
Verification began 2026-10-18 21:12 UTC against the current shared tree.
All selected gates finish successfully at 21:47:47 UTC. The owned portions of
all nine requested cards are ready for normal scoped coordinator gates.
This pass preserves all pre-existing work and changes evidence only: no
production, test, feature, coverage sidecar, board, index or commit edits.

## Scope and current execution

The requested portions are CMPCT-052/054/057/058/059/069/071/079/082 under
`rust/cli`, `rust/agent-loop` and `rust/tools`. This is scoped normal-gate
readiness, not full-package success, foreign-owner certification or permission
to bypass coordinator gates.

All fresh execution commands finish with Cargo exit 0:

| Package / selection | Passed | Failed | Scope |
| --- | ---: | ---: | --- |
| codelet-cli | 50 | 0 | 16 explicitly selected integration targets |
| codelet-agent-loop | 28 | 0 | 12 explicitly selected integration targets |
| codelet-tools dispatch | 4 | 0 | compaction_generation_dispatch integration target |
| codelet-tools injection | 7 | 0 | inject_summary::tests library selection; 963 unrelated tests filtered |

The loop selection also checks CMPCT-053/056/068 and PROV149 compatibility;
those neighboring card portions are not newly assigned to this pass.

## Per-card execution and historical inventory

Every mapped owned scenario below has an exact current passing test. The
failure columns count retained logs containing the exact compiled test name
with `FAILED`, not a reconstructed original test-before-production history.

| Card | Owned mapped scenarios passed | Located named failures | Without located named failure |
| --- | ---: | ---: | ---: |
| CMPCT-052 | 8/8 | 8 | 0 |
| CMPCT-054 | 9/9 | 5 | 4 |
| CMPCT-057 | 4/4 | 2 | 2 |
| CMPCT-058 | 2/2 | 2 | 0 |
| CMPCT-059 | 19/19 | 14 | 5 |
| CMPCT-069 | 2/2 | 2 | 0 |
| CMPCT-071 | 4/4 | 4 | 0 |
| CMPCT-079 | 8/8 | 8 | 0 |
| CMPCT-082 | 4/4 | 3 | 1 |
| Total | 60/60 | 48 | 12 |

The 60 mapped scenarios are a subset of the 89 executed tests, not additional
tests. Full-card provider/core/session/Rig portions retain their own evidence.

## Traceability and identity

- Fresh ordinary CLI audit: 16 features, 50 tests, 99 native commands, no errors
  or failed commands. Fresh loop audit: 12 features, 28 tests, 76 native
  commands, no errors or failed commands. Both audit processes exit 0.
- All 85 selected feature/sidecar/test/implementation paths still match
  `/tmp/cmpct-independent-all-card-readiness.json`; no mapped execution gaps.
- The owned Rust source and Cargo inputs are captured by
  `/tmp/cmpct-cli-loop-finisher-source-before.sha256` using Rust-workspace paths.
- The dependency manifest is
  `/tmp/cmpct-cli-loop-finisher-dependencies-before.sha256`, using repository
  paths. Its resumed verification exits 0 from the repository root. Running it
  from the Rust workspace causes missing-path diagnostics, not source drift.

## Quality verification

`cargo check --profile ci-test -p codelet-cli -p codelet-agent-loop
-p codelet-tools -j4` finishes exit 0 at 21:46:01 UTC.
All remaining selected gates finish exit 0:

| Gate | Completion UTC | Selection |
| --- | --- | --- |
| CLI Clippy | 21:46:34 | Library and the same 16 integration targets |
| Loop Clippy | 21:47:17 | Library and the same 12 integration targets |
| Tools Clippy | 21:47:46 | Library and dispatch integration target |
| CLI/loop format | 21:47:47 | cargo fmt, both packages, check only |
| Tools format | 21:47:47 | Four compaction tool/test files, rustfmt check only |
| Owned diff check | 21:47:47 | CLI, agent-loop and tools |
| Owned source drift | 21:47:47 | Before/after source manifests identical |

Loop/tools Clippy additionally deny `clippy::unwrap_used`, `clippy::panic` and
`unused_must_use`. Existing dependency warnings remain warnings; these results
do not claim a warning-free workspace. The final repository-root dependency
manifest verification also exits 0. No production or Cargo input changes occur
during this verification pass.

Every Cargo invocation uses `--profile ci-test`, `-j4`, and
`CARGO_BUILD_JOBS=4`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_CI_TEST_DEBUG=0`.
No unscoped or workspace-wide test invocation is used.

## Historical evidence limits

No production repair is made here, so this pass neither needs nor manufactures
a new RED. The retained inventory distinguishes actual compiled failures from
compiler errors, fixture/setup mistakes, passing controls and all-green logs
whose filenames contain `red`. Later fixture or usage failures are not
automatically evidence for the original root-cause repair. The corrected
originals oracle in a nominal overflow-green log is not claimed as new repair
proof.

One additional exact failure missing from the earlier independent inventory is
`equivalent_typed_overflow_routes_select_the_same_protocol` in
`/tmp/cmpct-cli-completion-semantic.log` (CMPCT-058).

The 12 tests without located compiled failure output are:

- CMPCT-054: `http_diagnostic_wording_does_not_authorize_compaction`,
  `bare_text_cannot_impersonate_a_provider_rejection`,
  `provider_overflow_code_survives_agent_context`,
  `scenario_cancellation_releases_the_operation_registration`.
- CMPCT-057: `committed_success_resets_operation_attempts`,
  `executing_work_is_bounded_by_its_operation_deadline`.
- CMPCT-059: `automatic_builder_publication_refreshes_the_authoritative_context`,
  `composite_persisted_selection_survives_commit_and_restore`,
  `different_persisted_provider_cannot_authorize_selected_compaction`,
  `different_persisted_model_cannot_authorize_selected_compaction`,
  `changed_selected_model_invalidates_the_installed_preparer`.
- CMPCT-082: `declared_typed_overflow_remains_detectable_through_wrappers`.

Missing located failure output does not establish an ACDD violation: some tests
were passing controls when introduced. Exact original session retrieval returns
typed `missing_envelope` failures for multiple evidence ranges. No blanket
test-before-production chronology certification or unqualified all-card
acceptance is asserted from previews or filenames.

## Reproduction and machine-readable evidence

- `/tmp/cmpct-cli-loop-finisher-verify.sh`: exact target lists and gate sequence.
- `/tmp/cmpct-cli-loop-finisher-commands.log`: expanded commands.
- `/tmp/cmpct-cli-loop-finisher-status.log`: timed exit statuses.
- `/tmp/cmpct-cli-loop-finisher-{cli-tests,loop-tests,dispatch,inject}.log`:
  complete execution output, including exact names and counts.
- `/tmp/cmpct-cli-loop-finisher-{cli,loop}-traceability.json`: fresh native
  scenario/test/implementation audit results.
- `/tmp/cmpct-cli-loop-finisher-report.py`: exact-name execution, source-hash,
  gate-status and historical-failure inventory generator.
- `/tmp/cmpct-cli-loop-finisher-report.json`: per-card scenario mappings,
  historical test names/log hashes and selected source hashes. Final generation
  exits 0 with `scoped_ready: true`, all 12 recorded statuses zero, no execution
  mapping errors and no selected source drift.

Final retained artifact SHA256 values:

| Artifact suffix under /tmp/cmpct-cli-loop-finisher- | SHA256 |
| --- | --- |
| source-before.sha256 and source-after.sha256 | 66c6a69e55ed9f88b36fac55d7305b80ab703c5addbf1825e473969820388e95 |
| dependencies-before.sha256 | 5468098c850f4c0ae7764c3c15ce3ee47ef7de592481c951233b8c48d31be6a8 |
| cli-tests.log | 0fcf04fbe07eab478baafab13ace6513cf8dd885a59432d42be7e861099de20e |
| loop-tests.log | 0ffe70818f809536bbb1d630eb74ab939b3a4bd8283735f5a306bd18c46bdabd |
| dispatch.log | 582e9f8591f4573c928a5e3d427811f7db29536d89f97b3e753de848e6bb7515 |
| inject.log | ef4eedc625de42ef2fcf63536216ea2399f094b85f94ce0d366f529c934f740b |
| status.log | 44a5ec045c27f07725f9146b369f83396cc27a69bb0dde7d938915feff78b960 |
| commands.log | abfcd479477161126adb42fb0eca1a5583a01c7470c5283cedb2f0b02822b5c7 |
| report.json | 3c2c04615577f523122e5daabc3d684ddede32b0d60a0cce064adeec8b8e7a21 |

The `/tmp` paths are local retained evidence, not permanent repository fixtures.
This attachment preserves the scoped summary and its explicit limitations.
