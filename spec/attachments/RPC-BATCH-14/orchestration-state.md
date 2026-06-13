# Batch 14 Orchestration State (TS → Rust command port)

## RESUMED 2026-06-13 (3rd resume) — supervisor 1033f35f + its fleet were KILLED again.
ROOT CAUSE (confirmed): firing all 5 workers + cargo runner to generate in parallel spikes
memory/CPU on the `node fspec` host process and it gets OOM-killed/crashes (deaths happened
immediately after the parallel Phase-A kickoff, when NO cargo was even running). Mitigation
this run: WAVE-BASED concurrency — at most 2 workers generating at once; cargo runner DEFERRED
until Phase B. NEVER run kill/pkill/cargo from any agent.

New supervisor session_id: `b34f180a-aa2a-4daa-a395-67a467b047d6` (active 2026-06-13)
DEAD supervisor session_ids: `1033f35f-...`, `a23a8ae6-...`, `3aa7d9cd-...`, `39a3d9bd-...`
Cargo Serial Worker session_id: 944f7be5-feca-4e9a-ae1e-3c1156615799 (PHASE B — online 2026-06-13). (dead: d89013ae/d32d2675/9ace4e87/dcc38f55)
No Rust code lost (codelet/ clean, last commit f1855e3b). All 10 RPC units still in `specifying`.
fspec host PID 1566596 healthy, ~115Gi mem available. RPC-180 already has feature files + rules.

## Wave plan (2 workers concurrent)
- Wave 1: Slot 1 (RPC-191/RPC-280) + Slot 2 (RPC-254/RPC-292)
- Wave 2: Slot 3 (RPC-180/RPC-272) + Slot 4 (RPC-224/RPC-237)
- Wave 3: Slot 5 (RPC-312/RPC-208)
After each wave: await_idle → collect report → next wave.

## Current batch — 10 commands, 5 slots (2 commands each)

| Slot | Worker session_id | RPC IDs | Commands | Phase | Notes |
|------|-------------------|---------|----------|-------|-------|
| 1 | 73e7d701-830e-4a61-8519-21358bec3fd4 | RPC-191, RPC-280 | add-schedule, remove-schedule | C DONE (impl clean; only dispatch wiring left) | schedules.json mutation |
| 2 | da5bfad3-c181-4507-9430-36f70019b7d0 | RPC-254, RPC-292 | pause-schedule, resume-schedule | C DONE (impl clean; only dispatch wiring left) | schedules.json status toggle |
| 3 | 7c07da99-0a47-44d4-ad84-736dac42f8d3 | RPC-180, RPC-272 | add-domain-event-to-foundation, remove-domain-event-from-foundation | C DONE (impl clean; dispatch wiring left) | foundation.json eventStorm |
| 4 | a94f7f92-1e4b-4f98-9f01-07e0ad1e0e6f | RPC-224, RPC-237 | dependencies, get-scenarios | C DONE (impl clean; dispatch wiring left; stalled 2x, completed after directive) | read-only |
| 5 | 08954b61-e8a2-411e-9760-bb5fc2fcaeba | RPC-312, RPC-208 | update-foundation, configure-tools | C DONE (impl clean; dispatch wiring left) | foundation.json + tools config json |

## PHASE C IMPL COMPLETE (2026-06-13) — all 10 command modules real-impl, ZERO errors inside modules
Every worker confirmed: the only build errors are E0061 at shared dispatch.rs 1-arg call sites (the run_stub arms).
SUPERVISOR WIRING NOW IN PROGRESS (b34f180a owns these): Cargo.toml (croner/chrono-tz DONE) → dispatch.rs run_ported
arms + remove run_stub arms (10 cmds) → canonical.rs PORTED_COMMANDS (10) → help/configs/mod.rs (10 pub mod + registry)
→ main.rs (10 Mode variants + forward! + intercept_ts_help + mod decls). Then cargo runner green run per-crate.
NOTE on signatures: foundation twins (RPC-180/272) use `async fn run(...) -> Result<String,_>`; schedule + read-only +
foundation-update use `fn run(...) -> Result<Value,_>`. Wire each per its actual signature (check before adding arm).

## SUPERVISOR WIRING DONE (2026-06-13) — both crates compile green
All 10 commands wired: Cargo.toml (croner/chrono-tz), dispatch.rs (run_ported arms + run_stub removed),
canonical.rs PORTED_COMMANDS, help/configs/mod.rs (10 pub mod), main.rs (10 mod + 10 Mode + 10 forward! + 10
intercept_ts_help), cargo_shape.rs locked-list bumped 109→119. `cargo build --release -p codelet-fspec-core`
and `-p codelet-fspec` BOTH compile (1 harmless dead_code warning: get_scenarios core `format` field unused —
CLI handles format, non-blocking). NEXT: cargo runner GREEN test pass (10 core + 10 CLI test binaries).

## PHASE C GREEN (2026-06-13) — ALL 10 COMMANDS PASS
Core dispatcher tests: 54/54 green (10 binaries). CLI integration tests: 43/43 green (10 binaries).
One fix during green run: worker da5bfad3's pause/resume bridges hardcoded the success-message literal, tripping
their own thin-bridge guard. Fixed in owned files — bridges now print the `message` field RETURNED by fspec_core
(single source of truth). Re-verified 16/16 green. fspec host PID 1566596 alive through entire Phase C (wave-based
+ serial cargo runner held — no OOM/crash). Total batch-14 GREEN: 97 tests across 20 binaries.
NEXT: validating (validate + validate-tags + show-coverage) → done; @wip→@done tag swap on 20 feature files.

## PHASE B COMPLETE (2026-06-13) — all 10 RPC units in `testing`, RED confirmed by cargo runner 944f7be5
~80 failing tests across 20 test binaries + 10 help fixtures. All COMPILE; all fail for correct red-phase
reasons (core: NotYetPorted stub; CLI: unknown command / absent bridge module). All scenarios link-covered
(test mappings; impl mappings deferred to Phase C). fspec host PID 1566596 stayed alive through all 3 Phase-B
waves (wave-based 2-concurrent + serial cargo runner held — no OOM/crash). Baseline core build green (exit 0).
NEXT: Phase C (IMPLEMENTING) — workers write isolated impl + CLI bridge + help config; then SUPERVISOR wires
shared files (canonical/dispatch/mod/help-mod/main.rs) per the change-request table; then cargo runner green run.

## PHASE A COMPLETE (2026-06-13) — all 10 RPC units spec-complete via 3 waves (2-concurrent)
All 20 feature files created/validated (`fspec validate` = all 1310 valid). fspec host PID 1566596 stayed
alive through all waves (wave-based concurrency fix held — no OOM/crash). All units still in `specifying`.
Estimates: RPC-191=5, RPC-280=3, RPC-254=3, RPC-292=3, RPC-180=3, RPC-272=3, RPC-224=3, RPC-237=5, RPC-312=5, RPC-208=3.
NEXT: Phase B (TESTING) requires the cargo serial runner — spawn it then, still 2-worker waves. Awaiting user go-ahead.
Wave-3 decisions: RPC-312 deferred deps (discover_foundation/validate_foundation_schema still stubs) — spec'd around them;
RPC-208 reconfigure-message-not-wrapped TS behavior spec'd bug-for-bug (confirm at Phase B).
| 5 | (wave3 — not yet spawned) | RPC-312, RPC-208 | update-foundation, configure-tools | pending | foundation.json + tools config json |

## Reference ports (already done — copy the shape)
- schedule: list_schedules.rs (schedules.json as IndexMap<String, Value>)
- foundation event-storm: add_command_to_foundation.rs / remove_command_from_foundation.rs
- read-only work-units: query_dependency_stats.rs, export_dependencies.rs
- read-only gherkin: show_feature.rs, show_acceptance_criteria.rs, io/gherkin.rs
- foundation update: batch 13 add/remove foundation cmds; show_foundation.rs

## Shared-file change requests (pending supervisor action)
| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| RPC-191 | codelet/fspec-core/Cargo.toml | add `croner`, `chrono-tz` (workspace deps) for cron/TZ validation parity | Phase C |
| RPC-191/280 | new types/schedule.rs + types/mod.rs | home for `SchedulesData { version, schedules: IndexMap<String,Value>, #[serde(flatten)] extra }` | DECIDED: create types/schedule.rs at Phase C |
| RPC-191/280 | io/ensure.rs | add `ensure_schedules_file()` (auto-create) + `schedules_file_path()` | Phase C |
| RPC-191/254/280/292 | canonical.rs, dispatch.rs, help/configs/mod.rs, main.rs | standard wiring (PORTED_COMMANDS, run_ported arms, help config, Mode/forward!/intercept/mod) | Phase C (supervisor batch) |

## Supervisor decisions
- RPC-292/254 missing-file divergence: APPROVED — Rust returns clean `"Schedule '<name>' does not exist"` instead of mirroring the TS TypeError crash. Scenarios may assert the clean error.
- SchedulesData lives in new `codelet/fspec-core/src/types/schedule.rs` (NOT inline per command); ensure/path helpers go in `io/ensure.rs`. Supervisor wires these at Phase C.
- RPC-180/272 `generate_foundation_md::regenerate` conflict: APPROVED to CALL regenerate, matching the already-ported sibling twins `add_command_to_foundation.rs`/`remove_command_from_foundation.rs` (TS parity + consistency win over the RPC-178 skip note).
- RPC-180 feature naming: the prior `add-domain-event-*` files belong to already-DONE RPC-179 (work-units variant); RPC-180 correctly uses distinct `add-domain-event-to-foundation-*` files. Leave RPC-179 files untouched.
- RPC-312 D1 (discover_foundation stub → draft-path systemReminder chaining not reproduced): ACCEPTED as deferred-dep divergence — cannot reproduce behavior gated on an unported command. Parity-note it; revisit when discover_foundation lands.
- RPC-312 D2 (validate_foundation_schema stub → final-path schema gate deferred): ACCEPTED — write+regenerate-MD without the schema check, parity-noted. Revisit when validate_foundation_schema lands.
- RPC-312 signature: project_root-ONLY (2-arg `run(args_json, project_root)`); derive draft path as `spec/foundation.json.draft`. TS CLI never passes draftPath, so no override param. CONFIRMED.
- RPC-208 D3 (installAgentFiles/init template regen not ported → deferred): ACCEPTED — config-write parity preserved, template regen parity-noted. Revisit when init lands.
- RPC-208 D4 (TS bug: reconfigure message not wrapped in <system-reminder>): CONFIRMED bug-for-bug parity. Add a `// TODO(parity-bug RPC-208-D4): TS passes cwd string as AgentConfig; reconfigure msg unwrapped — preserved for byte-parity` comment at the site + note in feature docstring so it is not silently lost.

## Supervisor wiring checklist (Phase C)
- [ ] canonical.rs: add 10 names to PORTED_COMMANDS
- [ ] dispatch.rs::run_ported: add 10 arms; remove 10 from run_stub
- [ ] commands/mod.rs: register 10 modules (already declared as stubs — confirm)
- [ ] help/configs/mod.rs: register 10 help configs
- [ ] main.rs: 10 Mode variants + 10 forward! arms + 10 intercept arms + 10 `mod` decls
- [ ] cargo_shape.rs: bump lock-list + main_cap if needed
