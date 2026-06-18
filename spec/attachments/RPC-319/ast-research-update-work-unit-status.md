# AST / Behaviour Research — `update-work-unit-status` (RPC-319)

TS source of truth: `src/commands/update-work-unit-status.ts` (1404 LOC).
Rust stub: `codelet/fspec-core/src/commands/update_work_unit_status.rs` (1-arg `run(args_json)` → NotYetPorted).

## 1. Command surface (registerUpdateWorkUnitStatusCommand, line 1324)
```
fspec update-work-unit-status <workUnitId> <status> [options]
  --blocked-reason <reason>        required when status === 'blocked'
  --skip-temporal-validation       bypass temporal-ordering gate
```
Positional: workUnitId, status. CLI handler at line 1351 marshals to `updateWorkUnitStatus(...)`.

## 2. Core entry (updateWorkUnitStatus, line 93)
- `ALLOWED_STATES` (line 51): backlog, specifying, testing, implementing, validating, done, blocked (7).
- `STATE_TRANSITIONS` (line 61): the directed map (already captured verbatim in rule [0]).
- Line 117: unknown `newStatus` not in ALLOWED_STATES → error `Invalid status value: <s>. Allowed values: ...`.
- Unknown workUnitId → `Work unit <id> does not exist`.
- `isValidTransition(from,to)` (line 846): membership check against STATE_TRANSITIONS[from].
- Moving to `blocked` requires `blockedReason` (else error).

## 3. Forward-transition validation gates (first failure wins)
- `checkScenariosExist` (line 1014): specifying→testing requires the linked feature to have ≥1 scenario.
- Prefill-detection: linked feature must not contain placeholders (shared regex with `review`).
- `validateTestStepDocstrings` (line 863): test files must carry feature docstring + @step coverage.
- `checkFeatureCoverage` (line 1048) + `checkCoverageCompleteness` (line 1132): testing/implementing→validating requires all scenarios have test mappings.
- Temporal-validation: `checkFileCreatedAfter` / `findStateHistoryEntry` — feature file must be modified at/after the state-entry time, UNLESS `skipTemporalValidation`.
- Dependency/children gates (lines 351, 418): incomplete deps/children block some transitions; unanswered questions (line 273) gate too.

## 4. Side-effects on a successful transition
- Auto git checkpoint BEFORE applying transition when working dir is dirty (skipped for →backlog).
- transaction over `spec/work-units.json` (line 662): move id between state arrays, push state-history entry w/ timestamp.
- →done: compact the work unit (reuse `compact_work_unit`) + cleanup AUTO checkpoints, preserve MANUAL.
- Done array kept sorted (line 634 comparator).
- Virtual hooks: filter by `hookEvent` matching the transition, run via `executeHooks` (line 604) BEFORE transition; blocking failure aborts.
- Pre/post global hooks around the command.
- Consolidated system-reminders emitted (status-change + virtual-hooks + cleanup).
- IPC notify TUI (`onWorkUnitStatusUpdated`, line 39) — **NO-OP in Rust port** (Batch 18 precedent).

## 5. Rust reuse map (what already exists)
- `codelet_git::ghost_commit::create_ghost_commit` (checkpoint.rs:39/109) — auto-checkpoint primitive. ✓
- `restore_ghost_commit` / `get_checkpoint_diff_files` (restore_checkpoint.rs) + `AUTO_CHECKPOINT_PATTERN` (list_checkpoints.rs) — auto vs manual classification for the →done cleanup. ✓
- `commands::compact_work_unit::run(args_json, project_root)` (compact_work_unit.rs:57) — reuse for →done compaction; already 2-arg. ✓
- `io::gherkin` + `io::gherkin_format` — scenario/prefill detection + feature parse. ✓
- `io::ensure::{ensure_work_units_file, read_work_units_or_empty}` + `types::work_unit::{WorkUnitStatus, ...}` (work_unit.rs:400) — store + status enum w/ `as_str`. ✓

## 6. GAP FLAGGED TO SELF (supervisor) — fspec-command hooks executor
- TS imports `executeHooks` from `../hooks/executor` and `HookDefinition` from `../hooks/types` (lines 36-37).
- There is NO ported fspec-command hooks executor in `codelet/fspec-core/src` (only add/remove/list hook commands manage config; no runner).
- `codelet/core/src/lifecycle_hooks/executor.rs` is the AGENT lifecycle-hooks executor (different domain: session_start/user_prompt_submit). NOT reusable as-is.
- DECISION for this port: implement a minimal blocking hooks executor inside fspec-core (or a small `hooks` module) that runs configured pre/post + virtual hooks via blocking `std::process::Command`, honouring `blocking`, `timeout`, and `condition` (tags/prefix/epic/estimate). Must be synchronous (poll_sync_future). Blocking-hook stderr wrapped in `<system-reminder>`. This is the single largest sub-task and the main reason for the 13-point estimate.

## 7. Async note
All IO is blocking std::fs + git2/gitoxide via codelet_git + blocking std::process for hooks. NO real tokio .await. Safe under poll_sync_future (single poll).

## 8. Shared-file changes (supervisor applies in Phase C — I own these for this card)
1. `commands/update_work_unit_status.rs`: rewrite stub `run(args_json)` → `run(args_json, project_root)`.
2. `dispatch.rs`: update-work-unit-status arm → 2-arg `run(args_json, project_root).await`; move from run_stub to run_ported.
3. `canonical.rs` PORTED_COMMANDS: add "update-work-unit-status".
4. `main.rs`: Mode::UpdateWorkUnitStatus clap variant {work_unit_id, status, blocked_reason:Option, skip_temporal_validation:bool} + forward! arm + --help intercept + `mod update_work_unit_status;`.
5. `help/configs/mod.rs`: register `pub mod update_work_unit_status;`.
6. New files I create: bridge `codelet/fspec/src/update_work_unit_status.rs`, help config `codelet/fspec-core/src/help/configs/update_work_unit_status.rs`, dispatcher test `codelet/fspec-core/tests/update_work_unit_status.rs`, CLI test `codelet/fspec/tests/cli_update_work_unit_status.rs`, help fixture `codelet/fspec/tests/fixtures/help/update-work-unit-status.txt`, and possibly a `codelet/fspec-core/src/hooks/` module for the executor (§6).
