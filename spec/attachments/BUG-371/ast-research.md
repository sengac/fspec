# BUG-371 AST research

## grep -rn 'pub context_window' rust/rpc-types/src
session_state.rs:45  pub context_window: Option<i64>,   (SessionModel)
dialogs.rs:29/54/81  pub context_window: Option<u32>,   (ModelEntry, ModelInfo, ...)
chrome.rs:27         pub context_window: Option<u32>,
chunk_support.rs:47  pub context_window: f64,            (token usage, unrelated)

## Line-count helper
rust/fspec-tui/tests/source_shape_support/mod.rs `count_lines_path` -> `source_composition::read_tree_or_file` -> `modules::read_tree` (rust/test-helpers/src/source_composition/modules.rs:11), which appends `read_tree(child)` for every `mod x;` without inline content.

## Physical sizes (wc -l, views/agent)
Largest: slash_command_popup.rs 298, confirm_dialog.rs 296, scrollback_copy.rs 296, views/agent.rs 293.

## Red evidence (REFAC-044 verification run)
rpc018: 7 passed, 2 failed (new_shared_types_live_in_rpc_types; new_and_modified_agent_modules_stay_under_300_lines with views/agent.rs: 14934 and seven others); rpc022: 9 passed, 1 failed (new_shared_types_live_in_rpc_types).
