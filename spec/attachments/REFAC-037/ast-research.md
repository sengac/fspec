# REFAC-037 AST research

## fspec astgrep --pattern 'assert_focus($$$A)' --lang rust --path rust/fspec-tui/tests/menu007_agent_board_view_item.rs
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:158:5:assert_focus(
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:164:5:assert_focus(&app, MenuFocus::ZoneB(1), "2nd Right must land on chip #2")
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:166:5:assert_focus(
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:172:5:assert_focus(
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:180:5:assert_focus(
rust/fspec-tui/tests/menu007_agent_board_view_item.rs:201:5:assert_focus(


Before the split these six sites were 5-line `assert_eq!(app.navigator().agent.menu_focus(), Some(..), ..)` blocks. The moved helpers had no callers outside this file.
