# COV-059 design: retire or rewrite coverage scenarios that point at removed code

## Problem

COV-058 left two coverage mappings naming files that no longer exist:

- `spec/features/model-selector-cloud-sections.feature.coverage` mapped its only scenario
  ("Uncredentialed cloud sections are dropped from the selector", `@e2e`) to
  `e2e/prov-126-cloud-sections.test.ts`. The `e2e/` TypeScript client was deleted; the
  directory does not exist.
- `spec/features/rpc017-cross-transport-parity.feature:52` ("napi::move_work_unit_up is wired
  through the same shared helper") reads `rust/napi/src/work_units_watcher.rs`. `rust/napi` was
  removed by REFAC-027. Its coverage pointed at `move_work_unit_rpc017.rs:163-193`, which is the
  done-column transport test, not a napi test (no napi test exists).

Looking at the same RPC-017 features found the same problem in three more places:

- `rpc017-source-shape.feature:87` "NAPI exports for move_work_unit_up / _down delegate to the
  shared helper" — the same removed file; its coverage pointed at other tests' lines
  (`source_shape_rpc017.rs:158-186`).
- `rpc017-source-shape.feature:43-51` "codelet_common::file_lock is lifted out of
  schedule_handler.rs" — three steps name `rust/napi/src/schedule_handler.rs`, but the test
  (`rust/fspec-tui/tests/source_shape_rpc017/shared_helpers.rs:10-13`) reads
  `rust/agent-loop/src/schedule_handler.rs`, which is where the handler lives today
  (`rust/agent-loop/src/schedule_handler.rs:18` imports `codelet_common::file_lock::with_file_lock`).
- `rpc017-source-shape.feature:93-96` "Views do not directly import codelet_core / napi / tarpc" —
  the `codelet_napi::` needle checks for a crate that cannot exist any more.

## Decision per scenario

| Feature | Scenario | Decision | Why |
|---|---|---|---|
| model-selector-cloud-sections | Uncredentialed cloud sections are dropped from the selector | **Retired** (feature file and coverage removed; the feature had no other scenario) | It launched the binary from a TypeScript e2e harness that is gone. Every rule it covered is already pinned by Rust tests: drop-empty in `model-selector-cloud-section-assembly` ("A cloud provider with no models is dropped…", `rust/sessions/src/profile_sections/tests.rs:115`), credential-seeded `list_providers` drop in `rpc-073-list-providers-wiring` (`rust/sessions/tests/rpc073_list_providers_wiring.rs:184-243`), and pluralisation in `model-selector-header-count-label` (`rust/fspec-tui/src/views/model_selector/rows_tests.rs:226-270`). A rewritten copy would duplicate them. |
| rpc017-cross-transport-parity | napi::move_work_unit_up is wired through the same shared helper | **Retired** (`fspec delete-scenario`) | The NAPI exports no longer exist; the shared helper is pinned through both transports and FspecService by the remaining six scenarios. |
| rpc017-source-shape | NAPI exports for move_work_unit_up / _down delegate to the shared helper | **Retired** (`fspec delete-scenario`) | Same removed file; no Rust surface replaces it. |
| rpc017-source-shape | codelet_common::file_lock is lifted out of schedule_handler.rs | **Rewritten** | Behaviour still exists: steps now name `rust/agent-loop/src/schedule_handler.rs`, the file the real test reads; `@step` comments updated byte-identically. |
| rpc017-source-shape | Views do not directly import codelet_core / napi / tarpc | **Rewritten** as "Views do not directly import codelet_core / tarpc" | The `codelet_napi::` needle is dropped from the step and the test; the other needles stay. |

Doc strings that described napi files as current are corrected (`rpc017-cross-transport-parity`,
`rpc017-source-shape` points 1, 2 and 6, `rpc017-work-units-write-helper`). The two test files that
named the deleted feature in their `// Feature:` header now name the features that hold their
scenarios (`profile_sections/tests.rs` → model-selector-cloud-section-assembly, `rows_tests.rs` →
model-selector-header-count-label).

## Alternatives rejected

- **Rewrite the e2e scenario as a Rust `list_providers` test** — duplicates
  rpc-073-list-providers-wiring's scenario, and the providers it names (together, moonshot,
  cohere, mistral) are not the ones the current catalog seeds.
- **Keep the napi scenarios with `--skip-validation` coverage** — that is exactly a dead mapping
  that lies.

## Out of scope (reported, not changed)

A scan of every `spec/features/*.coverage` file finds 472 features / 3152 scenarios whose test or
implementation mappings name files that do not exist (mostly `src/**/__tests__/*.test.ts` from the
removed TypeScript CLI). That clean-up is much larger than this card and needs its own cards.

## Wire / API changes

None.
