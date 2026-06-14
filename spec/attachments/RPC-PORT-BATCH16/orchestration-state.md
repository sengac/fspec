# Batch 16 — TS→Rust Command Port Orchestration

Supervisor session: `76156a4c-a8c7-4fe9-826c-368cb2c249e7`
Started: 2026-06-14
Topology: 4 workers + 1 cargo serial worker (user constraint: 5 agents, 1 reserved for cargo)
Project root: /home/rquast/projects/fspec

## Selected batch (10 commands)

Theme: validation + search + coverage + generators/tag-rename. All reuse
existing shared infra (types::tags, types::coverage, io::feature_glob,
io::gherkin, io::coverage_glob, types::work_unit) — minimal new shared types.

| Slot | RPC ID  | Command                  | Worker | Phase | Notes |
|------|---------|--------------------------|--------|-------|-------|
| 1a   | RPC-324 | validate-tags            | W1     |       | reuse tags + feature_glob + gherkin |
| 1b   | RPC-325 | validate-work-units      | W1     |       | reuse work_unit types |
| 1c   | RPC-322 | validate-hooks           | W1     |       | reads fspec-hooks.json |
| 2a   | RPC-321 | validate-foundation-schema | W2   |       | reads foundation.json (Ajv→hand-rolled) |
| 2b   | RPC-320 | validate                 | W2     |       | gherkin validation; reuse io::gherkin + feature_glob |
| 3a   | RPC-297 | search-scenarios         | W3     |       | reuse feature_glob + gherkin |
| 3b   | RPC-296 | search-implementation    | W3     |       | reuse coverage_glob + coverage types |
| 3c   | RPC-311 | unlink-coverage          | W3     |       | coverage sidecar mutation |
| 4a   | RPC-236 | generate-tags-md         | W4     |       | generator (analog generate-foundation-md, done) |
| 4b   | RPC-293 | retag                    | W4     |       | tag rename across registry + features |

## Session IDs

- Cargo Serial Worker: `f1c7045c-2935-4118-985a-3c65b2af0c0c`
- W1 (validate-tags / validate-work-units / validate-hooks): `4fa5233c-6c47-4784-85f3-47b71720f36d`
- W2 (validate-foundation-schema / validate): `33280468-d27d-459e-962c-b28a5907ccf0`
- W3 (search-scenarios / search-implementation / unlink-coverage): `fdd662b5-1bfb-4026-b485-ec03c83d64d4`
- W4 (generate-tags-md / retag): `7b6b7414-886b-4163-b374-23065be00a47`

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| W1/W2/W3/W4 | canonical.rs, dispatch.rs, mod.rs×2, main.rs | Standard wiring: +10 PORTED_COMMANDS, +10 run_ported arms, −10 run_stub arms, register modules/help-configs, Mode variants + forward! + intercept + mod decls | Phase C |
| W1 RPC-324 | io/ensure.rs | possible null-parity work-units loader + non-throwing feature glob (confirm exact sig in Phase B/C) | pending |
| W1 RPC-325 | io/ensure.rs | raw-Value work-units loader (validate must see invalid statuses, so cannot use typed WorkUnitStatus enum) | pending |
| W2 RPC-321 | (none) | reuses existing generators::foundation_schema native validator — no JSON-schema crate | OK |
| W2 RPC-320 | (none) | gherkin validation via io::gherkin + feature_glob; RPC-329 divergence captured as note (non-blocking) | OK |
| W3 ×3 | dispatch/canonical | stub signature run(args_json) → run(args_json, project_root); all have rich -help.ts | Phase C |
| W4 RPC-293 | (none) | retag faithful-to-TS: touches ONLY feature files (NOT tags.json); flag surface --from/--to/--dry-run | OK (accepted) |
| W4 RPC-236 | (none) | mirrors generate_foundation_md.rs shape | OK |

## Decisions (supervisor)
- D1: validate-hooks (RPC-322) = Framing A (TS shell discards result); Rust implements help-doc canon with core run returning {valid, exitCode, message}. APPROVED.
- D2: validate (RPC-320) exit codes 0/1/2 transported via envelope `{valid, exitCode}`; CLI bridge maps to process exit. APPROVED.
- D3: retag (RPC-293) faithful-to-TS (feature files only, flag surface). APPROVED.
- D4: io/ensure.rs raw-Value loaders: workers may read raw JSON directly in impl; if a shared helper is cleaner, specify exact fn signature in Phase C report and supervisor adds it during the wiring pass.

## Phase log

- Phase A (specifying): ✅ DONE — all 10 commands, ~20 feature files, validated + @wip.
- Phase B (testing): ✅ DONE — all 10 moved to `testing`; RED confirmed (NotYetPorted) by cargo runner; help fixtures captured; coverage linked (test-file only).
- Phase C (implementing): ✅ DONE. Batch built GREEN (12m33s). First test pass: 17/20 green + both guards green. 3 RED were test-data/expectation bugs (NOT impl):
  - validate_work_units (10c+3cli): impl loaded typed WorkUnitsData → rejected fixtures missing createdAt before raw checks ran. Fixed: read raw Value. → 12c/4cli GREEN.
  - unlink_coverage (1c): test asserted unreachable error branch; TS fires "Must specify --all or --test-file" first. Fixed test → 10c GREEN.
  - generate_tags_md (1c): test wrongly copied foundation_md "no trailing newline"; tags-md join('\n') DOES end with newline. Fixed test → 4c GREEN.
  All re-runs GREEN. Workers did combined link-coverage (test+impl in one entry per scenario).
  LESSON: a worker reusing `ensure_*_file` typed loaders for a VALIDATION command defeats the purpose — validators must read raw Value so malformed data reaches the checks.

## Phase C supervisor wiring checklist (NEW files workers create; I register)
- generators/mod.rs: register `pub mod tags_md;` (+ tags schema validator) for RPC-236 (W4 creates the files)
- canonical.rs PORTED_COMMANDS += 10
- dispatch.rs run_ported += 10 arms (2-arg); run_stub: remove validate/validate-tags/validate-work-units/validate-hooks/validate-foundation-schema/search-scenarios/search-implementation/unlink-coverage/generate-tags-md/retag arms
- commands/mod.rs: modules already exist (stubs) — no add needed
- help/configs/mod.rs: register up to 10 new help configs
- main.rs: 10 Mode variants + forward! arms + intercept arms + mod decls
- cargo_shape.rs: lock-list +10, bump main_cap if needed

## Supervisor-owned shared files (wire in Phase C)

- codelet/fspec-core/src/canonical.rs (PORTED_COMMANDS +10)
- codelet/fspec-core/src/dispatch.rs (run_ported +10 arms; run_stub -10 arms)
- codelet/fspec-core/src/commands/mod.rs
- codelet/fspec-core/src/help/configs/mod.rs
- codelet/fspec/src/main.rs (Mode variants + forward! + intercept + mod decls)
- codelet/fspec/tests/cargo_shape.rs (lock-list + main_cap bump)

## Phase log

- Phase A (specifying): not started
- Phase B (testing): not started
- Phase C (implementing): not started
