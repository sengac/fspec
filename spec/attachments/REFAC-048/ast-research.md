# REFAC-048 AST research

- `grep -n "^fn\|^pub async fn\|^type\|^const\|^struct" update_work_unit_status.rs` -> 45 items; banners at 425,
  547, 746, 980, 1205, 1281, 1338 already group them (accessors, feature discovery + review, coverage, temporal,
  checkpoints, state moves, pre-hooks).
- Cross-chunk calls (scripted grep over the line ranges): `run` calls into every group; review uses `array_len`,
  `string_array`, `linked_feature_files`; temporal uses `git` (checkpoints) and `linked_feature_files`; every group
  uses `msg`.
- `run` is lines 88-428 (341 lines), so it is cut into `check_transition_gates`, `check_testing_entry`,
  `apply_status_change` and `render_output`.
- Callers: `grep -rn "update_work_unit_status::" rust/` -> `run`, `reminders` (pub(crate)).
