# RPC-397 — View-specific accurate help content for board and agent

## Problem
The shared `HelpDialog` shows ONE hardcoded 7-line list
(`help_dialog.rs:24-32`) that mixes board and agent bindings and is **wrong**:
- `q Quit fspec-tui` — **q no longer quits** (RPC-102). Global quit is **Ctrl+D**.
- Board-only keys (`.`, `[`/`]`, `f`/`c`/`d`/`a`, Shift+Right) are missing.
- Agent slash commands are entirely absent.
- The same list is shown for both `?` (board) and `/help` (agent).

## Goal
Parameterize `HelpDialog` into **Board** and **Agent** variants with accurate,
view-specific content, and push the correct variant from each call site.

- **Board variant** (`?`): all board keybindings + short explanations. NO slash commands.
- **Agent variant** (`/help`): all agent keybindings + short explanations, AND every
  slash command with its description.

## Dependency
**Depends on RPC-396** — the agent variant is long and must scroll; RPC-396 delivers the
scroll + space-filling mechanics.

## Implementation plan
1. Add `HelpKind { Board, Agent }` to `help_dialog.rs`; constructors
   `HelpDialog::for_board()` / `HelpDialog::for_agent()` (keep `new()` if needed for compat,
   or map it to Board). Preserve canonical id `"help-dialog"`.
2. Define two content tables (arrays of `(keys, description)` and, for agent, a slash-command
   section). Source content from:
   - Board keys: `views/board.rs` handle_event + `board/mouse.rs`.
   - Agent keys: `views/agent/dispatch.rs` + `dispatch_select.rs` + `multiline_input.rs`.
   - Slash commands: `views/agent/slash_commands.rs` `SLASH_COMMANDS` (17 entries).
3. `render` picks the table by `kind`.
4. Update call sites:
   - `app/events.rs:127` → `HelpDialog::for_board()`.
   - `app/dispatch_slash_commands.rs:33` → `HelpDialog::for_agent()`.
5. Keep `help_dialog.rs` < 300 LoC — move the content tables into a
   `help_content.rs` submodule if needed.

## Acceptance Criteria (Gherkin)
Feature: `spec/features/view-specific-help-content.feature`
1. Board `?` → shows board keys (arrows/hjkl, `.` new agent, Shift+Right, `[`/`]`, f/c/d/a, ESC),
   does NOT show slash commands.
2. Agent `/help` → shows agent keys (Enter send, Shift+Enter, Ctrl+C, scrollback paging,
   Shift+arrows, Tab select) AND the full slash-command list with descriptions.
3. Neither variant shows `q Quit`; quit entry reads `Ctrl+D Quit`.

## Business Rules
- R1: Board `?` → board-only content; Agent `/help` → agent keys + slash commands.
- R2: Every entry has key(s) + a one-line explanation.
- R3: Content must be accurate to source — no `q Quit`; use `Ctrl+D Quit`.
- R4: `?` pushes board variant; `/help` pushes agent variant.
- R5: Agent variant lists all 17 slash commands with descriptions.

## ACDD Workflow
1. Failing tests first: assert board buffer contains board keys and NOT slash commands;
   agent buffer contains agent keys + slash commands; neither contains `q Quit`.
2. Implement variants + rewire call sites.
3. `cargo test/clippy/fmt` clean; regenerate snapshot(s).
4. Link coverage per scenario.
