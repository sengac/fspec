# Batch 19 orchestration — port 9 complex commands (TS → Rust)

Supervisor session: `bde16e89-698a-4f87-8b04-748f1a89b967`
Started: 2026-06-17

## Scope (confirmed with user)
- 9 STORY cards under RPC-003 umbrella. RPC-003 itself is the epic umbrella (excluded).
- 3 BUG cards (RPC-328/329/330) deferred to a later stage.
- RPC-319 (update-work-unit-status, 1404 LOC) supervised SOLO by supervisor (not in worker batch).
- 8 stories distributed across 4 workers (2 each), 1 agent reserved as cargo serial worker.

## Constraints
- ONLY the cargo serial worker runs cargo build/test/clippy or invokes the binary. All others route via it.
- Workers never edit shared files (canonical.rs, dispatch.rs, commands/mod.rs, help/configs/mod.rs, main.rs, Cargo.toml, io/ensure.rs).
- Current stubs are 1-arg `run(args_json)`. Rewriting to 2-arg breaks dispatch.rs::run_stub for everyone → supervisor wires shared files per wave once impls land.

## Roster
| Slot | Role | session_id |
|------|------|------------|
| Cargo | Cargo Serial Worker | 7b92204f-131a-459f-a71f-1a6135ba1c9d |
| W1 | Worker | 2e241c69-7679-4cea-b16d-9f197b7754e7 |
| W2 | Worker | 3c959a7c-80fd-4144-aa93-bb816f849850 |
| W3 | Worker | 81395d45-5f2d-4fa7-a708-b145d3b0e662 |
| W4 | Worker | 2d83ddf2-72bd-4a59-a730-1a22b6e0ceb4 |

## Assignments (waves to manage the 1-arg→2-arg signature coupling)
| Worker | Wave 1 card | Wave 2 card |
|--------|-------------|-------------|
| W1 | RPC-294 reverse | RPC-200 bootstrap |
| W2 | RPC-226 discover-foundation | RPC-239 init |
| W3 | RPC-234 generate-scenarios | RPC-285 report-bug-to-github |
| W4 | RPC-286 research | RPC-295 review |
| Supervisor (solo) | RPC-319 update-work-unit-status | — |

## Architectural watch-items
- research (RPC-286): TS spawns child_process for research tools → use blocking std::process (resolves first poll, no real tokio .await).
- report-bug-to-github (RPC-285): builds GitHub issue URL (no network fetch) — confirm no real async.
- update-work-unit-status (RPC-319): hooks executor + git checkpoints + temporal validation + IPC. IPC is NO-OP in Rust (per Batch 18). Reuse codelet_git ghost_commit + hooks infra.

## Status log
- specifying: all 8 worker cards + (RPC-319 pending supervisor).

## RESUME 2026-06-17 (after host process killed)
Original supervisor session `bde16e89` and its 4 workers + cargo runner died with the host node process (no self-kill detected — no pkill/kill/killall ran in any Batch-19 session; 121 GiB RAM, no OOM; fspec.log shows a JS→Rust `FspecResult.data` conversion error at 06:24 but the process simply went silent ~06:32). Old in-memory agent loops are unrecoverable; persisted fspec state + this file are the recovery anchors.

NEW supervisor session: `019f161f-7271-44b0-8dbd-0f3cf06bf2f7`.
Smaller fleet chosen by user: 2 workers now (Phase A needs no cargo); cargo serial runner to be spawned at Phase B/C.

NEW roster:
| Slot | session_id | Wave-1 card | Wave-2 card |
|------|-----------|-------------|-------------|
| WA | 44378b0a-0ace-415e-b624-718fe9a01beb | RPC-294 reverse (research file already exists) | RPC-234 generate-scenarios |
| WB | fcc504f7-34fd-46fd-ad60-91be11f306c0 | RPC-226 discover-foundation | RPC-286 research |
| Supervisor (solo) | 019f161f | RPC-319 update-work-unit-status | — |

Progress carried over: RPC-294 ast-research-reverse.md (complete, 6 shared-file changes flagged); RPC-319 had user story + 4 rules + 1 arch note from old supervisor.

RPC-319 Phase A COMPLETE (supervisor): user story, 4 rules, 11 examples, 1 arch note, estimate 13; features update-work-unit-status-rust-port.feature (13 scenarios) + update-work-unit-status-cli-subcommand.feature (8 scenarios), tagged @wip @cli @work-unit-management @rust, validated.

## Phase A reports (all idle at wave gate)
- WA RPC-294 reverse: 15 rules / 12 examples, est 8; reverse-rust-port (18) + reverse-cli-subcommand (7); @wip @rust @cli @querying; validated.
- WB RPC-226 discover-foundation: 10 rules / 6 examples, est 8; discover-foundation-rust-port (9) + discover-foundation-cli-subcommand (5); @wip @cli @foundation-management @rust; validated.
- Supervisor RPC-319: as above.

## Supervisor decisions (2026-06-17)
- RPC-226 Decision A (field-reminder reuse): CHOSE (ii) COPY scanDraftForNextField/generateFieldReminder/extract_detected_value/agent_supports_meta_cognition/is_known_agent into discover_foundation.rs — isolated, parallel-safe for this batch. Follow-up tech-debt card to extract a shared foundation field-reminder module post-batch (avoid the ~150 LOC dup long-term).
- RPC-226 Decision B (FOUND auto-unit): APPROVED inline-build of the FOUND task in discover_foundation.rs (mirror create_story.rs: idempotent on FOUND- id, reuse create_prefix::run swallowing already-exists, push to states.backlog + prefixCounters), whole block best-effort/swallow per TS try/catch. No shared create-work-unit helper for now.
- Shared-file change requests (canonical.rs add "discover-foundation"; dispatch.rs:634 1-arg→2-arg run_ported; main.rs Mode::DiscoverFoundation variant+forward+intercept+mod; help/configs/mod.rs register) — ACKNOWLEDGED, supervisor will apply during Phase C wiring. Same pattern queued for RPC-294 reverse (6 flags in ast-research-reverse.md §8).
- RPC-294 Decision 6 (Strategy-D implementationContext): CONFIRMED dispatcher-JSON-only — verified reverse.ts registers only --strategy/--continue/--status/--reset/--complete/--dry-run (lines 675-683); implementationContext used at line 119 but never registered as a Commander option. CLI bridge will NOT expose it; parity with TS preserved. Dispatcher-only scenario in reverse-rust-port.feature covers the persona path.
- RPC-294 shared-file requests (1-5: dispatch.rs reverse arm 1-arg→2-arg; fspec-core/Cargo.toml add sha2; types/mod.rs pub mod reverse_session; help/configs/mod.rs pub mod reverse; main.rs mod reverse + Mode::Reverse variant+forward+intercept) — ACKNOWLEDGED, supervisor applies during Phase C wiring.

## GATE: Phase B/C NOT yet authorized — awaiting human go-ahead on build-heavy phase. Cargo serial runner NOT yet spawned (deferred per smaller-fleet choice). Workers parked.

## PHASE B AUTHORIZED by human (2026-06-17) — "yes, continue (everything ACDD too)"
- Cargo serial runner spawned: 08cf4ed4-7edb-4929-b24c-529850475716 (ONLY agent that builds; baseline build requested).
- WA released to Phase B (RPC-294 reverse): write failing dispatcher+CLI tests, help config, help fixture, type module; link-coverage; red confirm; STOP before Phase C.
- WB released to Phase B (RPC-226 discover-foundation): same red-phase plan.
- RPC-319 delegated to NEW worker WC: 53b1c2f4-4822-44e0-aad2-f1485fea790a. Rationale: 1404 LOC / 13 pts is too heavy to inline in supervisor context; supervisor retains coordination + shared-file wiring. Card already in TESTING (research attached: ast-research-update-work-unit-status.md; KEY finding §6 — NO ported fspec-command hooks executor exists, WC must build a minimal blocking one).
- Supervisor owns ALL shared-file wiring in lockstep during Phase C: dispatch.rs (3 cards 1-arg→2-arg), canonical.rs PORTED_COMMANDS (+3), main.rs (3 Mode variants+forward+intercept+mod), types/mod.rs (reverse_session), help/configs/mod.rs (+3), Cargo.toml (sha2 for reverse).
- ACDD red-phase rule: tests must COMPILE against current 1-arg stubs and FAIL on assertions (NotYetPorted). Supervisor does minimal mod-wiring only where needed for compile.

## Phase B progress
- WB RPC-226: tests written + coverage linked (rust-port 9/9, cli 5/5 = 100%). Files: tests/discover_foundation.rs (8 dispatcher), cli_discover_foundation.rs (5+1 two-front-doors), help/configs/discover_foundation.rs (latent until mod wired), fixtures/help/discover-foundation.txt. Red-phase cargo run relayed to runner. Note: help config mod NOT wired yet (latent, won't break test build). No TYPICAL WORKFLOW section in fixture (TS formatCommandHelp doesn't render workflow array).
- WA RPC-294 / WC RPC-319: still writing red-phase tests.

## RED PHASE CONFIRMED — RPC-226 (2026-06-17)
Both binaries compiled clean; 0 compile-error blockers. core discover_foundation: FAILED 0 passed/8 failed (4 on NotYetPorted stub msg, 4 on empty-data JSON EOF parse). cli_discover_foundation: FAILED 0 passed/6 failed (5 "unknown command 'discover-foundation'" + 1 dispatcher NotYetPorted). All assertion-level, no symbol panics. Clean ACDD red. WB parked at Phase C gate.

## EFFICIENCY NOTE: workers requested `cargo test --release` — CLI test binary took 24m08s to compile (release). For green-phase iteration use DEBUG builds (drop --release) to cut compile time; reserve --release for a final pass. Will instruct workers/runner accordingly.

## SHARED WIRING APPLIED (compile-only, incremental)
- RPC-294 red-phase compile: types/mod.rs += `pub mod reverse_session;`; fspec-core/Cargo.toml [dependencies] += `sha2 = { workspace = true }` (workspace already pins sha2=0.10). Verified io::project_root::find_project_root + io::time::iso8601_now exist (reverse_session.rs deps). These are additive/isolated — do not affect RPC-226 or RPC-319.
- WA reverse red-phase cargo run relayed (DEBUG builds now).
- WC RPC-319: still writing red-phase tests at last check.

## Phase B test-writing COMPLETE for all 3 cards (2026-06-17)
- WB RPC-226: red confirmed (8 core + 6 cli FAIL clean). Parked at Phase C gate.
- WA RPC-294: tests written (18 core + 7 cli), 100% coverage linked. Compile-wiring applied (reverse_session mod + sha2). Red-phase cargo run relayed (debug).
- WC RPC-319: tests written (13 core + 8 cli), 21/21 coverage linked. help config + fixture written (note: TS help renders 3 option flags as literal `undefined` — WC asked to confirm it's live TS behaviour before enshrining in byte-exact fixture). Red-phase cargo run QUEUED on runner after WA. Parked at Phase C gate.
- Cargo runner order: reverse (running) → update_work_unit_status (queued). DEBUG builds. Package names: codelet-fspec-core / codelet-fspec.
- Pending supervisor Phase-C wiring queue: dispatch.rs (reverse, discover-foundation, update-work-unit-status all 1-arg→2-arg run_ported), canonical.rs PORTED_COMMANDS +3, main.rs 3 Mode variants+forward+intercept+mod, help/configs/mod.rs +3 (reverse, discover_foundation, update_work_unit_status). reverse_session mod + sha2 already done.

## ALL THREE RED PHASES CONFIRMED (2026-06-17) — debug builds fast
- RPC-294 reverse: 18 core + 7 cli FAIL clean.
- RPC-226 discover-foundation: 8 core + 6 cli FAIL clean.
- RPC-319 update-work-unit-status: 13 core + 8 cli FAIL clean.
All assertion-level (NotYetPorted / unknown clap subcommand / side-effect), zero compile blockers, no symbol panics.

## PHASE C IN PROGRESS — all 3 workers green-lit to implement (parallel, own files only)
Strategy: workers implement their command (rewrite stub to 2-arg run(args_json, project_root) + bridge + help config) in their OWN files. Supervisor does ALL shared wiring in ONE atomic pass once all 3 report done, then ONE combined build + 3 test suites.
Shared wiring TODO (supervisor): dispatch.rs move 3 arms from run_stub→run_ported (lines 634 discover-foundation, 741 reverse, 782 update-work-unit-status; replace stub arms with 'handled by run_ported' comment); canonical.rs PORTED_COMMANDS (line 847 list) += reverse, discover-foundation, update-work-unit-status; main.rs 3 Mode variants + forward! + intercept + mod; help/configs/mod.rs += 3 mods. Plus any NEW module mod-line WC requests for a hooks executor.
Already done: types/mod.rs reverse_session; fspec-core Cargo.toml sha2.

## Note: RPC-319 help `undefined` flag quirk VERIFIED by WC (2026-06-17)
WC re-ran `node dist/index.js update-work-unit-status --help` live: OPTIONS block prints literal `undefined` for all 3 option flags (TS help config uses `name` not `flag`; formatter reads opt.flag → undefined). Fixture byte-identical to live (3286 bytes). Genuine current TS behaviour, NOT a bug we're enshrining — byte parity wins. Documented as parity quirk in help/configs/update_work_unit_status.rs lines 7-14 + 33-34.

## Inbound message note: several STALE/replayed worker reports (WA red-phase + cargo-pkg-name correction; WC Phase-B-done + ACK) arrived out-of-order AFTER their work was already handled. Verified via file-state + get_status each time (pending_messages, stub-vs-impl). All 3 workers confirmed actively in Phase C implementing; no duplicate instructions sent. Supervisor shared-wiring still pending until all 3 impls land.
