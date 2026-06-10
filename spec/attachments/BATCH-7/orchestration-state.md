# Batch 7 — Orchestration State (COMPLETE)

**Started:** 2026-06-10
**Completed:** 2026-06-10
**Supervisor session_id:** 30167441-9c28-4e18-8db9-11e53befc0a4
**Cargo Serial Worker session_id:** d6a0bcba-6ac6-4943-92fc-6f1287df22cc (closed)

## Workers (all closed)

| Slot | RPC IDs                  | Command(s)                                              | Phase   | Result |
|------|--------------------------|---------------------------------------------------------|---------|--------|
| 1    | RPC-211, RPC-217         | create-epic, delete-epic                                | C done  | 🟢 done |
| 2    | RPC-213, RPC-313         | create-prefix, update-prefix                            | C done  | 🟢 done |
| 3    | RPC-265, RPC-316, RPC-222| register-tag, update-tag, delete-tag                    | C done  | 🟢 done |
| 4    | RPC-176, RPC-271, RPC-204| add-dependencies, remove-dependency, clear-dependencies | C done  | 🟢 done |

**Total: 10 mutation commands ported, all 10 work units in `done` status.**

## Test results (final)
- Core (10 test binaries): **103 / 103** passed
- CLI (10 test binaries): **62 / 62** passed
- `cross_frontend_parity`: **8 / 8** passed
- `cargo_shape`: **11 / 11** passed
- **Grand total: 184 / 184** ✅

## Supervisor wiring (shared files)
- `codelet/fspec-core/src/canonical.rs`: 10 entries added to `PORTED_COMMANDS`
- `codelet/fspec-core/src/dispatch.rs`: 10 arms added to `run_ported`, 10 arms removed from `run_stub` (replaced with "intentionally absent" comments)
- `codelet/fspec-core/src/help/configs/mod.rs`: 10 `pub mod ...` declarations added
- `codelet/fspec/src/main.rs`: 10 `mod ...` decls + 10 `Mode::*` variants + 10 `forward!` arms + 10 intercept arms
- `codelet/fspec/tests/cargo_shape.rs`: lock-list extended by 10 bridge filenames, `main_cap` bumped from 850 → 1100

## Notable fixes during orchestration
1. **Worker 1 over-eager `epicId"` substring guard in cli_delete_epic** — removed from forbidden list (the bridge legitimately needs JSON-key marshalling).
2. **Worker 1 create-epic help config indentation** — `Title:` / `Description:` continuation lines need 2-space indent in example output.
3. **Worker 3 register-tag fixture mismatch** — TS has no `register-tag-help.ts`, so TS CLI emits bare Commander.js. Rust uses rich `format_command_help`; updated fixture to match rich format (Framing A: when TS has no doc canon, Rust's rich help becomes the new canon).
4. **Worker 4 add-dependencies clap variadic** — TS uses `--blocks <ids...>` (variadic per flag). Rust initially used `value_delimiter = ','` (comma-separated). Fixed to `num_args = 1..`.
5. **Worker 2 update-prefix doc-comment leaked `prefixes.json`** — removed the literal to satisfy the two-front-doors substring guard.
6. **Workers edited dispatch.rs** — workers replaced one-arg stub calls with two-arg ported signatures in `run_stub` (which has no `project_root` in scope). Supervisor reverted/removed those and added proper arms in `run_ported`.

## Lessons for future batches
- Workers must NOT edit `dispatch.rs` even when they "rewrite their stub" — the stub signature change makes `run_stub` not compile. Supervisor wires shared files.
- Pre-flight question: does the command have a TS `-help.ts` companion? If NO, the rich Rust formatter becomes canon (Framing A inverse) — capture the rich-format fixture, don't capture bare Commander.js.
- `main.rs` line budget needs bumping for batches that add many clap variants with multi-field bodies (Batch 7 +10 commands ≈ +200 lines).
- Workers occasionally proceed to next command without waiting — fine if shared file changes can wait, but supervisor must wire ALL commands before any cargo run.

## Completed batches log
| Batch | RPC IDs | Completed | Notes |
|-------|---------|-----------|-------|
| 7 | RPC-211, 217, 213, 313, 265, 316, 222, 176, 271, 204 | 2026-06-10 | First mutation-command batch; established write_json_atomic pattern, IndexMap/extra-map preservation, atomic write-at-end discipline |

