# Batch 8 Orchestration State (COMPLETED 2026-06-11)

## Workers

| Slot | RPC IDs | Worker session_id | Status |
|------|---------|-------------------|--------|
| A    | RPC-189 (add-rule), RPC-279 (remove-rule), RPC-169 (add-assumption) | 51bc7542-23db-4463-8192-2477834da8e2 | ✅ DONE — closed |
| B    | RPC-181 (add-example), RPC-273 (remove-example) | c618bb6c-3f84-41b6-b9fd-b6e3b5584484 | ✅ DONE — closed |
| C    | RPC-188 (add-question), RPC-278 (remove-question) | 06d59632-62c1-4cf6-bf22-7b827f864dfe | ✅ DONE — closed (fixed JSON-fixture bug late) |
| D    | RPC-168 (add-architecture-note), RPC-267 (remove-architecture-note) | e30d8a18-22c4-4144-978f-5e07455a0ac6 | ✅ partial — finished 168/267, abandoned 298; closed |
| E    | RPC-298 (set-user-story) | ba9e366a-de13-49ef-bd31-45c4732d38af | ✅ DONE — replacement worker; closed |

**Cargo Serial Worker session_id:** 5632a0b6-1ae7-42c9-8d6e-99c33820347b (closed)

## Final tally
- **10 commands ported**: add-rule, remove-rule, add-assumption, add-example, remove-example, add-question, remove-question, add-architecture-note, remove-architecture-note, set-user-story
- All work units → DONE
- All feature files tagged @done (20 feature files: 10 *-rust-port + 10 *-cli-subcommand)
- Dispatcher tests: 62 passing (all 10 commands)
- CLI tests: 50+ passing (all 10 commands)
- cross_frontend_parity: 8/8 passing
- (Unrelated pre-existing failures: 2 io::time tests — not touched by batch)

## Issues encountered & resolved

1. **Worker D abandoned mid-PHASE-B on RPC-298** — spawned Worker E to finish.
2. **Worker C's test fixtures had duplicate JSON keys** ("specifying"/"testing") in `remove_question.rs` and `cli_add_question.rs`. Worker C fixed.
3. **Worker A's CLI bridge had inline `✓ Rule added`** which violated their own bridge-purity test. Worker A chose Option B (test assertion update) with TS-parity rationale.
4. **Stale orphan link-coverage entries** in `set-user-story-*-feature.coverage` and `remove-example-cli-subcommand.feature.coverage` — cleaned manually.

## Shared file edits (supervisor)
- `codelet/fspec-core/src/canonical.rs::PORTED_COMMANDS` — added 10 entries
- `codelet/fspec-core/src/dispatch.rs::run_ported` — added 10 arms; removed 10 from run_stub
- `codelet/fspec-core/src/help/configs/mod.rs` — added 10 module declarations
- `codelet/fspec/src/main.rs` — added 10 `mod`, 10 Mode variants, 10 forward arms, 10 intercept arms
