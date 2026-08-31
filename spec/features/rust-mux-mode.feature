@done
@tui
@ui-enhancement
@keyboard-navigation
@mouse-events
@MUX-001
Feature: Mux mode — multiplexed top-level views with /mux configuration
  """
  New views/multiplex/ module (mod/layout/render/keys/mouse/presets) + store/mux_state.rs (MuxState + serde, shared fspec-config.json under tui.mux) + app/mux_parser.rs (/mux grammar) + app/dispatch_mux.rs. ViewMode::Mux in Navigator; MultiplexLayout owns MuxConfig + cached pane rects. App::dispatch is the single mutation surface; keyboard isolation = unfocused panes receive NO events.
  NOTE: the board pane minimum is 64 columns (7×8 + 6 separators + 2 borders, per views/board/grid.rs::calculate_column_widths + the render bail at views/board/render.rs); the design-doc figure of 52 was an arithmetic slip. Non-board panes use a 20-col (horizontal) / 10-row (vertical) floor.
  NOTE (MUX-003, 2026-08-26): the mux layout NO LONGER enforces those minimums — panes divide the terminal equally by pane count and an explicit split percent is honored as-is (the board view simply degrades to a blank pane below its 64-col fit width). The 64-col figure above now documents only the board view's own render bail.
  NOTE: the divider is mouse-drag-resizable ONLY. Tab pane/divider cycling and keyboard divider resize were removed (2026-08-26 user directive): Tab is reserved for the agent view's turn-select mode and Esc is never used as a mux exit. The ONLY mux keybindings are Shift+Left/Right (pane focus cycling); the 'm' toggle was also removed — mux exits are /mux (toggle) or /mux off.
  NOTE (BUG-166, 2026-08-27): the single first-pane divider + "missing splits mean equal share" model is SUPERSEDED — every inter-pane gap has its own draggable divider, drag release persists the released position (no snap-back to equal), and splits are a percentage scale (n-1 entries, last pane = remainder) that rescales proportionally when the pane count changes. See spec/features/mux-dividers-percentage-scale.feature.
  NOTE (MUX-004, 2026-08-28): bare /mux no longer TOGGLES — it opens the MuxConfigDialog (the on/off toggle now lives inside the dialog's Enabled row; /mux on|off stay explicit). See spec/features/mux-config-dialog.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: /mux toggles mux on/off; /mux off returns to the pre-mux view
  #   2. R2: keyboard input goes only to the focused pane; clicking a pane focuses it and routes the click to that pane
  #   3. R3: Shift+Left/Right move pane focus one pane at a time. SUPERSEDED by MUX-002 (2026-08-27): Shift+Left at the first pane STOPS (no wrap-around); Shift+Right at the rightmost pane prompts to create a new agent; Shift+Left/Right on the rightmost agent pane rotate the agent window.
  #   4. R4: the divider is drag-resizable (mouse) only — keyboard divider resize and Tab pane/divider cycling were removed (Tab is reserved for the agent view's turn-select mode)
  #   5. R5: no board pane may be narrower than 64 columns (board minimum); splits are clamped, never allowed to produce a sub-minimum pane — SUPERSEDED by MUX-003 (2026-08-26): the minimum clamps are removed; panes divide the area equally by pane count and an explicit split percent is honored as-is. The mouse divider drag (10..=90 percent clamp) is the only non-equal split source.
  #   6. R6: config persists across restarts when saved; missing file -> default preset (horizontal, Board|Agent, 50/50)
  #   7. R7: parse errors in /mux leave the current config untouched and show a one-line error
  #   8. R8: Enter on a board work unit in mux mode focuses the agent pane for that unit (does NOT flip the whole screen to Agent)
  #   9. R9: mux mode coexists with the Compositor: dialogs (help, exit confirmation, create-session) still overlay the full screen and capture input above mux
  #   10. R10: existing single-view behavior is byte-for-byte unchanged when mux is off (all existing tests stay green)
  #   11. Default preset: horizontal orientation, Board | Agent, 50/50 split. Fresh entry focuses the Board pane (the view the user came from); the agent pane is the persisted "home" focus restored on reload.
  #   12. The ONLY mux keybindings are Shift+Left/Right (pane focus cycling). The 'm' toggle, Tab pane/divider cycling, and keyboard divider resize were all removed (2026-08-26 user directive) — everything else is /mux-driven. Mouse divider drag is retained.
  #   13. The mux config persists in the shared CONFIG-008 fspec-config.json under tui.mux (user scope, project-scope override via deep merge). Missing key -> default preset. Loaded at bootstrap; auto-saved on /mux save and on mux exit.
  #   14. The divider is mouse-drag-resizable ONLY. Tab pane/divider cycling and keyboard divider resize were removed (2026-08-26 user directive): Tab is reserved for the agent view's turn-select mode; Esc is never a mux exit (exits are 'm' or /mux off).
  #   15. R15 (BUG-164, 2026-08-27): BackToBoard (session close, detach, ESC with no session) FOCUSES the board pane within the active mux grid instead of flipping the whole view out of Mux. The grid, its pane list, and its layout are retained; only the in-grid focus moves to the Board pane.
  #   16. R16 (BUG-165, 2026-08-27): Esc pressed while the Board pane is focused in mux mode pushes the BoardExitConfirmationDialog onto the compositor — identical to the single Board view (R9: dialogs overlay the mux). This applies regardless of whether agent panes are rendered (with no open agents the agent slots are dropped, only the Board pane remains, and Esc must still offer the exit dialog). Esc on a focused AGENT pane keeps the existing agent exit-confirmation cascade (Detach / Close Session / Cancel).
  #
  # EXAMPLES:
  #   1. User types /mux in the agent input; the MuxConfigDialog opens (MUX-004 supersession — the bare /mux no longer toggles). Entering mux mode now happens via /mux on (default preset: 2-pane horizontal Board | Agent, agent pane the persisted home focus), or by committing the dialog with Enabled: On. /mux off returns to the single Agent view that was active before mux was entered.
  #   2. With Board | Agent mux active and the Board pane focused, typing 'j' moves the board selection down; the agent scrollback and input are untouched. Clicking inside the agent pane moves focus there and the same click is delivered to the agent pane.
  #   3. With 3 panes Board | Agent | Files and focus on the Files pane, Shift+Left moves focus to the Agent pane; Shift+Left again to Board; Shift+Left again stops at the Board pane (no wrap-around — MUX-002).
  #   4. Dragging the divider between Board and Agent with the mouse live-resizes the split; releasing at a position that would make the Board pane 40 cols clamps it to the 64-col board minimum.
  #   5. User types /mux 3 in the agent input; the screen becomes Board | Agent | ChangedFiles with the agent pane focused; typing 'hello' lands in the agent pane only. /mux 4 adds a fourth Checkpoints pane; /mux 2 collapses back to Board | Agent.
  #   6. User types /mux board agent 40; the grid shows Board on the left at 40% and Agent on the right at 60%. /mux v flips the same panes into a top/bottom stack; /mux h flips back to side-by-side.
  #   7. User types /mux board zzz; a one-line error '/mux: unknown pane kind: zzz' appears in the agent scrollback and the grid keeps its previous configuration unchanged.
  #   8. User types /mux save; the current grid config is written to the shared fspec-config.json under tui.mux. Restarting the TUI loads that key and /mux on restores the saved grid. With no saved tui.mux key present, /mux on applies the default preset (horizontal, Board | Agent, 50/50).
  #   9. In mux mode with Board | Agent, pressing Enter on a selected board work unit binds that unit to the agent pane's session and focuses the agent pane; the board stays visible in its pane (the whole screen does NOT flip to Agent). Pressing /mux off exits mux mode back to the single view that was active before mux was entered (MUX-004: bare /mux now opens the config dialog instead of toggling — commit it with Enabled: Off to exit the same way).
  #
  # QUESTIONS (ANSWERED):
  #   Q: What should the default mux preset be?
  #   A: Default preset: horizontal orientation, Board | Agent, 50/50 split. Fresh entry focuses the Board pane (the view the user came from); the agent pane is the persisted "home" focus restored on reload.
  #
  #   Q: Is 'm' acceptable as the mux toggle key (vs a different key)?
  #   A: NO (revised 2026-08-26, user directive) — the 'm' toggle was removed. The ONLY mux keybindings are Shift+Left/Right (pane focus cycling); everything else is /mux-driven.
  #
  #   Q: Where should the mux config persist to?
  #   A: The shared CONFIG-008 fspec-config.json under tui.mux (user scope ~/.fspec/fspec-config.json; project spec/fspec-config.json overrides via deep merge) — same pattern as tui.defaultThinkingLevel. Missing key -> default preset. NO dedicated spec/mux.json file.
  #
  #   Q: Should the divider be focusable via Tab in MVP?
  #   A: NO (revised 2026-08-26, user directive) — Tab pane/divider cycling and keyboard divider resize were removed. Tab is reserved for the agent view's turn-select mode, and Esc is never a mux exit. The divider is mouse-drag-resizable only; mux exits are /mux (toggle) or /mux off.
  #
  # ASSUMPTIONS:
  #   1. Mux MVP hosts at most ONE live agent pane (AgentView is a single instance); extra agent panes are out of scope. Pane kinds: board, agent, files (ChangedFiles), checkpoints.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents
    I want to see the board and an agent conversation side-by-side in a configurable grid
    So that monitor progress without switching views

  # ========================================
  # SCENARIOS (one per business rule)
  # ========================================
  # R1 (MUX-004 supersession): bare /mux now OPENS the MuxConfigDialog
  # (spec/features/mux-config-dialog.feature) — the explicit /mux on
  # keeps the old "enable with the default preset" behavior.
  # R1: /mux on enables mux mode with the default preset
  Scenario: /mux on enables mux mode with the default preset
    Given the TUI is showing the single Board view
    When I submit the slash command "/mux on"
    Then mux mode is active with the default preset
    And the grid shows two horizontal panes: Board on the left and Agent on the right
    And the split is 50/50 with the Board pane focused on fresh entry (the agent pane is the persisted home focus)
    And the mux footer row is painted at the bottom of the screen

  # R1: /mux off returns to the pre-mux view
  Scenario: /mux off returns to the pre-mux view
    Given the TUI is showing the single Agent view
    When I submit the slash command "/mux on"
    And I submit the slash command "/mux off"
    Then mux mode is inactive
    And the TUI shows the single Agent view that was active before mux was entered
    And the board and agent store state is unchanged by the round trip

  # R3: /mux v sets vertical orientation
  Scenario: /mux v sets the grid to vertical
    Given mux mode is active with panes Board and Agent
    When I submit the slash command "/mux v"
    Then the grid is vertical with Board on top and Agent on the bottom

  # R3: /mux h sets horizontal orientation
  Scenario: /mux h sets the grid to horizontal
    Given mux mode is active with panes Board and Agent
    When I submit the slash command "/mux h"
    Then the grid is horizontal with Board on the left and Agent on the right

  # R4: /mux 3 sets the pane count to three
  Scenario: /mux 3 sets the pane count to three
    Given mux mode is active with the default two panes
    When I submit the slash command "/mux 3"
    Then the grid shows three panes: Board, Agent and ChangedFiles

  # R4: /mux 4 sets the pane count to four
  Scenario: /mux 4 sets the pane count to four
    Given mux mode is active with three panes
    When I submit the slash command "/mux 4"
    Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints

  # R4: /mux 2 collapses the pane count to two
  Scenario: /mux 2 collapses the pane count to two
    Given mux mode is active with four panes
    When I submit the slash command "/mux 2"
    Then the grid shows two panes: Board and Agent

  # R5: explicit pane list + split percent
  Scenario: /mux board agent 40 sets an explicit pane list and split
    Given mux mode is active
    When I submit the slash command "/mux board agent 40"
    Then the grid shows Board at 40 percent on the left and Agent at 60 percent on the right

  # R5: explicit two-pane list
  Scenario: the mux files checkpoints command sets a two-pane list
    Given mux mode is active
    When I submit the slash command "/mux files checkpoints"
    Then the grid shows ChangedFiles on the left and Checkpoints on the right

  # R7: invalid pane kind leaves config unchanged and shows an error
  Scenario: /mux with an invalid pane kind leaves the config unchanged and shows an error
    Given mux mode is active with panes Board and Agent at a 50/50 split
    When I submit the slash command "/mux board zzz"
    Then a one-line error "/mux: unknown pane kind: zzz" is shown in the agent scrollback
    And the grid still shows Board and Agent at a 50/50 split
    And the out-of-range command "/mux board agent 5" is rejected with a one-line error and leaves the config unchanged

  # R2: keyboard input routes only to the focused pane
  Scenario: pressing j on the focused board pane moves the selection
    Given mux mode is active with Board and Agent panes
    And the Board pane is focused
    When I press the key "j"
    Then the board selection moves down one row
    And the agent input and scrollback are unchanged

  # R2: keyboard input on the board pane leaves the agent untouched
  Scenario: pressing k on the focused board pane leaves the agent untouched
    Given mux mode is active with Board and Agent panes
    And the Board pane is focused
    When I press the key "k"
    Then the agent input and scrollback are still unchanged

  # R2: clicking a pane focuses it and routes the click
  Scenario: clicking inside the Agent pane focuses it and routes the click
    Given mux mode is active with Board and Agent panes
    And the Board pane is focused
    When I click inside the Agent pane rect
    Then the Agent pane is focused
    And the click event is forwarded to the Agent pane handler

  # R2: clicking in a gap leaves focus unchanged
  Scenario: clicking in a gap outside every pane rect leaves focus unchanged
    Given mux mode is active with Board and Agent panes
    And the Board pane is focused
    When I click in a gap outside every pane rect
    Then the focus stays on the Board pane

  # R3: Shift+Left cycles focus from the last pane to the middle pane
  Scenario: Shift+Left cycles focus from ChangedFiles to Agent
    Given mux mode is active with three panes Board, Agent and ChangedFiles
    And the ChangedFiles pane is focused
    When I press Shift+Left
    Then the Agent pane is focused

  # R3: Shift+Left cycles focus from the middle pane to the first pane
  Scenario: Shift+Left cycles focus from Agent to Board
    Given mux mode is active with three panes Board, Agent and ChangedFiles
    And the Agent pane is focused
    When I press Shift+Left
    Then the Board pane is focused

  # R3: Shift+Left at the first pane stops without wrapping (MUX-002)
  Scenario: Shift+Left at the first pane stops without wrapping
    Given mux mode is active with three panes Board, Agent and ChangedFiles
    And the Board pane is focused
    When I press Shift+Left
    Then the Board pane is still focused

  # R4: divider drag resizes the split live
  Scenario: dragging the divider left resizes the split live
    Given mux mode is active with Board and Agent panes at a 50/50 split on a 120-column terminal
    When I press the mouse down on the divider and drag it left
    Then the Board pane shrinks and the Agent pane grows live during the drag

  # R4 (BUG-166): releasing the divider stores the released percent — the
  # old snap-back to an equal split is fixed; the 64-column minimum clamp
  # was removed by MUX-003 and is not re-introduced
  Scenario: releasing the divider stores the released percent without snapping back
    Given mux mode is active with Board and Agent panes at a 50/50 split on a 120-column terminal
    When I release the mouse at a position that would make the Board pane 40 columns
    Then the Board pane keeps its released 34 percent of the width (40/119, rounded — no 64-column minimum, no equal-split reset)
    And the drag state is cleared after the release

  # R5 (superseded by MUX-003): an explicit split percent is honored as-is —
  # the old 64-column board minimum clamp is removed
  Scenario: split clamping never produces a pane narrower than the minimum width
    Given a 110-column terminal with two mux panes
    When the split is requested at 10/90
    Then the first pane is clamped to the 64-column board minimum and the second pane takes the 44-column remainder
    And no board pane rect is ever narrower than the 64-column minimum

  # R6: /mux save persists the config to the shared fspec-config.json
  Scenario: /mux save persists the config to the shared fspec-config.json
    Given mux mode is active with panes Board and Agent at a 40/60 vertical split
    When I submit the slash command "/mux save"
    Then the shared fspec-config.json exists and its tui.mux key contains the saved orientation, splits and pane list

  # R6: a fresh bootstrap restores the saved mux config
  Scenario: a fresh bootstrap restores the saved mux config
    Given a fresh TUI bootstrap with a saved tui.mux config of 40/60 vertical Board and Agent
    When I submit the slash command "/mux on"
    Then the grid restores the saved 40/60 vertical Board | Agent layout

  # R6: a fresh bootstrap with no saved config applies the default preset
  Scenario: a fresh bootstrap with no saved config applies the default preset
    Given a fresh TUI bootstrap with no tui.mux config present
    When I submit the slash command "/mux on"
    Then the grid applies the default preset: horizontal Board | Agent at 50/50

  # R8: Enter on a board work unit in mux mode focuses the agent pane
  Scenario: Enter on a board work unit in mux mode focuses the agent pane
    Given mux mode is active with Board and Agent panes
    And the Board pane is focused with work unit AUTH-001 selected
    When I press Enter
    Then the work unit AUTH-001 is bound to the agent pane session
    And the Agent pane is focused
    And the TUI is still in mux mode showing both panes
    And the board stays visible in its pane

  # BUG-164: BackToBoard is a "focus the board" semantic — it must never
  # flip the whole view out of the active mux grid.
  @BUG-164
  Scenario: closing a session in mux mode retains the mux and focuses the board pane
    Given mux mode is active with Board and Agent panes and two agent sessions are open
    And the Agent pane is focused
    When the exit dialog is answered with Close Session
    Then the destroyed session is removed from the open-session list
    And the TUI is still in mux mode with the same panes and layout
    And the Board pane is focused within the grid
    And no single-view flip to Board occurs

  @BUG-164
  Scenario: detaching from a session in mux mode retains the mux and focuses the board pane
  # BUG-164: the Detach choice routes the same BackToBoard action — the
  # grid must survive that transition too.
    Given mux mode is active with Board and Agent panes and one agent session is open
    And the Agent pane is focused
    When the exit dialog is answered with Detach
    Then the session remains open in the store
    And the TUI is still in mux mode with the same panes and layout
    And the Board pane is focused within the grid

  # R9: dialogs overlay mux mode and capture input
  Scenario: pressing ? in mux mode shows the HelpDialog over the full screen
    Given mux mode is active with Board and Agent panes
    When I press the "?" key
    Then the HelpDialog is shown over the full screen
    And keys typed while the dialog is open do not reach any mux pane

  # R9: closing the dialog leaves mux mode active
  Scenario: closing the dialog leaves mux mode active with the same panes and focus
    Given mux mode is active with Board and Agent panes and the HelpDialog is open
    When I close the dialog
    Then mux mode is still active with the same panes and focus

  # R10: existing single-view behavior is unchanged when mux is off
  Scenario: existing single-view behavior is unchanged when mux is off
    Given the TUI is showing the single Board view with mux inactive
    When I press Enter on a selected work unit
    Then the TUI flips to the single Agent view exactly as before mux existed
    And pressing Esc in the Agent view returns to the single Board view
    And no mux footer row is painted and no pane divider is rendered

  @BUG-165
  Scenario: pressing Esc on the board pane in mux mode with no open agents shows the exit dialog
    Given mux mode is active with the default Board and Agent panes and no agent sessions are open
    When I press the Esc key
    Then the BoardExitConfirmationDialog is shown over the full screen
    And the Board pane is still focused and the mux grid is retained

  @BUG-165
  Scenario: confirming the board exit dialog in mux mode quits the application
    Given mux mode is active with the default Board and Agent panes and no agent sessions are open
    When I press the Esc key
    Then the BoardExitConfirmationDialog is shown over the full screen
    When I confirm the Exit option (pre-selected — Enter commits)
    Then the application exits
    And the BoardExitConfirmationDialog is removed from the compositor

  @BUG-165
  Scenario: pressing Esc on the agent pane in mux mode with an open agent still shows the agent exit dialog
    Given mux mode is active with the default Board and Agent panes and one agent session is open
    When I press the Esc key
    Then the agent exit confirmation dialog (Detach / Close Session / Cancel) is shown
    And the Agent pane is focused
