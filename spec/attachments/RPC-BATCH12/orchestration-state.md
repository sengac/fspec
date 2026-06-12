# Batch 12 Orchestration — work-units.json mutation + export commands

Supervisor session_id: `f9d9a7e5-7b54-47b0-8d8b-0835ccd762ca`

## Batch rationale

10 commands that operate on `spec/work-units.json` (+ epics.json) and reuse
existing shared infra: `WorkUnitsData` (extra-field preservation), `EpicsData`,
`ensure_work_units_file` / `read_work_units_or_empty`, `write_json_atomic`,
`iso8601_now`. NO FOUNDATION.md cross-dependency (deliberately avoided the
foundation event-storm batch which all chain `generateFoundationMd`).

## Current batch

| Slot | RPC ID  | Command                   | Worker session_id | Phase | Notes |
|------|---------|---------------------------|-------------------|-------|-------|
| 1a   | RPC-317 | update-work-unit          | bec19b6b-713e-4f67-803d-139c7bf93ced | A | needs ensureEpicsFile/EpicsData (exists) |
| 1b   | RPC-318 | update-work-unit-estimate | bec19b6b-713e-4f67-803d-139c7bf93ced | A | needs prefill-detection port |
| 2a   | RPC-223 | delete-work-unit          | 736bc5d5-c889-400b-9b5a-9e27afef9dc2 | A |       |
| 2b   | RPC-206 | compact-work-unit         | 736bc5d5-c889-400b-9b5a-9e27afef9dc2 | A | soft-delete compaction |
| 3a   | RPC-255 | prioritize-work-unit      | b0fc6da1-f6db-437b-a6ab-a83dd5c4d5eb | A | reorder IndexMap/priority |
| 3b   | RPC-284 | repair-work-units         | b0fc6da1-f6db-437b-a6ab-a83dd5c4d5eb | A |       |
| 4a   | RPC-264 | record-iteration          | a986b44b-ba8c-4356-a94f-3f1b996f58d9 | A | small (78 LOC) |
| 4b   | RPC-229 | export-work-units         | a986b44b-ba8c-4356-a94f-3f1b996f58d9 | A | small (82 LOC) |
| 5a   | RPC-228 | export-example-map        | a72a547b-5e16-4a77-a5b7-c7990f8c6b38 | A | reads work-units example map |
| 5b   | RPC-227 | export-dependencies       | a72a547b-5e16-4a77-a5b7-c7990f8c6b38 | A | mermaid/json/dot output |

**Cargo Serial Worker session_id:** 44654d35-a886-47c9-bd5c-b201553fa13b

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| W1 | (none) | prefill-detection ported as private helper inside update_work_unit_estimate.rs — APPROVED, no io/ensure.rs change | resolved |
| W3 | dispatch.rs | prioritize/repair stubs at :522/:570 use old single-arg `run(args_json)` → become `run(args_json, project_root)` when moved to run_ported | supervisor Phase C |

## PHASE A reports (all complete 2026-06-12)

- W1 RPC-317 (est 5): 9 rust-port + 3 cli scenarios. RPC-318 (est 5): 7 + 3. Prefill detector as private helper.
- W2 RPC-223 (est 5): 9 + 6. RPC-206 (est 3): 7 + 6. Framing A on compact (TS discards warning).
- W3 RPC-255 (est 5): 10 scenarios. RPC-284 (est 5): 8. position polymorphism (str|num); TS dry-run no-op bug preserved.
- W4 RPC-264 (est 2): 4 + 2. RPC-229 (est 2): 5 + 3. Framing A both (TS shells broken).
- W5 RPC-228 (est 3): 6 + 5. RPC-227 (est 3): 8 + 5. mermaid special-cased, all else→JSON branch.

Total new scenarios: ~96 across 20 feature files. All 1267 feature files validate.

## PHASE C wiring DONE (supervisor, 2026-06-12)

- [x] canonical.rs: added 10 to PORTED_COMMANDS (Batch 12)
- [x] dispatch.rs: added 10 run_ported arms; commented 10 out of run_stub
- [x] commands/mod.rs: modules already declared (were stubs) — no change needed
- [x] help/configs/mod.rs: registered 10 new help configs
- [x] main.rs: 10 Mode variants + 10 forward! arms + 10 intercept arms + 10 mod decls (2250 lines, < 2300 cap)
- [x] cargo_shape.rs: added 10 bridge filenames to both lists; "88"→"98"

Note: W2 (delete/compact) and W5 (export-map/export-deps) ran ahead into PHASE C
during PHASE B — collapsed into one impl pass. W3 (prioritize/repair) needed a
feature-file 1:1-split fix. All 10 impls + bridges + help configs confirmed present.
Awaiting cargo serial worker green run.

## GREEN RUN RESULTS (2026-06-12)

Build fixes applied by supervisor before green:
- regex moved from [dev-dependencies] → [dependencies] in fspec-core/Cargo.toml (W1 prefill uses regex).
- W2 delete_work_unit help config: CommandError → CommonError.
- main.rs: re-added the `#[tokio::main] async fn main()` signature accidentally dropped during enum edit.

Test results — ALL GREEN:
- cargo build --release -p codelet-fspec-core ✓ (1 unused-var warning)
- cargo build --release -p codelet-fspec ✓
- Core dispatcher tests: compact 7, delete 9, export_dependencies 8, export_example_map 6,
  prioritize 9, record_iteration 4, repair 6, update_work_unit 9, update_work_unit_estimate 7
  + dispatcher_test 6 (2 ignored) = 0 failures.
- CLI tests: cli_compact 6, cli_delete 6, cli_export_dependencies 5, cli_export_example_map 5,
  cli_export_work_units 3, cli_prioritize 10, cli_record_iteration 2, cli_repair 4,
  cli_update_work_unit 3, cli_update_work_unit_estimate 3 = 0 failures.
- cargo_shape: 11 passed, 11 ignored, 0 failed (98-file lock-list accepted).

## BATCH 12 COMPLETE (2026-06-12)

All 10 work units → `done`: RPC-317, RPC-318, RPC-223, RPC-206, RPC-255, RPC-284,
RPC-264, RPC-229, RPC-228, RPC-227. @wip→@done flipped on all 20 feature files.

Final regression: core lib + all command tests 208+ passed / 0 failed;
cargo_shape 11/11 (11 ignored); cross_frontend_parity 8/8. All worker sessions
+ cargo serial worker closed. command-port.md batch-12 log row + lessons added.

## Supervisor wiring checklist (Phase C)

- [ ] canonical.rs: add 10 to PORTED_COMMANDS
- [ ] dispatch.rs: add 10 run_ported arms; comment them out of run_stub
- [ ] commands/mod.rs: (modules already registered as stubs — rewrite in place)
- [ ] help/configs/mod.rs: register 10 new help configs
- [ ] main.rs: 10 Mode variants + 10 forward! arms + 10 intercept arms + 10 `mod` decls
- [ ] cargo_shape.rs: add 10 bridge filenames to lock-list + allowed set; "88"→"98"; main_cap 2300→~2600
