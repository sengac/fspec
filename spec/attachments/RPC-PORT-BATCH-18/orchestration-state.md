# Batch 18 Orchestration State (TS → Rust command port)

Supervisor session_id: `82583120-475f-48e5-b54f-93a816f0190d`
Started: 2026-06-16

## Selected commands (10) — all currently `run_stub` (1-arg), unported

| Slot | RPC ID  | Command                                    | TS LOC | Worker | Phase | Notes |
|------|---------|--------------------------------------------|--------|--------|-------|-------|
| W1-a | RPC-202 | checkpoint                                 | 90     | W1     | spec  | wraps codelet_git::ghost_commit::create_ghost_commit |
| W1-b | RPC-203 | cleanup-checkpoints                        | 115    | W1     | spec  | codelet_git delete + count |
| W1-c | RPC-288 | restore-checkpoint                         | 200    | W1     | spec  | codelet_git::restore_ghost_commit |
| W2-a | RPC-225 | discover-event-storm                       | 91     | W2     | spec  | event-storm scaffold on work-units.json |
| W2-b | RPC-232 | generate-example-mapping-from-event-storm  | 213    | W2     | spec  | event-storm → example map transform |
| W3-a | RPC-309 | suggest-dependencies                       | 267    | W3     | spec  | read-only analysis |
| W3-b | RPC-323 | validate-spec-alignment                    | 104    | W3     | spec  | read-only validation |
| W3-c | RPC-276 | remove-init-files                          | 202    | W3     | spec  | file deletion |
| W4-a | RPC-198 | auto-advance                               | 135    | W4     | spec  | work-units.json status transitions |
| W4-b | RPC-326 | workflow-automation                        | 219    | W4     | spec  | read work-units.json |

## Agents

| Role | session_id |
|------|------------|
| Cargo Serial Worker | f9674958-dffa-47be-aa7e-0823f826bcd1 |
| Worker 1 (checkpoints) | 519fbdf7-1030-46b4-9399-cc2d3323348a |
| Worker 2 (event storm) | 7dd009a2-ae16-4975-8271-023426bf109e |
| Worker 3 (analysis/validation/init) | 5f91c954-fd56-44a8-8784-e946fb3f68fb |
| Worker 4 (work-unit status) | 69e983c1-eeb0-473a-a9bc-acdbfd670e92 |

## Key infra notes
- `codelet-git` crate already exposes ghost-commit primitives (create/restore/delete/count) — reference `list_checkpoints.rs` for usage.
- All 10 modules already registered in `commands/mod.rs` (as stubs). NO mod registration needed.
- Help configs: NONE of the 10 registered in `help/configs/mod.rs` — supervisor adds 10.
- main.rs: NO Mode variants exist — supervisor adds 10 variants + forward + intercept + `mod <snake>;`.
- Signature footgun: stubs are 1-arg `run(args_json)`; ported = 2-arg `run(args_json, project_root)`.
  When workers rewrite to 2-arg in PHASE C, `run_stub` breaks. Supervisor wires dispatch.rs
  (move arm to run_ported, comment out run_stub arm) AS SOON AS impls land.
- IPC (sendIPCMessage) in checkpoint TS → treat as no-op in Rust (dispatcher has no TUI IPC).

## Shared-file change requests (pending supervisor action)
| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| W2 | discover_event_storm.rs | inline ~220-line event-storm guidance const | APPROVED inline (no shared edit) |
| W2 | (system-reminder) | inline private wrap_in_system_reminder | APPROVED inline (parity w/ show_work_unit.rs) |
| W2 | gen_example_mapping | inline pascal_case_to_sentence private fn | APPROVED inline |
| W3 | validate_spec_alignment | clap surface = required `<workUnitId>`, `--fix` no-op; help fixture mirrors TS verbatim (Framing A) | APPROVED |
| W3 | validate_spec_alignment | map DirectoryNotFound→empty Vec locally | APPROVED (no shared edit) |
| W3 | remove_init_files | inline local AGENT registry const | APPROVED (init.rs still stub) |
| W3 | remove_init_files | unspecified keepConfig → remove config (false); `--keep-config` overrides | APPROVED |

## Phase status
- PHASE A: COMPLETE (all 20 feature files + estimates). Estimates: RPC-202/203/288=5, others=3, RPC-326=5.
- PHASE B: COMPLETE — all core tests written + RED-confirmed (NotYetPorted).
- PHASE C: COMPLETE — all 10 impls (2-arg) + 10 help configs + 10 bridges written.
  - W1 (checkpoint trio) STALLED twice → closed; replaced by W1b (5143b88f) which finished.
- WIRING: supervisor wired all 10 in canonical.rs + dispatch.rs + help/configs/mod.rs + main.rs;
  cargo_shape bumped (main_cap 3300→3500, lock-list +10 → "159", added 10 bridge filenames).
- GREEN: all 10 core + 10 CLI test binaries pass; cargo_shape 11p/11ign; cross_frontend_parity 8/8.
  Supervisor fix-ups: (a) cleanup_checkpoints core tests pass format:json on JSON-asserting scenarios;
  (b) cli_validate_spec_alignment parity guard strips comment lines before `.feature` substring check;
  (c) removed unused `json` import in cleanup_checkpoints.rs.
- PHASE D: COMPLETE. All 10 work units → `done`. link-coverage applied per scenario.
  Supervisor swapped @wip→removed on the 6 checkpoint feature files (W1b added @done but
  left @wip). Final regression: all 10 core + 10 CLI binaries green; cargo_shape 11p/11ign;
  cross_frontend_parity 8/8. All 5 agent sessions closed.

## BATCH 18 COMPLETE ✅
