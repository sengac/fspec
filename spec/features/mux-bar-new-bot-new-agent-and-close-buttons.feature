@done
@tui
@mux
@menu-bar
@MUX-013
Feature: The mux bar has New Bot, New Agent and Close buttons
  """
  views/multiplex/menu_bar_rows.rs (new): the mux bar's Zone C buttons (MUX_ZONE_C: New Bot, New Agent, Close; Close only while the focused pane shows a session), the bar row count from the terminal width (Zone A + gap + all three buttons must fit, else the buttons wrap onto the rows below) and the two-pass paint (buttons-only snapshot over the full row, then Zone A/B in what is left; the same shared MENU-001 painter, no change to it). render.rs starts the body below the bar rows; MultiplexLayout caches the bar rect and the Zone C rects for the mouse arms (menu_mouse.rs) and build_snapshot carries the Zone C slice so the ring walk (MenuSnapshot::advance) reaches the buttons.
  app/dispatch_menu_mux.rs resolves MenuZoneCActivate against the same slice: New Agent / New Bot set the MUX-002 pending new-agent placement and open the create dialogs (handle_open_create_session_dialog / handle_open_new_bot_dialog); Close runs request_close on the focused pane's session (TUI-124). The mux Kanban rows New Agent / New Bot take the same path. The bar's ring seam to the board columns moves from the last Zone B cell to the last ring stop.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The mux bar shows the board and agent views' action buttons, right-aligned, with the same labels and order: [ New Bot ] and [ New Agent ], then [ Close ] while the focused pane shows a bot or an agent; on a board, files or checkpoints pane there is no session to close, so Close is not shown, as on the board view (product owner, 2026-11-08; Close only on a session pane: helper decision, board/agent view parity)
  #   2. [ New Agent ] and [ New Bot ] open the same Start New Agent? and Start New Bot? dialogs as the other views; confirming places the new session by MUX-002's rule: it fills the next agent slot as a new pane, or the last agent slot through a forward window step when every slot is filled, and that pane is focused; Cancel changes nothing. The mux bar's Kanban dropdown rows New Agent and New Bot do the same (product owner, 2026-11-08; dropdown rows: helper decision)
  #   3. [ Close ] opens TUI-124's close dialog for the focused pane's session: Close Agent? or Close Bot?, the idle or working wording, and for a bot the count of its direct sub-agents that close with it; confirming closes it through the same close path, Cancel leaves it open (product owner, 2026-11-08)
  #   4. The buttons are on the bar's key ring after the Zone B cells (Left/Right reach them, Enter activates the focused one) and a left click activates the button exactly where it is painted, as MENU-009 does on the board and agent bars
  #   5. Wrap (TUI-131): when Zone A, the gap and all three buttons do not fit on the bar row, the buttons wrap onto a second bar row, right-aligned, and the panes start below it; nothing is cut off. The row count depends only on the terminal width, so the panes never move when focus or the open sessions change: one row at 120 columns, two at 80 (helper decision)
  #   6. On a one-row bar the buttons keep their room and Zone B (pane labels and chips) shortens through the MENU-001 truncation ladder to fit what is left (helper decision)
  #   7. The buttons keep the keys they have in the other views: . New Agent and b New Bot on the board pane, and the Kanban dropdown rows; they gain no new keys. /mux help lists the three buttons and what they do (product owner, 2026-11-08)
  #
  # EXAMPLES:
  #   1. At 120x40 with /mux board, agent, one agent open and the agent pane focused: the bar's single row ends with [ New Bot ]  [ New Agent ]  [ Close ] and the panes start on row 1
  #   2. At 80x24 with the same grid: row 0 holds the menu items, pane label and chip; row 1 holds [ New Bot ]  [ New Agent ]  [ Close ] right-aligned; the panes start on row 2 and the 21-row body is shared by them
  #   3. Focus moves to the board pane: [ Close ] disappears, [ New Bot ] and [ New Agent ] stay, and the panes do not move
  #   4. With /mux board, agent, agent and one agent open, a click on [ New Agent ] and Start: agent 2 opens as a new pane and is focused; with every agent slot filled the window steps forward so the new agent is in the last slot and focused
  #   5. A click on [ New Bot ] opens Start New Bot?; Cancel leaves the panes and focus as they were
  #   6. With an idle agent focused, Enter on [ Close ] opens Close Agent? with Close and Cancel; Close closes it and its pane drops out of the grid
  #   7. With a bot that has two sub-agents focused, [ Close ] opens Close Bot? saying its 2 sub-agents close with it
  #   8. From the last Zone B chip, Right focuses [ New Bot ], then [ New Agent ], then [ Close ]; Right again wraps to the first menu item
  #   9. /mux help mentions [ New Bot ], [ New Agent ] and [ Close ]
  #
  # ========================================
  Background: User Story
    As a fspec TUI user running mux mode
    I want to start a bot or an agent and close the focused pane's bot or agent from buttons on the mux bar, as I can in the board and agent views
    So that I don't have to leave mux or remember keys to do the most common session actions

  Scenario: At 120 columns the buttons end the single bar row
    Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    When the mux is drawn
    Then the bar row ends with "[ New Bot ]  [ New Agent ]  [ Close ]"
    And the panes start on the row below the bar

  Scenario: At 80 columns the buttons wrap onto a second bar row
    Given the terminal is 80x24 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    When the mux is drawn
    Then the first bar row holds the menu items and no button
    And the second bar row ends with "[ New Bot ]  [ New Agent ]  [ Close ]"
    And the panes start on row 2 over a 21-row body

  Scenario: Close is shown only while the focused pane shows a bot or an agent
    Given the terminal is 80x24 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    When the user presses Shift+Left to focus the board pane
    Then the second bar row ends with "[ New Bot ]  [ New Agent ]" and shows no "[ Close ]"
    And the panes have not moved

  Scenario: New Agent starts an agent in the next agent pane and focuses it
    Given the terminal is 120x40 and mux is on with the layout "board, agent, agent", one agent open and the board pane focused
    When the user clicks "[ New Agent ]"
    Then the "Start New Agent?" dialog is shown
    When the user chooses Start
    Then agent 2 opens in a new pane
    And the agent 2 pane is focused

  Scenario: New Agent with every agent slot filled steps the agent window forward
    Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the board pane focused
    When the user walks the bar ring to "[ New Agent ]", presses Enter and chooses Start
    Then the agent pane shows agent 2
    And the agent pane is focused

  Scenario: The mux Kanban menu's New Agent row places the new agent like the button
    Given the terminal is 120x40 and mux is on with the layout "board, agent, agent", one agent open and the board pane focused
    When the user runs "New Agent" from the Kanban menu and chooses Start
    Then agent 2 opens in a new pane
    And the agent 2 pane is focused

  Scenario: New Bot opens the Start New Bot dialog and Cancel changes nothing
    Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    When the user clicks "[ New Bot ]"
    Then the "Start New Bot?" dialog is shown
    When the user cancels it
    Then no session is started
    And the panes and the focus are as they were

  Scenario: Close on an idle agent opens Close Agent and Close closes it
    Given the terminal is 120x40 and mux is on with the layout "board, agent", one idle agent open and the agent pane focused
    When the user walks the bar ring to "[ Close ]" and presses Enter
    Then the "Close Agent?" dialog is shown with "Close" and "Cancel"
    When the user confirms "Close"
    Then the agent is closed
    And only the board pane is left in the grid

  Scenario: Close on a bot says its sub-agents close with it
    Given the terminal is 120x40 and mux is on with the layout "agent, agent", a bot with two idle sub-agents open and the bot pane focused
    When the user clicks "[ Close ]"
    Then the "Close Bot?" dialog says "Its 2 sub-agents will be closed too."
    And nothing is closed before the user confirms

  Scenario: The bar ring reaches the buttons after the chips
    Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open, the agent pane focused and the bar focus on the last chip
    When the user presses Right three times
    Then "[ New Bot ]", "[ New Agent ]" and "[ Close ]" are focused in turn
    When the user presses Right again
    Then the "Kanban" menu item is focused

  Scenario: The mux help lists the buttons
    Given mux is on with the layout "board, agent"
    When the user submits "/mux help"
    Then the help says the bar buttons "[ New Bot ]", "[ New Agent ]" and "[ Close ]" start a bot, start an agent and close the focused pane's bot or agent
