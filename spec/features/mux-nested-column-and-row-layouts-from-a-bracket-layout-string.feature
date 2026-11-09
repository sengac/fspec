@CLEAN-014
@done
@tui
@mux
@ui-enhancement
@keyboard-navigation
@dialog
@MUX-010
Feature: Mux layouts nest columns and rows with a bracket layout string, edited in the config dialog with a live preview
  """
  New pure module views/multiplex/layout_tree/ (MuxLayout tree of MuxNode::Pane{kind,weight} | MuxNode::Group{children,weight}; parse.rs = the layout grammar with column-carrying MuxLayoutError; display = canonical string; geometry.rs = pane rects + dividers per group with the alternating direction; spatial.rs = Shift+Up/Down neighbour search; drag.rs = pair re-weighting). It replaces layout.rs, splits.rs, presets.rs and the orientation/splits/panes fields.
  MuxConfig becomes { layout: MuxLayout, focused_pane, enabled } with serde through the canonical string (deserialize = parse; an invalid string is a serde error). CLEAN-014 deleted the one-time migration of the old {orientation, splits, panes} shape (store/mux_state/migrate.rs): an old saved value is an unreadable tui.mux (MUX-011's error and the default layout).
  MultiplexLayout keeps rendered_panes (leaves in reading order) for every index-based caller, derived from a live rendered MuxLayout that drops unfilled agent slots and Esc-closed panes; each rendered node keeps the path of its stored node so a divider drag release rewrites the stored weights.
  app/mux_parser.rs keeps the subcommands (on/off/save/default/help/bare) and hands any other argument text to the layout parser; removed-syntax detection (h/v/horizontal/vertical, a bare integer, or 2+ whitespace-separated words with no , [ ] :) returns MuxError::RemovedSyntax. Fit is checked in dispatch_mux_apply against the mux body area (MIN_PANE 16x5).
  MuxConfigDialog (components/mux_config_dialog*.rs) holds a single-line text field + Enabled toggle + Apply/Save buttons; mux_config_dialog_preview.rs draws the boxes with the same geometry function at the preview area size. mux_config_dialog_rows.rs is deleted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A mux layout is a comma-separated list of panes and [ ... ] groups. The top level is ALWAYS columns (side by side); a [ ... ] group splits its slot the other way, and directions alternate at each nesting level (top: columns; [..]: rows; [[..]] inside rows: columns, ...), at any depth. Whitespace is free. (product owner, 2026-11-08)
  #   2. Pane kinds are the existing MuxPaneKind: board, agent, files, checkpoints. A kind may repeat (e.g. [board, board]). A one-pane layout is valid. (product owner, 2026-11-08; one pane follows from removing the pane-count limit)
  #   3. Panes and groups in one group share its space equally unless they carry a weight suffix: pane:N or [ ... ]:N, at every level. Weights are positive integers; a missing weight is 1. Each item gets floor(available x weight / total weight) cells and the last item takes the remainder, after one 1-cell divider between neighbours. (product owner, 2026-11-08; the rounding keeps today's equal-division geometry)
  #   4. Parse errors are exact, one line, and carry the 1-based column in the layout string: an unbalanced bracket, an empty group, a missing pane, an unknown pane, a bad weight, an unexpected character. The live config stays untouched on error. (product owner, 2026-11-08)
  #   5. The old syntax is removed with no legacy path: /mux h|v|horizontal|vertical, /mux <count> and space-separated pane lists with or without a split percent fail with one exact error that shows the new syntax: /mux: '<args>' is the removed mux syntax; write a layout string, e.g. /mux agent, [board, board], board, files (product owner, 2026-11-08; wording by the helper)
  #   6. Kept subcommands: on, off, save, default, help, and bare /mux opens the MuxConfigDialog. Anything else after /mux is a layout string, which applies the layout and enables mux. (product owner, 2026-11-08)
  #   7. The h/v orientation setting is removed from MuxConfig, the dialog and the saved config. MuxConfig is { layout, focused_pane, enabled } and is saved under tui.mux as {"layout": "<canonical string>", "focused_pane": N, "enabled": bool}. (product owner, 2026-11-08)
  #   8. The canonical layout string separates items with ', ', writes groups as [ ... ] and writes a weight only when it is not 1, e.g. agent:2, [board, board]:3, files. It is what /mux save writes, what the dialog field is seeded with and what the help shows. (helper decision)
  #   9. (CLEAN-014 removed this migration: an old-shape tui.mux is now MUX-011's error plus the default layout.) A saved tui.mux in the old shape (orientation, splits, panes) was migrated ONCE on load: Horizontal becomes a top-level list, Vertical becomes one [ ... ] group, and the split percents become weights reduced by their greatest common divisor; the migrated value is written back at once. Afterwards readers accept only the new shape. (product owner, 2026-11-08; mapping by the helper)
  #   10. The 2..4 pane-count limit is removed. Instead every pane must be at least 16 columns by 5 rows at the current mux body size when a layout is applied (by /mux <layout>, the dialog Apply/Save, or /mux on). A layout that does not fit is refused with an exact error naming the pane and the minimum, e.g. /mux: layout does not fit 80x21: pane 5 (files) would be 15x21; every pane needs at least 16x5. A later terminal shrink renders best-effort without an error. (product owner, 2026-11-08; 16x5 and the apply-time check are helper decisions)
  #   11. Shift+Up/Down move mux focus spatially: to the nearest pane above/below whose span overlaps the focused pane's span on the other axis (the largest overlap wins, then the left-most); with no such pane they do nothing. (product owner, 2026-11-08; overlap rule by the helper) MUX-012 (product owner, 2026-11-08): Shift+Left/Right move to the previous/next pane in the layout string's reading order, stop at both ends, and Shift+Right on a last agent pane prompts for a new agent (see the MUX-002 feature).
  #   12. Pane indexes (focus, flash, click-to-focus, the bar's view labels, MUX-002 agent slots) number the panes in reading order of the layout string (depth first, left to right). Agent slots beyond the open sessions and panes closed with Esc (BUG-183) are dropped from the live layout; a group left empty disappears, the stored layout is untouched. (helper decision, keeps MUX-002/BUG-174/BUG-183)
  #   13. Dragging a divider resizes the two neighbouring items of its group live; on release their weights are rewritten in the stored layout (group weights x100, the pair re-split by their cell sizes, then the group reduced by its greatest common divisor), so a 1:1 pair dragged to 40/60 becomes 2:3. (helper decision, keeps BUG-166 drag-to-resize)
  #   14. MuxConfigDialog is an editable layout-string field seeded with the canonical string, a live preview that draws the panes as boxes labelled with their pane kind and sized by weight, the exact parse or fit error under the field, the Enabled toggle and Apply/Save. The per-pane rows and the Orientation row are removed. Enter on Apply/Save with an invalid string keeps the dialog open with the error. (product owner, 2026-11-08)
  #   15. The /mux help output and the help text describe the layout grammar, weights, the kept subcommands and Shift+arrow focus. (product owner, 2026-11-08)
  #
  # EXAMPLES:
  #   1. /mux agent, [board, board], board, files at 80x24: four columns 19, 19, 19 and 20 wide over the 21-row body (MUX-013: the bar takes two rows at 80 columns); the second column holds two boards stacked as rows of 10 and 10; panes in reading order are agent, board, board, board, files
  #   2. /mux agent:2, [board, board], files at 120x40: agent gets twice the width of each other column (59, 29, 30 columns after the 2 dividers)
  #   3. /mux [agent, [board, files]:2] at 120x40: one column of two rows; the bottom row (weight 2) is split into board | files columns
  #   4. /mux agent, [board, board errors: /mux: unbalanced bracket: '[' at column 8 is never closed; the live layout is unchanged
  #   5. /mux agent, [], files errors: /mux: empty group at column 8
  #   6. /mux agent, bord errors: /mux: unknown pane 'bord' at column 8; panes are board, agent, files, checkpoints
  #   7. /mux agent:0, board errors: /mux: bad weight '0' at column 7; weights are positive integers
  #   8. /mux v, /mux 3 and /mux board agent 40 each error: /mux: '<args>' is the removed mux syntax; write a layout string, e.g. /mux agent, [board, board], board, files
  #   9. Shift+Down from the top board of agent, [board, board], files focuses the bottom board; Shift+Right from the bottom board focuses files, the next pane in layout order; Shift+Left from files focuses the bottom board, the previous pane in layout order (MUX-012)
  #   10. A saved tui.mux {orientation: Vertical, splits: [40], panes: [Board, Agent], focused_pane: 1, enabled: true} loads as layout '[board:2, agent:3]' and the file is rewritten as {layout, focused_pane, enabled}
  #   11. Bare /mux opens the dialog seeded with 'board, agent'; typing ', [files, checkpoints]' redraws the preview with three columns, the last split into two rows; typing a stray ']' shows the exact error under the field and Apply keeps the dialog open
  #   12. Dragging the divider between agent and board in 'agent, board' at 120 columns to 40/60 and releasing stores 'agent:2, board:3'
  #   13. /mux board, agent, files, checkpoints, board at 80x24 is refused (76 columns / 5 = 15): /mux: layout does not fit 80x21: pane 1 (board) would be 15x21; every pane needs at least 16x5. The same five panes fit at 120x40
  #
  # ========================================
  Background: User Story
    As a fspec TUI user running mux mode
    I want to describe a mux layout of nested columns and rows as one bracket layout string, typed after /mux or edited in the config dialog with a live preview
    So that I can stack panes in rows inside a column (and columns inside those rows) at any depth instead of only one flat row or column of 2 to 4 panes

  Scenario: A bracket layout nests rows inside a column
    Given the terminal is 80x24 and mux is off
    When the user submits "/mux agent, [board, board], board, files"
    Then mux is on with four columns 19, 19, 19 and 20 cells wide over the 21-row body below the two bar rows
    And the second column holds two board panes stacked as rows 10 and 10 cells tall
    And the panes in reading order are agent, board, board, board, files

  Scenario: Groups alternate direction at every nesting level
    Given the terminal is 120x40
    When the user submits "/mux [agent, [board, files]:2]"
    Then one column holds two rows, the bottom row twice as tall as the top row
    And the bottom row is split into a board column and a files column

  Scenario: Weights size panes and groups within their group
    Given the terminal is 120x40
    When the user submits "/mux agent:2, [board, board], files"
    Then the columns are 59, 29 and 30 cells wide
    And the saved layout string reads "agent:2, [board, board], files"

  Scenario: Layout parse errors are exact, carry the column and leave the live layout untouched
    Given mux is on with the layout "board, agent"
    When the user submits each of "/mux agent, [board, board", "/mux agent, [], files", "/mux agent, bord" and "/mux agent:0, board"
    Then the errors read "/mux: unbalanced bracket: '[' at column 8 is never closed", "/mux: empty group at column 8", "/mux: unknown pane 'bord' at column 8; panes are board, agent, files, checkpoints" and "/mux: bad weight '0' at column 7; weights are positive integers"
    And the live layout is still "board, agent"

  Scenario: The removed mux syntax fails with the new syntax in the error
    Given mux is on with the layout "board, agent"
    When the user submits each of "/mux v", "/mux horizontal", "/mux 3" and "/mux board agent 40"
    Then each error reads "/mux: '<args>' is the removed mux syntax; write a layout string, e.g. /mux agent, [board, board], board, files" with its own arguments
    And the live layout is still "board, agent"

  Scenario: The kept subcommands still work
    Given mux is off
    When the user submits "/mux on", "/mux save", "/mux default", "/mux off" and "/mux help" in turn
    Then mux turns on with the saved layout, saves it, resets to "board, agent" and turns off
    And the help output lists the layout grammar, the weight suffix, the subcommands on, off, save, default and help, and Shift+arrow focus

  Scenario: A layout whose panes would be smaller than the minimum pane size is refused
    Given the terminal is 80x24 and mux is on with the layout "board, agent"
    When the user submits "/mux board, agent, files, checkpoints, board"
    Then the error reads "/mux: layout does not fit 80x21: pane 1 (board) would be 15x21; every pane needs at least 16x5"
    And the live layout is still "board, agent"
    But at 120x40 the same layout is applied with five columns

  Scenario: Shift+Up/Down move focus spatially and Shift+Left/Right in layout order across nested panes
    Given the terminal is 120x40 and mux is on with the layout "agent, [board, board], files" and the top board focused
    When the user presses Shift+Down
    Then the bottom board is focused
    When the user presses Shift+Right
    Then the files pane is focused
    When the user presses Shift+Left
    Then the bottom board, the previous pane in layout order, is focused
    When the user presses Shift+Up and then Shift+Up again
    Then the top board is focused and the second Shift+Up leaves it focused

  Scenario: Dragging a divider rewrites the weights of its two neighbours
    Given the terminal is 120x24 and mux is on with the layout "agent, board"
    When the user drags the divider to 40 percent of the width and releases it
    Then the stored layout reads "agent:2, board:3"
    And the agent pane is 47 cells wide

  Scenario: Unfilled agent slots drop out of a nested layout without touching the stored layout
    Given the terminal is 120x40 and one agent session is open
    When the user submits "/mux [agent, agent], board"
    Then the first column shows a single agent pane over its full height
    And the stored layout is still "[agent, agent], board"

  Scenario: An old saved tui.mux shows an error and the default layout
    Given fspec-config.json holds tui.mux {"orientation": "Vertical", "splits": [40], "panes": ["Board", "Agent"], "focused_pane": 1, "enabled": true}
    When the TUI loads the mux config
    Then the Session error dialog lists an error naming tui.mux and the unknown field "orientation"
    And the loaded mux layout is the default "board, agent"
    And fspec-config.json still holds the old tui.mux unchanged

  Scenario: A saved tui.mux in the new shape round-trips and an unreadable one is left on disk
    Given fspec-config.json holds tui.mux {"layout": "agent, [board, files]", "focused_pane": 0, "enabled": false}
    When the TUI loads the mux config and the user runs /mux save
    Then the saved tui.mux is byte-for-byte the same value
    And a saved value whose layout string does not parse loads the default layout and is left on disk unchanged

  Scenario: The config dialog edits the layout string with a live preview
    Given the terminal is 120x40 and mux is off
    When the user opens the mux config dialog with bare /mux
    Then the dialog shows the layout field "board, agent", a preview of two boxes labelled board and agent, the Enabled toggle and Apply and Save buttons
    And the dialog has no Orientation row and no per-pane rows
    When the user types ", [files, checkpoints]" at the end of the field
    Then the preview redraws as three columns, the last split into a files box over a checkpoints box

  Scenario: The config dialog shows parse errors under the field and refuses to apply them
    Given the mux config dialog is open with the layout field "board, agent"
    When the user types "]" at the end of the field
    Then the line under the field reads "unbalanced bracket: ']' at column 13 has no matching '['" and the preview keeps the last valid layout
    When the user activates Apply
    Then the dialog stays open with the error and the live mux layout is unchanged
    When the user deletes the "]" and activates Save
    Then the dialog closes, mux uses "board, agent" and tui.mux holds the layout string

  Scenario: The config dialog's focus moves between the field, the Enabled toggle and the buttons
    Given the mux config dialog is open with the layout field "board, agent" focused and Enabled Off
    When the user presses Tab
    Then the Enabled toggle is focused and pressing Space turns it On
    When the user presses Tab twice and then Down
    Then focus wraps from the Save button back to the layout field
    And typing "s" in the field inserts it into the layout text instead of saving
    And the dialog footer lists Tab move, type to edit, Space toggle, Enter apply and Esc cancel

  Scenario: Shift+Up and Shift+Down stay the agent pane's history keys when no pane is above or below
    Given mux is on with the layout "board, agent", one open session and the agent pane focused
    When the user presses Shift+Up
    Then the agent pane stays focused and the agent input receives Shift+Up as its history-recall key
