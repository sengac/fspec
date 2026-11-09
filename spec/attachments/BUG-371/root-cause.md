# BUG-371 root cause

Found by the regression sweep after TOOL-027 and BUG-370. Three failures, all of them already present before TOOL-027:

1. `source_shape_rpc018::new_shared_types_live_in_rpc_types` and `source_shape_rpc022::new_shared_types_live_in_rpc_types` require the substring `pub context_window: u32` in rpc-types. PROV-166 (e799d469b, "model limits on the wire are null when unavailable") changed `ModelInfo.context_window` and `ModelEntry.context_window` to `Option<u32>` (`rpc-types/src/dialogs.rs`, `chrome.rs`), so no `u32` field is left. The scenarios were never updated for that approved breaking change.
2. `source_shape_rpc018::new_and_modified_agent_modules_stay_under_300_lines` measures each file with `count_lines_path`, which calls `codelet_test_helpers::source_composition::read_tree_or_file`. That helper returns `modules::read_tree`: the file followed by every file-backed child module, recursively. So `views/agent.rs` (293 physical lines) reports 14934. `reflow_cache.rs`, `scrollback_trim.rs`, `mouse_dispatch.rs`, `scrollback_paint.rs`, `input_transition.rs`, `multiline_input_select.rs` and `scrollback.rs` report themselves plus their `*_tests` child modules. No file in `views/agent/` is physically 300 lines or longer; the largest is 298. The helper's own `expect` message says "read physical source for line count", so the intent was physical lines.

## Fix
- The two scenarios now say `pub context_window: Option<u32>`, and the tests assert that substring.
- `count_lines_path` (now in `tests/source_shape_support/mod.rs`, REFAC-044) reads the file itself with `std::fs::read_to_string`. The scenario step says "physical lines". The RPC-022 300-line test uses the same helper and gets the same, stricter-in-intent measurement.
- No assertion is loosened. The ceiling stays `< 300` per file.

`read_tree_or_file` also swallows a `read_tree` error and falls back to a plain read (`if let Ok(text) = modules::read_tree(path)`). That is a silent fallback in the test-helpers crate, and it is filed as its own card.
