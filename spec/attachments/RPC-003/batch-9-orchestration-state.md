# Batch 9 — Orchestration State (RESUMED 2026-06-11 05:02 UTC)

**Supervisor session:** 0f7e0496-7944-4017-bcb8-795772779fe0 (resumed from 3cbd055f-2b0b-42fe-b364-54865fc177f1 which was killed)
**Cargo Serial Worker:** 37cf1ff4-2caa-42c0-b870-579e4b7e1f37 (respawned from beeb7ac2 which was killed)

## Current batch (10 commands, 5 workers) — ✅ COMPLETE (all DONE)

| Slot | RPC IDs | Commands | Worker session_id | Final | Tests Passing |
|------|---------|----------|-------------------|-------|---------------|
| 1 | RPC-177, RPC-196 | add-dependency, answer-question | f5e9766e-c9db-4c82-8117-3583849ee1e2 | DONE ✅ | 37/37 |
| 2 | RPC-289, RPC-291 | restore-example, restore-rule | 752d2aa2-ed24-4a06-9c9e-fbf6820edf71 | DONE ✅ | 32/32 |
| 3 | RPC-290, RPC-287 | restore-question, restore-architecture-note | ea6bcad9-cc1c-4c35-8944-977ccbe6e5a7 | DONE ✅ | 31/31 |
| 4 | RPC-193, RPC-281 | add-tag-to-feature, remove-tag-from-feature | 50c4075a-8486-4c87-b98d-b59dc4782b6e | DONE ✅ | 31/31 |
| 5 | RPC-194, RPC-282 | add-tag-to-scenario, remove-tag-from-scenario | 11086a69-f368-4028-9664-604f0e6f4a17 | DONE ✅ | 32/32 |

## Batch 9 final stats
- **10 commands ported** from TypeScript to Rust
- **163 tests passing** (0 failing)
- **20 feature files** created, all retagged @done
- **Shared-file wiring**: canonical.rs (10 added), dispatch.rs (10 moved stub→ported), help/configs/mod.rs (10 added), main.rs (10 Mode variants + intercepts + forwards)
- **Workspace build**: clean release build, 0 errors, 0 warnings

## Resumption notes
- Prior supervisor (3cbd055f) + workers all killed before any progress past playbook reading.
- New supervisor (0f7e0496) respawned cargo runner + 5 workers and ran the full A→B→C→Validating→Done cycle.


## Phase A summary (Done)
40 feature files written, ~150 scenarios. All tagged @wip + work-unit. All validate green.

## Phase B summary (Done)
~131 dispatcher + CLI tests written (10 test binaries). All red-phase confirmed via cargo runner — NotYetPorted stubs fire. Help fixtures captured byte-for-byte from `node dist/index.js`. All coverage linked.

## Phase C plan
Workers do steps 1-5 (impl files + bridge files + cargo build core), then PAUSE.
Supervisor then wires shared files for ALL 10 commands at once (canonical.rs, dispatch.rs, commands/mod.rs, help/configs/mod.rs, main.rs).
Workers then resume to run green-phase tests + link impl coverage.

## Shared-file wiring (supervisor TODO after workers report Phase C step 5)

| File | Edit |
|------|------|
| codelet/fspec-core/src/canonical.rs | Add 10 commands to PORTED_COMMANDS |
| codelet/fspec-core/src/dispatch.rs | Add 10 run_ported match arms; remove from run_stub |
| codelet/fspec-core/src/commands/mod.rs | Confirm pub mod for all 10 (stubs already declared) |
| codelet/fspec-core/src/help/configs/mod.rs | `pub mod add_dependency; pub mod answer_question; pub mod restore_example; pub mod restore_rule; pub mod restore_question; pub mod restore_architecture_note; pub mod add_tag_to_feature; pub mod remove_tag_from_feature; pub mod add_tag_to_scenario; pub mod remove_tag_from_scenario;` |
| codelet/fspec/src/main.rs | Add 10 Mode variants + 10 `mod` decls + 10 forward arms + 10 intercept_ts_help arms |


## Phase A summary (40 feature files written, ~150 scenarios)
All 10 commands have rust-port + cli-subcommand feature files tagged @wip. All validate green.

## Shared-file change requests (pending supervisor action in Phase C)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| Slot 4 (RPC-193, RPC-281) | codelet/fspec-core/src/canonical.rs | Add add-tag-to-feature + remove-tag-from-feature to PORTED_COMMANDS | pending |
| Slot 4 | codelet/fspec-core/src/dispatch.rs | Wire run_ported match arms for both | pending |
| Slot 4 | codelet/fspec-core/src/help/configs/mod.rs | `pub mod add_tag_to_feature; pub mod remove_tag_from_feature;` | pending |
| Slot 4 | codelet/fspec/src/main.rs | Mode::AddTagToFeature + Mode::RemoveTagFromFeature variants + intercept | pending |
| Slot 5 (RPC-194, RPC-282) | codelet/fspec-core/src/canonical.rs | Add add-tag-to-scenario + remove-tag-from-scenario | pending |
| Slot 5 | codelet/fspec-core/src/dispatch.rs | Wire run_ported match arms | pending |
| Slot 5 | codelet/fspec-core/src/help/configs/mod.rs | Add new modules | pending |
| Slot 5 | codelet/fspec/src/main.rs | Add Mode variants + intercept | pending |
| Slot 5 (deferred) | codelet/fspec-core/src/io/gherkin_tags.rs (NEW) | Shared is_work_unit_tag/is_regular_tag predicates — slot 5 will inline-duplicate in Phase C and request extraction later | deferred |
| Slots 1, 2, 3 | (none) | — | n/a |

## Common pattern observations
- All workers documented that TS help files advertise `--ids` flag but TS Commander.js doesn't wire it. Rust ports mirror Commander shape (positional-only, `--ids` in help text only).
- All slots 1-3 reuse existing io::ensure/io::locked_file/io::time infrastructure verbatim.
