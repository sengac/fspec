# Batch 17 — TS→Rust Command Port Orchestration

Supervisor session: `a7fa957d-ca9e-47fd-ad81-fc0cce368c23`
Started: 2026-06-15

## Batch (10 commands across 4 workers + 1 cargo serial worker)

| Slot | RPC ID | Command | TS source | Worker | Phase | Notes |
|------|--------|---------|-----------|--------|-------|-------|
| 1 | RPC-197 | audit-coverage | src/commands/audit-coverage.ts | W1 | - | |
| 2 | RPC-199 | board | src/commands/display-board.ts | W1 | - | |
| 3 | RPC-201 | check | src/commands/check.ts | W1 | - | |
| 4 | RPC-207 | compare-implementations | src/commands/compare-implementations.ts | W2 | - | |
| 5 | RPC-220 | delete-scenarios | src/commands/delete-scenarios-by-tag.ts | W2 | - | |
| 6 | RPC-230 | format | src/commands/format.ts | W2 | - | |
| 7 | RPC-231 | generate-coverage | src/commands/generate-coverage.ts | W3 | - | |
| 8 | RPC-240 | link-coverage | src/commands/link-coverage.ts | W3 | - | |
| 9 | RPC-235 | generate-summary-report | src/commands/generate-summary-report.ts | W4 | - | |
| 10 | RPC-238 | import-example-map | src/commands/import-example-map.ts | W4 | - | |

## Sessions

NOTE: Supervisor session a7fa957d died ~19:49 during Phase B kickoff; original
worker runtimes also died (~19:58). RESUMED 2026-06-15 under new supervisor with
fresh worker sessions (below). Old session_ids retained for history lookup.

| Role | NEW session_id | OLD (history) |
|------|-----------|-----------|
| Cargo Serial Worker | 95e8b784-af0b-45b9-95a8-760fa185bfa2 | ca30b3e0-8bd9-4016-a98c-29c74c0f84f5 |
| W1 (RPC-197/199/201) | e2327c1b-62cd-4ba0-bb15-cd1b5070e261 | 8fe20681-d9ae-4369-bdf4-ba023e3e16fc |
| W2 (RPC-207/220/230) | c6519650-bb9e-4324-85bb-ee6d4e4f3caa | 2a119d3f-8488-4f33-a5c6-ab6134a256bd |
| W3 (RPC-231/240) | a41ca9e5-c101-4ba0-b76f-8986ec392673 | 1d12dee6-bdff-4776-8415-8478b5d118e1 |
| W4 (RPC-235/238) | e553f0b4-47a8-4934-9aa3-7db4232a2f8d | 3182418f-0888-4860-b72f-1f0b94e70ad7 |

## Resume status (2026-06-15, 3rd resume — supervisor 6fef18a4)
- Prior supervisor dd14460f died right after re-tasking workers to add IMPL mappings.
- New supervisor: 6fef18a4-3b1e-4322-86d2-e8cef05b69e3.
- Verified disk state: test mappings 100% done; IMPL mappings missing for 92 scenarios.
  Per-worker remaining (noImpl):
    W1: audit-coverage rust 4 + cli 5; board rust 4 + cli 4; check-cli 5 (check-rust DONE) = 22
    W2: compare-impl rust 4 + cli 6; delete-scenarios rust 6 + cli 5; format rust 5 + cli 6 = 32
    W3: generate-coverage rust 5 + cli 6; link-coverage rust 12 + cli 6 = 29
    W4: import-example-map rust 4 + cli 5 (generate-summary-report DONE) = 9
- Impl file convention: rust-port→codelet/fspec-core/src/commands/<cmd>.rs;
  cli bridge→codelet/fspec/src/<cmd>.rs; --help scenarios→codelet/fspec-core/src/help/configs/<cmd>.rs
  (delete-scenarios --help lives in codelet/fspec/src/main.rs DELETE_SCENARIOS_HELP).
- RULE (REINFORCED BY HUMAN): NEVER kill any node/fspec process. No kill/pkill/killall.
- Re-spawned workers (3rd resume):
  - W1 → a6bcfd3d-109a-4199-8934-e1efa6f53c9b (hist 763b648e/e2327c1b)
  - W2 → 9631974e-4c34-4eb3-8c7d-2ca7bc706cfa (hist fd585de0/c6519650)
  - W3 → 660eeb45-2993-4eae-80bd-5bcdac9742f6 (hist 7a00aa12/a41ca9e5)
  - W4 → efbc4663-85d2-403a-b024-cf5461be0631 (hist 0c049290/e553f0b4)

## Resume status (2026-06-15, 2nd resume — supervisor dd14460f)
- Prior supervisor 240198ee died right after dispatching coverage-linking tasks.
- New supervisor: dd14460f-a0a6-4fe4-b2da-496186c6ca7f.
- Re-spawned workers (coverage-linking only; supervisor advances status serially):
  - W1 → 763b648e-78d2-4622-9fbc-5b3aefd2ccd2 (hist e2327c1b) — check-cli 3 uncovered
  - W2 → fd585de0-6519-4303-96be-f221c87bb34b (hist c6519650) — delete-scenarios-cli 2, format-rust 5, format-cli 6
  - W3 → 7a00aa12-b3ae-4b32-a090-fe3abc7c0dad (hist a41ca9e5) — generate-coverage-rust 6, link-coverage-rust 12
  - W4 (e553f0b4) NOT re-spawned: its features (generate-summary-report, import-example-map) are fully covered; only needs status advance.
- Remaining uncovered at resume: 34 scenarios total (W1:3, W2:13, W3:18, W4:0).
- RULE: do not kill fspec processes.

## Resume status (2026-06-15, 1st resume)
- Phase A: COMPLETE (all features/example-maps/estimates on disk).
- Phase B: COMPLETE for all workers (W3's missing cli_generate_coverage.rs +
  cli_link_coverage.rs were written on resume).
- Phase C: COMPLETE. All 10 core impls + CLI bridges written; gherkin_format.rs
  created + registered in io/mod.rs. Supervisor shared wiring done:
  canonical PORTED_COMMANDS (+10), dispatch run_ported (+10) / stub arms→comments,
  help/configs/mod.rs (+9), main.rs (mods/Mode/forward/intercept/DELETE_SCENARIOS_HELP),
  cargo_shape.rs (allowed +10, count 139→149, main_cap 3000→3300).
  Shared-test fixups: dispatcher_test.rs + list_work_units.rs swapped the
  "unported example" from audit-coverage/RPC-197 → auto-advance/RPC-198;
  generate_coverage bridge doc comment reworded (forbidden "system-reminder").
- BUILD + TESTS: GREEN. Warning-free build; full suite 2861 passed / 0 failed.
- DISK: codelet/target/debug (213G) removed on resume to clear a 100%-full disk;
  release binary preserved. Rebuilt clean.
- REMAINING: complete per-scenario coverage linking (workers in progress) →
  advance the 10 work units validating→done → flip @wip→@done → optional commit.

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| W2 (format/RPC-230) | codelet/fspec-core/src/io/mod.rs | register NEW `pub mod gherkin_format;` (W2 creates the file) | PENDING (Phase C) |
| W2 (delete-scenarios/RPC-220) | codelet/fspec/src/main.rs | add `DELETE_SCENARIOS_HELP` const + bare-Commander intercept arm (no help-config module) | PENDING (Phase C) |
| W1/W2/W3/W4 (all) | canonical.rs / dispatch.rs / help/configs/mod.rs / main.rs / cargo_shape.rs | standard wiring of 10 commands | PENDING (Phase C) |

## Phase A reports (estimates + scenario counts)

- RPC-197 audit-coverage (W1, 2pt): rust-port 4 + cli 5. Framing A: `--fix` in help-doc but TS CLI doesn't implement → port actual output.
- RPC-199 board (W1, 3pt): rust-port 4 + cli 4. DECISION: serve headless plain-text default + format=json (list-* precedent). APPROVED.
- RPC-201 check (W1, 5pt): rust-port 4 + cli 5. DECISION: format sub-check → SKIP (formatter in-flight on W2); document Framing-A divergence. APPROVED Option B.
- RPC-207 compare-implementations (W2, 3pt): rust-port 4 + cli 6. Help fixture preserves TS `undefined`-render bug.
- RPC-220 delete-scenarios (W2, 5pt): rust-port 6 + cli 5. Bare-Commander help → main.rs special-case. APPROVED.
- RPC-230 format (W2, 8pt): rust-port 5 + cli 6. Needs NEW io/gherkin_format.rs + io/mod.rs registration. APPROVED.
- RPC-231 generate-coverage (W3, 5pt): rust-port 6 + cli 6. Reuses types::coverage.
- RPC-240 link-coverage (W3, 8pt): rust-port 12 + cli 6. update_stats duplicated locally; types::coverage sufficient.
- RPC-235 generate-summary-report (W4, 3pt): rust-port 7 + cli 5.
- RPC-238 import-example-map (W4, 3pt): rust-port 6 + cli 5. Inverse of export-example-map (RPC-228).

## Supervisor wiring checklist (Phase C)
- [ ] canonical.rs: add 10 to PORTED_COMMANDS
- [ ] dispatch.rs: add 10 run_ported arms; comment-out 10 run_stub arms
- [ ] commands/mod.rs: already declares modules (stubs) — no change needed
- [ ] help/configs/mod.rs: register 10 new help configs
- [ ] main.rs: 10 Mode variants + 10 forward! arms + 10 intercept arms + 10 `mod` decls
- [ ] cargo_shape.rs: add 10 bridges to allowed set; bump main_cap if needed

## Resume status (2026-06-15, 4th resume — supervisor 175ff2bc)
- ROOT CAUSE CONFIRMED (per human request to diagnose first):
  fspec host deaths are NOT from kill/pkill (no agent ever ran one) and NOT OOM
  (121G RAM, cgroup memory.max=max, zero journalctl OOM events). The killer is
  DISK-FULL: cargo build/test ballooned codelet/target to 224G → disk 100% →
  node/fspec host fails session/DB/lock writes → process dies.
- Current disk: / at 83% (152G free); codelet/target=73G (debug 62G, release 11G).
  A full from-scratch rebuild (~224G peak) WOULD refill the disk — only INCREMENTAL
  cargo allowed, abort if free < 30G.
- Remaining work: 111 coverage impl-mappings (link-coverage only, NO cargo, NO risk).
  Split across 3 parallel workers (human chose 3):
    WA (36): audit-coverage(9) board(8) check(9) compare-implementations(10)
    WB (34): delete-scenarios(11) format(11) generate-coverage(12)
    WC (41): link-coverage(18) generate-summary-report(12) import-example-map(11)
  (counts = cli + rust feature files' empty implMappings)
- 1 cargo serial worker reserved for ONE final incremental verify at the end (human-approved).
- RULE: never kill any node/fspec process; no kill/pkill/killall; workers never run cargo.

## Resume status (2026-06-15, 5th resume — supervisor 66f03d4f)
- ROOT CAUSE re-confirmed via SessionSearch of ALL spawned workers' bash:
  NO worker ever ran kill/pkill/killall/cargo. The kill/cargo search hits were
  all from OLDER unrelated sessions (Mar + Jun-10/12). BATCH-17 coverage workers
  ran ONLY fspec link/unlink-coverage. fspec host deaths = DISK-FULL from cargo
  target (224G) during the earlier BUILD phase — NOT a kill, NOT OOM.
- Current disk: / at 74% (227G free); codelet/target ABSENT. Safe.
- ACTUAL remaining work (authoritative, parsed from .feature.coverage):
  NOT "add impl" — it is DUPLICATE testMapping cleanup left by prior parallel
  workers. 37 scenarios carry a correct (test+impl) mapping PLUS a stale empty
  duplicate; 1 scenario genuinely missing. Recipe per dup scenario:
    fspec unlink-coverage <feat> --scenario "<name>" --all
    fspec link-coverage   <feat> --scenario "<name>" --testFile .. --testLines .. --implFile .. --implLines ..
- Split (DISJOINT feature files → no lock contention):
    WA(14): check-cli + compare-implementations rust + cli   -> /tmp/BATCH17/WA.json
    WB(12): generate-summary-report rust + cli               -> /tmp/BATCH17/WB.json
    WC(9):  import-example-map rust + cli (1 genuinely missing) -> /tmp/BATCH17/WC.json
- RULE (REINFORCED): workers NEVER run cargo / kill / pkill / killall. Only fspec.
- Final: supervisor advances 10 WUs validating→done, @wip→@done, then ONE
  incremental cargo verify (human-approved) guarded: abort if free disk < 30G.
