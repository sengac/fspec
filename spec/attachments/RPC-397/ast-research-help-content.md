# AST Research — RPC-397 View-specific help content

## Call sites to update
- `?` (board): `src/app/events.rs:126-130` — `handle_app_shortcut` pushes `HelpDialog::new()`.
  Fires only while BoardView is active (Stage-4 fallback). → push **Board** variant.
- `/help` (agent): `src/app/dispatch_slash_commands.rs:31-35` — `SlashCommandAction::Help`
  pushes `HelpDialog::new()` (guarded by `compositor.contains("help-dialog")`). → push **Agent** variant.

## Content sources (verbatim from source)

### Board keybindings — `src/views/board.rs` handle_event (102-217) + `board/mouse.rs`
- Shift+Right → open Agent view (114-118)
- Enter → work focused unit (121-127)
- `.` → start new agent (208-212, RPC-395)
- `[` reorder up / `]` reorder down (164-171)
- ←/h prev col, →/l next col, ↓/j next, ↑/k prev (148-163)
- PageUp/PageDown scroll column (130-139); Home/End first/last (140-147)
- f/F Changed Files (173-176); c/C Checkpoints (178-181); d/D FOUNDATION.md (185-190); a/A Attachments (195-205)
- Mouse wheel scroll selection/columns; click focuses column/row (board/mouse.rs:55-99)
- ESC → exit-confirm dialog (app/events.rs:138-149); Ctrl+D quit (150-153); `?` help (126-130)

### Agent keybindings — `src/views/agent/dispatch.rs` (146-282) + dispatch_select/mouse_dispatch/multiline_input
- Enter send / Shift+Enter newline; Ctrl+C interrupt (206-209)
- PageUp/PageDown scrollback (210-217); Home top (219-222); End bottom
- Up/Down at edges scroll one line (231-239)
- Shift+Up/Down input history; Shift+Left/Right cycle sessions (223-228)
- Tab turn-select mode (190-192); in select: ↑/↓ nav turns, Enter open modal, Esc exit (dispatch_select.rs)
- Ctrl+R search history (35-38); `/` slash palette; `@` file search
- ESC back/esc-cascade (200-205); Ctrl+D quit

### Slash commands — `src/views/agent/slash_commands.rs` SLASH_COMMANDS (85-154)
17 commands, each `{ name, description }`:
/help "Show help dialog", /clear "Clear conversation history", /quit "Quit fspec TUI",
/model "Select AI model", /thinking "Set base thinking level", /role "Set or edit session role",
/resume "Resume a previous session", /search "Search command history",
/provider "Configure API providers", /debug "Toggle debug capture mode",
/compact "Compact context window", /isolation "Toggle worktree isolation",
/blocklist "Manage blocklist rules", /detach "Detach session from work unit",
/merge-worktree "Merge worktree changes and close session",
/schedule "Manage scheduled jobs", /loop "Quick recurring schedule (session-scoped)".

## Component to change
`src/components/help_dialog.rs` — parameterize with a `HelpKind { Board, Agent }`.
Keep id `"help-dialog"` (compositor `.contains` guards at events.rs / dispatch_slash_commands.rs).

## Dependency
RPC-396 must land first (scroll + space-filling), because the agent variant content
exceeds one screen and must scroll.

## Tests affected
`help_dialog.rs` inline tests (`help_dialog_body_lists_the_rpc009_keybindings` asserts the OLD
`q` line — must change) + insta snapshot (regenerate for new content).
