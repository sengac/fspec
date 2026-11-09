# REFAC-048 design

`rust/fspec-core/src/commands/update_work_unit_status.rs` (1445 lines) is split so BUG-378 can change
`linked_feature_files` in a file under 300 lines. No behaviour change: code moves verbatim, `run` is cut into named
phases.

| File (under `commands/update_work_unit_status/`) | Contents |
|---|---|
| `../update_work_unit_status.rs` | args, `run` orchestration |
| `transitions.rs` | `VALID_STATES`, `allowed_transitions`, `status_from_str`, `msg`, `check_transition_gates` (task testing, backlog, blocked reason, transition table, dependency blockers) |
| `queries.rs` | `Data` and the work-unit field accessors, `check_unanswered_questions` |
| `prefill.rs` | `linked_feature_files`, prefill detection, `check_prefill` |
| `review.rs` | `perform_review_validation`, capture mode, AST-research message, `check_scenarios_exist`, `check_testing_entry` (the specifying -> testing gate block of `run`) |
| `coverage_gate.rs` | `check_coverage_completeness`, `discover_feature_names_by_tag` |
| `temporal.rs` | `check_temporal_ordering`, test-file discovery, mtime/ISO helpers |
| `checkpoints.rs` | `git`, `working_dir_dirty`, `maybe_auto_checkpoint`, `cleanup_done_checkpoints` |
| `board_state.rs` | `apply_status_change` (status, blockedReason, stateHistory), `move_state`, `states_column`, `compact_done`, `run_pre_hooks` |
| `output.rs` | reminder collection and stdout rendering |
| `tests.rs` | the existing unit tests |

Children use `use super::*;` for the shared imports and helpers; moved functions become `pub(super)`.
Verification: `cargo check`/`clippy` on fspec-core and the update-work-unit-status integration tests.
