# Batch 11 Orchestration State

## ✅ BATCH COMPLETE (resumed after crash by supervisor session 5452972a)
All 10 work units DONE: RPC-165, RPC-174, RPC-179, RPC-185, RPC-172, RPC-182, RPC-187, RPC-214, RPC-210, RPC-215.
Both Rust crates compile clean (release); all per-command test binaries GREEN (core + CLI, incl. help-fixture byte parity).
Supervisor wired all shared files (canonical.rs, dispatch.rs, help/configs/mod.rs, main.rs, io/ensure.rs + error.rs FoundationMissing, cargo_shape.rs 78→88 / main_cap 1700→2000). @mutation tag registered.
Outstanding advisory (project-wide, NOT a batch regression): validate-tags flags ~430 sibling features for missing @component/@feature-group — a tagging-policy decision left for a dedicated work unit.

### Resume-verification (supervisor, current session) — all 10 RE-CONFIRMED done on disk
Read work-units.json at rest: RPC-165/174/179/185/172/182/187/214/210/215 all status=done, full EM (userStory+rules+examples) persisted; all per-feature coverages 100% test+impl. Workers W1/W3/W5 reported GREEN (W1 20 tests, W3 26, W5 36). W3 & W5 hit stale-snapshot/lost-write symptoms re-doing already-complete EM — diagnosed as crash-recovery redundancy against the already-COMPLETE store; no data corruption (final reads authoritative). All released.

### CONCURRENCY LEARNING (apply to future batches)
work-units.json has NO write serialization under N-parallel-workers + resumed sessions → lost writes + cross-worker output cross-talk (reported independently by W3 and W5). FIX: gate ALL work-units.json mutations through the supervisor (single writer), same as cargo is serialized through one runner. Workers should request status/EM changes, supervisor applies them.

### Cleanup item (DONE by supervisor)
create_story.rs:112 dead `mut` (flagged by W1/W3/W5). Worker 4 (7d8aee29) unreachable via AgentManager (peer session, not subordinate). Verified by inspection that `data` is read-only (lines 116/122/144/175; write path builds separate `top` JSON via write_json_atomic). Supervisor removed `mut` directly as batch closeout (porting phase complete → no parallel-write risk on that file). Edit is behavior-neutral (pure warning removal); test suite was already GREEN. CARGO RE-VERIFY PENDING — defer to next full `cargo test`/`cargo clippy` run; no behavioral risk.


Supervisor: this session. Project root: `/home/rquast/projects/fspec`.

## Batch theme
Event-Storm work-unit mutation commands (7) + create-* work-unit commands (3).
All operate on `spec/work-units.json` (event-storm items live in `wu.extra["eventStorm"]["items"]`;
create-* also touch `spec/prefixes.json` + `spec/epics.json`).

## Current batch

| Slot | RPC ID | Command | snake | Worker session_id | Phase | Notes |
|------|--------|---------|-------|-------------------|-------|-------|
| 1a   | RPC-165 | add-aggregate        | add_aggregate        | aa690f52-d255-4929-8949-4603d94cc047 | A | ES inline, color yellow, --responsibilities (CSV) |
| 1b   | RPC-174 | add-command          | add_command          | aa690f52-d255-4929-8949-4603d94cc047 | A | ES inline, color blue, --actor |
| 2a   | RPC-179 | add-domain-event     | add_domain_event     | cdc0d4bd-5e1f-4031-be8f-875ef8cb89ff | A | ES inline, color orange, dup-check (BUG-087) |
| 2b   | RPC-185 | add-hotspot          | add_hotspot          | cdc0d4bd-5e1f-4031-be8f-875ef8cb89ff | A | ES shared-util style, color red, --concern |
| 3a   | RPC-172 | add-bounded-context  | add_bounded_context  | ca7ab44f-e1bd-4b67-a21c-2b3a27156e29 | A | ES shared-util, color null, --description |
| 3b   | RPC-182 | add-external-system  | add_external_system  | ca7ab44f-e1bd-4b67-a21c-2b3a27156e29 | A | ES shared-util, color pink, --type (integrationType) |
| 4a   | RPC-187 | add-policy           | add_policy           | 7d8aee29-bf33-4d89-931f-76702c6f19bf | A | ES shared-util, color purple, --when/--then |
| 4b   | RPC-214 | create-story         | create_story         | 7d8aee29-bf33-4d89-931f-76702c6f19bf | A | work-units.json + epics; ref create_epic |
| 5a   | RPC-210 | create-bug           | create_bug           | 0ef34649-3554-4b4e-b1bc-cc62de82a5b4 | A | work-units.json + epics; ref create_epic |
| 5b   | RPC-215 | create-task          | create_task          | 0ef34649-3554-4b4e-b1bc-cc62de82a5b4 | A | work-units.json + epics; ref create_epic |

**Cargo Serial Worker session_id:** 13b85793-d1bb-4f3a-985c-cfa0ea879ac4
**Worker sessions (resumed after crash):** W1=aa690f52 W2=cdc0d4bd W3=ca7ab44f W4=7d8aee29 W5=0ef34649

### Phase B COMPLETE (all 5 workers) — gated into Phase C
All 10 cards in `testing`. RED baselines confirmed by cargo runner (13b85793): all test crates COMPILE; core tests fail NotYetPorted; CLI tests fail `unrecognized subcommand`. All scenarios link-coverage'd (test-only, impl deferred to C).
Phase C protocol: workers write ISOLATED impl files only (commands/<snake>.rs, help/configs/<snake>.rs, fspec/src/<snake>.rs), ask cargo runner to type-check core, then STOP and WAIT for supervisor shared-file wiring before green run.

### Phase C wiring DONE by supervisor — both crates compile clean (release)
canonical.rs/dispatch.rs/help/configs/mod.rs/main.rs/io/ensure.rs(+error.rs FoundationMissing)/cargo_shape.rs (78→88, main_cap 1700→2000) all wired.
Cargo runner confirmed: `cargo build -p codelet-fspec-core` exit 0; `cargo build -p codelet-fspec` exit 0. No errors. @mutation tag registered.
Workers notified to drive GREEN test runs via cargo runner, then validating → done.

### Supervisor Phase C wiring TODO (after workers finish isolated impl)
- [ ] canonical.rs: add 10 to PORTED_COMMANDS / is_ported
- [ ] dispatch.rs: move 10 arms stub→ported, pass project_root (create_* + event-storm cmds need (args_json, project_root))
- [ ] commands/mod.rs: confirm 10 modules declared
- [ ] help/configs/mod.rs: register 10 help configs
- [ ] main.rs: 10 Mode variants + forward/intercept arms + mod decls
- [ ] io/ensure.rs: add ensure_epics_file (auto-create), check_foundation_exists (verbatim msg+<system-reminder>); ensure_prefixes_file/ensure_work_units_file already present
- [ ] cargo_shape.rs: bump main_cap + lock-list file count
- [ ] register @mutation tag project-wide

### Phase A COMPLETE (all 5 workers, post-resume) — gated into Phase B
All 10 cards specified: feature files (rust-port + cli-subcommand) generated, validated, estimated, @wip tagged.
Estimates: RPC-165=5 RPC-174=5 RPC-179=3 RPC-185=3 RPC-172=3 RPC-182=3 RPC-187=3 RPC-214=5 RPC-210=5 RPC-215=5.
Supervisor rulings issued at Phase B gate:
- Event-storm cmds (165/174/179/185/172/182): Option B — inline existsSync + read_json, error (NO auto-create) on missing file. No ensure.rs change.
- add-aggregate CLI output `✓ Added aggregate "<text>" to <id> (ID: <n>)` to stdout / errors stderr: APPROVED.
- create-* cmds (214/210/215): supervisor WILL add to io/ensure.rs in Phase C: ensure_epics_file (auto-create), ensure_prefixes_file, check_foundation_exists; and update dispatch.rs/commands/mod.rs to pass project_root. Workers write failing tests in Phase B regardless.

### Crash-recovery note (resumed by supervisor session 5452972a)
Prior worker sessions (killed): W1=c3671c15 W2=543dff83 W3=a7ae7699 W4=936c2356 W5=cc5444e2; cargo=7d88ffa0.
All 10 cards still in `specifying` (Phase A). On-disk progress at crash: only W3 add-external-system-rust-port.feature and W4 add-policy-rust-port.feature created. Example-mapping data may exist in work-units.json for several cards. Each resumed worker is told to read its prior session history and the current fspec state before continuing Phase A.

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| (anticipated) | io/ensure.rs | add `ensure_epics_file` (auto-create) for create-* | open |
| (anticipated) | io/ (new) foundation_check.rs | `check_foundation_exists` for create-* | open |

## Supervisor wiring checklist (Phase C)
- [ ] canonical.rs: add 10 PORTED_COMMANDS lines (Batch 11)
- [ ] dispatch.rs::run_ported: add 10 arms; remove 10 from run_stub
- [ ] commands/mod.rs: ensure 10 modules registered (stubs already present)
- [ ] help/configs/mod.rs: register 10 help configs
- [ ] main.rs: 10 Mode variants + 10 forward! arms + 10 intercept arms + 10 `mod` decls
- [ ] cargo_shape.rs: bump main_cap + lock-list file count (78 → 88)
