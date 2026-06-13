# Batch 13 — Foundation mutation commands (TS → Rust port)

Supervisor session: `ec6ed932-9437-46cf-b981-33df60412dd6` (RESUMED 2026-06-12 11:52)
Prior (killed) supervisor sessions: `54be46a4-2308-40d2-9c29-f6a37f3c7273`, `2c348c21-ee47-4c82-b764-6a0fc1e674ea`

## RESUME NOTE #2 (2026-06-12 11:52)
The second supervisor (54be46a4) respawned the fleet and sent Phase A
kickoffs, then was itself killed before any worker produced output. All its
subordinates (cargo 3cc7e4a5 + workers da073f8e/cb844ddc/f9a4ec6e/d4307c5a/
8aa9a7d7) are now session_not_found. Verified on-disk: all 10 work units
still `specifying`; git tree clean except work-units.json + this attachment;
NO batch-13 feature files/tests/Rust impl on disk. Example-mapping progress
still preserved in work-units.json (RPC-173: 7 rules/6 examples/4 arch notes;
RPC-269: 7 rules only; other 8: none). Respawned cargo worker + 5 workers
with NEW session IDs below and re-sent Phase A kickoffs.

All 10 commands operate on `spec/foundation.json`. Shared infra
(`ensure_foundation_file`, `write_json_atomic`) already exists. Reference
templates: `add_diagram.rs` (foundation mutation), `add_bounded_context.rs`
(event-storm item append).

## Worker → command pairs

| Slot | Worker session_id | RPC pair | Commands | foundation.json target |
|------|-------------------|----------|----------|------------------------|
| 1    | 09b351ba-3f23-4fe5-a508-c7c6d1bf2a26 | RPC-173 / RPC-269 | add-capability / remove-capability | solutionSpace.capabilities (+draft precedence) |
| 2    | 7728d619-a73c-43d8-ab13-36be9209c665 | RPC-186 / RPC-277 | add-persona / remove-persona | solutionSpace.personas (+draft precedence) |
| 3    | 151b3b3a-2630-42a3-9234-27ae8b2888a6 | RPC-183 / RPC-274 | add-foundation-bounded-context / remove-foundation-bounded-context | eventStorm.items (big_picture) |
| 4    | b5d73b62-0cee-4982-8745-9efded28f3d2 | RPC-166 / RPC-266 | add-aggregate-to-foundation / remove-aggregate-from-foundation | eventStorm.items |
| 5    | 98ec4cf8-d39a-4194-ad23-35866943380b | RPC-175 / RPC-270 | add-command-to-foundation / remove-command-from-foundation | eventStorm.items |

Cargo Serial Worker session_id: 4323d7c9-8d2f-42b4-830a-b2784ec64ee3

## Current stub signatures (1-arg) — supervisor MUST rewire dispatch.rs

The 10 commands are currently 1-arg stubs in `dispatch.rs::run_stub`. When a
worker rewrites its core module to the 2-arg `(args_json, project_root)`
signature, `run_stub` stops compiling. CRITICAL LESSON (batch 12): collapse
B+C and wire dispatch.rs as soon as impls land.

## Supervisor wiring checklist (one pass after all impls land)

- [ ] canonical.rs::PORTED_COMMANDS — add 10 entries (Batch 13)
- [ ] dispatch.rs::run_ported — add 10 arms (2-arg, project_root)
- [ ] dispatch.rs::run_stub — REMOVE the 10 arms (comment "ported (Batch 13)")
- [ ] help/configs/mod.rs — register 10 new help configs
- [ ] main.rs — 10 `mod` decls + 10 Mode variants + 10 forward! arms + 10 intercept arms
- [ ] main.rs — bump cap if >2300 (currently 2269 → expect ~2370 → bump cargo_shape main_cap 2300→2500)
- [ ] cargo_shape.rs — lock-list 98→108: add 10 filenames to existence list + allowed set; update "locked 98" message

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| all workers | dispatch.rs | move 10 cmds from run_stub → run_ported (2-arg, project_root) | PENDING (Phase C wiring) |
| all workers | canonical.rs | add 10 to PORTED list | PENDING (Phase C wiring) |
| all workers | help/configs/mod.rs | register 10 help configs | PENDING (Phase C wiring) |
| all workers | fspec/src/main.rs | 10 mod + Mode + forward/intercept arms | PENDING (Phase C wiring) |
| all workers | cargo_shape.rs | lock-list 98→108 + main_cap bump | PENDING (Phase C wiring) |
| W1/W2 | io/locked_file.rs | trailing-newline write helper | ✅ DONE — supervisor added `write_json_atomic_trailing_newline` (compiles, exit 0) |

## Progress log
- 11:52 Fleet respawned (cargo 4323d7c9 + workers 09b351ba/7728d619/151b3b3a/b5d73b62/98ec4cf8).
- ~12:00 PHASE A complete: 20 feature files created + validated (1289 total valid). Estimates set (RPC-173/269/183/274=5; RPC-186=3,277=2; RPC-166/266/175/270=3).
- ~12:08 Supervisor added shared `write_json_atomic_trailing_newline` (W1/W2 byte parity); cargo confirmed fspec-core builds clean.
- ~12:09 AST-research attachments backfilled for all 10 work units; all 10 transitioned specifying→testing.
- ~12:10 PHASE B kickoffs sent to all 5 workers (write failing tests, verify red via cargo runner, link-coverage). Awaiting reports.
- ~12:25 PHASE B complete: all 5 workers red-verified (cap 29, persona 32, fbc 21, agg 30, cmd 18 tests). All 10 → implementing.
- ~12:48 PHASE C owned-files complete: all 5 workers wrote core impls + help configs + CLI bridges; reported exact wiring specs. STOPPED for supervisor wiring pass.
- ~13:00 Supervisor wiring pass DONE: canonical.rs PORTED +10; dispatch.rs run_ported +10 / run_stub −10; help/configs/mod.rs +10; commands/mod.rs `add_foundation_ctx` alias (RPC-183 bridge guard); main.rs +10 mod/Mode/forward!/help-intercept; cargo_shape lock-list 98→108 + main_cap 2300→2500.
- ~13:11 Cargo runner GREEN: build core (0), build cli (0), core tests 75/0, cli tests 58/0, cargo_shape 11/0. Batch 13 fully integrated.
- ~13:15 Coverage dedupe: workers collapsed double testMappings (Phase B test-only + Phase C test+impl) to one FULLY-COVERED mapping per scenario (0 problem scenarios).
- ~13:20 All 10 work units driven validating→done. @wip→@done swapped on all 20 feature files. Auto-checkpoints cleaned up.
- ✅ BATCH 13 COMPLETE. All 10 foundation mutation commands ported + wired + green + done. Subordinate fleet closed.
