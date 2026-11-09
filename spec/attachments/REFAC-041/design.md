# REFAC-041 design

BUG-351 must fix a stale bootstrap subscriber-count assertion in `rust/fspec-tui/tests/rpc434_session_events_tui_consumption.rs` (327 lines, over the 300-line limit). Pure restructure, no production change:
- The root file keeps the header, imports, `mod common;` and the helpers. Test functions move, with their doc/banner comments and attributes, into tests/rpc434_session_events_tui_consumption/removal_and_bootstrap.rs (the last three tests). The root pulls them in with `include!("<root>/<file>.rs")`, the layout tests/inline_pause_prompt_rpc406.rs already uses. A first try with `mod` + `use super::*` failed: the module path resolves next to the crate root, and the root's imports then warn as unused. One test binary per feature is kept.
- Every test body and `@step` comment is unchanged, byte for byte. Result: root 168, removal_and_bootstrap.rs 164.
- Coverage: every test mapping that pointed at the old file now points at the moved function's file and range (fn line to closing brace). The function is chosen by full `@step` match against the scenario's steps, because several old ranges had drifted into a neighbouring function. Impl mappings are kept.
- Moves were made with a block-moving script (top-level fn plus its leading comment and attribute lines). The script also checked that each old range resolved to exactly one function.
