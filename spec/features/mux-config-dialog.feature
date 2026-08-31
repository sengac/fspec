@done
@tui
@ui-enhancement
@keyboard-navigation
@MUX-004
Feature: Mux configuration dialog + /mux slash-popup entry
  """
  New MuxConfigDialog Component (Priority::Foreground, stable id 'mux-config-dialog', components/mux_config_dialog.rs <300 LoC + a rows sibling file) built on the base dialog architecture: dialog_theme::FspecDialog/render_dialog rows (Enabled, Orientation, one row per pane). State: draft MuxConfig + field cursor (usize over the 2 + n_panes rows). Emitted actions: Action::MuxConfigApplied(draft MuxConfig) on Enter, Action::MuxConfigAppliedAndSaved on 's' (App::dispatch persists via the existing save_mux_config path), Compositor-removal callback on every commit/close. App wiring: handle_open_mux_config_dialog() in a new app/dispatch_mux_config.rs (idempotent via compositor.contains, seeds from navigator.mux.config()). mux_parser: MuxSubcommand::Toggle renamed to Config (bare /mux). Slash registry: SlashCommandAction::Mux + SLASH_COMMANDS row 'mux' / 'Configure the mux layout'; handle_slash_command(Mux) opens the dialog. App::dispatch arms for the two new Actions live in dispatch_mux.rs. MUX-001 R1 scenarios/tests in rust-mux-mode.feature + tests/mux001.rs + tests/bug166_* get updated to the dialog behavior.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: bare /mux (no args) OPENS the MuxConfigDialog instead of toggling (MUX-001 R1 superseded). /mux on and /mux off remain explicit slash commands that toggle without opening the dialog.
  #   2. R2: the MuxConfigDialog is a Priority::Foreground Component rendered via the base dialog architecture (components/dialog_theme::render_dialog + FspecDialog rows), the same pattern as ThinkingLevelDialog/RoleDialog. It seeds from the live mux config and is idempotent on reopen (one instance, addressed by a stable id).
  #   3. R3: the dialog body shows exactly these rows in order: (1) 'Enabled' with On/Off, (2) 'Orientation' with Horizontal/Vertical, (3) one row per configured pane in grid order showing the pane kind (Board/Agent/Files/Checkpoints).
  #   4. R5: Dialog edits work on a DRAFT copy of the live MuxConfig. Enter applies the draft to the live layout (Action::MuxConfigApplied) and closes; S applies + persists to fspec-config.json tui.mux (Action::MuxConfigAppliedAndSaved) and closes; Esc closes without applying or saving. While the dialog is open, the live grid is untouched (cancel-safe). The applied draft always keeps 2..=4 panes and the scale is re-derived for the new pane count (BUG-166 percentage-scale semantics; split percents are NOT editable in the dialog — layout-only scope, user decision 2026-08-28).
  #   5. R4: Dialog editing keys: Up/Down move the field cursor across the rows (Enabled, Orientation, then each pane row, wrapping); Left/Right cycle the highlighted row's value (Enabled: On<->Off, Orientation: Horizontal<->Vertical, pane row: cycles kind Board->Agent->Files->Checkpoints->Board); 'a' appends a new pane row (kind Board, max 4 panes); Backspace removes the highlighted pane row (min 2 panes); Enter commits + closes; 's' commits + persists to fspec-config.json tui.mux + closes; Esc closes without committing. Mouse wheel scrolls the cursor like Up/Down. No other keys are consumed.
  #   6. R6: /mux is added to the SLASH_COMMANDS palette registry (SlashCommandAction::Mux with a description) so the slash popup lists and filters it like every other command; picking it (Enter) emits Action::SlashCommandSelected(Mux) which opens the MuxConfigDialog. The agent help dialog's slash-command list picks the row up automatically from the registry (no manual help edit). The /mux help notice documents that bare /mux opens the config dialog.
  #   7. R7: Opening the dialog with mux OFF is allowed (it shows Enabled: Off + the last/saved layout). Committing with Enabled: On enters mux mode with the draft layout (pre-mux view = current active view); committing with Enabled: Off while mux is on exits mux mode back to the pre-mux view. Committing while the enabled state is unchanged only refreshes the layout.
  #   8. Dialog replaces toggle (user decision 2026-08-28): bare /mux ALWAYS opens the MuxConfigDialog; the on/off toggle lives inside the dialog's Enabled row; /mux on and /mux off remain explicit commands. MUX-001 R1 + its tests get updated to match.
  #   9. Layout only (user decision 2026-08-28): the dialog manages panes/order/orientation/enabled; split percentages stay divider-drag-driven. Applying a layout change re-derives the scale for the new pane count (BUG-166 semantics), existing percents are not hand-edited in the dialog.
  #
  # EXAMPLES:
  #   1. User types /mux in the agent input while in the single Board view. The MuxConfigDialog opens (Foreground, overlaying the board) with rows 'Enabled: Off', 'Orientation: Horizontal', 'Pane 1: Board', 'Pane 2: Agent' and footer '↑↓ Field · ←→ Value · A Add · ⌫ Remove · S Save · Enter Apply · Esc Cancel'. The board view is still visible underneath; the mux is not yet active.
  #   2. With the dialog open on the default 2 panes, the user presses 'a' (a third pane row appears, kind Board), moves down to that row and presses Right until it reads Files, then presses Enter to apply. The screen becomes a 3-pane horizontal grid Board | Agent | Files, the dialog closes, and the Board pane is focused. Split percentages are unchanged by the dialog.
  #   3. Typing /mux on (explicit) still enters mux mode immediately without opening the dialog; typing /mux off exits back to the pre-mux view without the dialog. Only bare /mux opens the dialog. (Supersedes the MUX-001 'bare /mux toggles' example: the toggle moved into the dialog's Enabled row.)
  #   4. With the dialog open: Up from the Enabled row wraps to the LAST pane row; Down from the last pane row wraps back to Enabled. On the Orientation row, Right toggles Horizontal<->Vertical. On the 'Pane 2: Agent' row, Right shows 'Pane 2: Files', then 'Pane 2: Checkpoints', then 'Pane 2: Board'.
  #   5. While the MuxConfigDialog is open, submitting /mux again (or picking it from the slash popup) is a no-op — exactly one dialog instance exists. After pressing Esc, submitting /mux again opens a fresh dialog seeded from the CURRENT (possibly committed) config.
  #   6. In the agent input the user types '/' and sees the slash popup. Typing 'mux' filters the list to a single row: '/mux  Configure the mux layout' (or similar). Pressing Enter on that row opens the MuxConfigDialog and clears the input — exactly like picking /model or /role opens their views. The agent help dialog (? ) now also lists '/mux' with the same description.
  #   7. With the dialog open on a 2-pane grid, pressing Backspace on the 'Pane 2: Agent' row does nothing (the minimum of 2 panes is enforced — no row is removed). With 4 panes configured, pressing 'a' does nothing (the maximum of 4 is enforced).
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should bare /mux open the config dialog (replacing the old toggle), or should a separate subcommand like /mux config open it while bare /mux keeps toggling?
  #   A: Dialog replaces toggle (user decision 2026-08-28): bare /mux ALWAYS opens the MuxConfigDialog; the on/off toggle lives inside the dialog's Enabled row; /mux on and /mux off remain explicit commands. MUX-001 R1 + its tests get updated to match.
  #
  #   Q: Should the dialog also edit the pane split percentages, or stay layout-only (panes, order, orientation, enabled) with splits kept to the mouse divider drag?
  #   A: Layout only (user decision 2026-08-28): the dialog manages panes/order/orientation/enabled; split percentages stay divider-drag-driven. Applying a layout change re-derives the scale for the new pane count (BUG-166 semantics), existing percents are not hand-edited in the dialog.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents
    I want to open a mux configuration dialog from the /mux command
    So that I can configure which views appear in the mux grid, and enable or disable the mode, visually

  # ========================================
  # SCENARIOS (one per business rule)
  # ========================================
  # R1: bare /mux opens the MuxConfigDialog (supersedes MUX-001 R1 toggle)
  Scenario: bare /mux opens the MuxConfigDialog over the current view
    Given the TUI is showing the single Board view with one open agent session
    When I submit the slash command "/mux"
    Then the MuxConfigDialog is open on the compositor
    And the dialog shows an Enabled row with Off
    And the dialog shows an Orientation row with Horizontal
    And the dialog shows a Pane row for Board
    And the dialog shows a Pane row for Agent
    And the TUI is still in the single Board view (the dialog is an overlay and mux is not yet active)

  # R1: /mux on and /mux off keep their explicit toggle semantics
  Scenario: /mux on enters mux mode without opening the dialog
    Given the TUI is showing the single Board view with one open agent session
    When I submit the slash command "/mux on"
    Then mux mode is active with the default preset
    And no MuxConfigDialog is open on the compositor

  Scenario: /mux off exits mux mode without opening the dialog
    Given mux mode is active with the default two panes and the pre-mux view is the Board view
    When I submit the slash command "/mux off"
    Then mux mode is inactive
    And the TUI shows the single Board view
    And no MuxConfigDialog is open on the compositor

  # R3: the dialog body shows exactly the Enabled, Orientation and pane rows
  Scenario: the dialog body shows the enabled, orientation and pane rows in order
    Given the TUI is showing the single Board view with one open agent session
    When I submit the slash command "/mux"
    Then the dialog body rows appear in the order: Enabled, Orientation, Pane 1, Pane 2
    And the dialog footer lists the editing keybindings (cursor, value, add, remove, save, apply, cancel)

  # R4: cursor movement wraps; Left/Right cycle the highlighted row's value
  Scenario: cursor wraps and values cycle per highlighted row
    Given the MuxConfigDialog is open with the default two panes
    When I press Up on the Enabled row
    Then the cursor wraps to the last Pane row
    When I press Down on the last Pane row
    Then the cursor wraps back to the Enabled row
    When I press Down to the Orientation row and press Right
    Then the Orientation row shows Vertical
    When I press Right again
    Then the Orientation row shows Horizontal
    When I move the cursor to the Agent Pane row and press Right three times
    Then the Pane row shows Board again (the cycle is Board -> Agent -> Files -> Checkpoints -> Board)

  # R4: 'a' appends a pane row (max 4) and Backspace removes it (min 2)
  Scenario: adding a third pane and applying it rebuilds the grid
    Given the MuxConfigDialog is open with the default two panes
    When I press "a" to append a pane row
    Then the dialog shows a third Pane row with kind Board
    When I move the cursor to the third Pane row and cycle its kind to Files
    And I press Enter to apply
    Then the MuxConfigDialog is closed
    And mux mode is active with three panes: Board, Agent and Files
    And the split scale is re-derived for three panes (one entry per inter-pane gap, last pane takes the remainder)

  Scenario: pane count stays within the 2 to 4 bounds
    Given the MuxConfigDialog is open with the default two panes and the cursor on a Pane row
    When I press Backspace
    Then the pane row is NOT removed (minimum of two panes)
    Given the MuxConfigDialog is open with four panes
    When I press "a"
    Then no fifth Pane row is added (maximum of four panes)

  # R5: Enter applies the draft; 's' applies + persists; Esc cancels
  Scenario: Enter applies the draft layout and closes the dialog
    Given the MuxConfigDialog is open with the default two panes while mux is off
    When I cycle the second Pane row's kind to Checkpoints
    And I press Enter to apply
    Then the MuxConfigDialog is closed
    And the live mux layout has panes Board and Checkpoints
    And the shared fspec-config.json is NOT written (apply does not save)

  Scenario: S applies the draft and persists it to the shared config
    Given the MuxConfigDialog is open with the default two panes while mux is off
    When I set the Orientation row to Vertical
    And I press "s" to save
    Then the MuxConfigDialog is closed
    And the live mux layout is vertical with panes Board and Agent
    And the shared fspec-config.json tui.mux key contains the vertical orientation and the two-pane list

  Scenario: Esc closes the dialog without applying or saving
    Given the MuxConfigDialog is open with the default two panes
    And the live mux layout is horizontal with panes Board and Agent
    When I cycle the second Pane row's kind to Files
    And I press Esc to cancel
    Then the MuxConfigDialog is closed
    And the live mux layout is still horizontal with panes Board and Agent

  # R2: idempotent reopen
  Scenario: reopening while the dialog is open is a no-op
    Given the MuxConfigDialog is open
    When I submit the slash command "/mux" again
    Then exactly one MuxConfigDialog layer exists on the compositor
    When I press Esc to close it
    And I submit the slash command "/mux" again
    Then a fresh MuxConfigDialog is open seeded from the current config

  # R7: commit with Enabled On enters mux; Enabled Off exits mux
  Scenario: committing with Enabled On enters mux mode with the draft layout
    Given the TUI is showing the single Agent view with one open agent session
    And the MuxConfigDialog is open with Enabled set to Off and panes Board, Agent and Files
    When I move the cursor to the Enabled row and press Right to set it to On
    And I press Enter to apply
    Then mux mode is active with three panes: Board, Agent and Files
    And the pre-mux view recorded for exit is the single Agent view

  Scenario: committing with Enabled Off exits mux mode back to the pre-mux view
    Given mux mode is active with the default two panes and the pre-mux view is the Board view
    And the MuxConfigDialog is open with Enabled set to On
    When I move the cursor to the Enabled row and press Left to set it to Off
    And I press Enter to apply
    Then mux mode is inactive
    And the TUI shows the single Board view

  # R6: /mux appears in the slash popup registry with a description
  Scenario: the slash popup lists /mux with a description and picking it opens the dialog
    Given the agent input contains "/" (the slash popup is open)
    When I type "mux" as the popup filter
    Then the popup shows exactly one row: "/mux" with a description of the mux layout configuration
    When I press Enter on the highlighted /mux row
    Then the agent input is cleared
    And the MuxConfigDialog is open on the compositor

  Scenario: the agent help dialog lists /mux from the registry
    Given the agent help content is generated from the slash command registry
    When I read the slash-command lines of the agent help
    Then the lines include a /mux row with the same description as the slash popup registry

  # R6: /mux help documents that bare /mux opens the config dialog
  Scenario: /mux help documents the config dialog
    Given the TUI is showing the single Board view with one open agent session
    When I submit the slash command "/mux help"
    Then a one-line notice appears in the agent scrollback
    And the notice mentions the config dialog (bare /mux) and the on/off subcommands
