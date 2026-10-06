@done
@ui-enhancement
@menu-bar
@tui-component
@keyboard-navigation
@MENU-008
Feature: Board menu bar — reorganize Zone A into Kanban / Tools / Settings categories
  """
  MenuAction gains one variant: `Providers` -> Action::OpenProviderSettingsView (RPC-054, already dispatched by App::try_dispatch_provider_settings and flipped to ViewMode::ProviderSettings by Navigator::apply_action). Registry reshape in items.rs: KANBAN = [., /, D, A], TOOLS = [F, C], SETTINGS = [M, P], HELP unchanged; CATEGORIES = [kanban, tools, settings, help]. The legacy ACTIONS const is replaced by KANBAN (rename), keeping the 300-LoC ceiling. The 'P' bare-key arm lands in views/board/keys.rs (modifier-free, Ctrl+P falls through) — it flows into the focused Board pane in mux mode via the existing keyboard isolation routing, so no mux change is needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: The 'Kanban' category lists the work-unit workflow entries in order: '. New Agent' (OpenAgentView), '/ Search' (OpenWorkUnitSearch), 'D FOUNDATION.md' (OpenFoundation), 'A Attachments' (OpenAttachmentPicker).
  #   2. R1: The MenuCategories registry (components/menu_bar/items.rs) defines four categories in fixed order: 'Kanban', 'Tools', 'Settings', 'Help'. The 'Actions' label is renamed to 'Kanban' (id 'kanban'). The registry stays the single source of truth for the bar, dropdowns, 'u' help dialog and bare-key docs.
  #   3. R4: The 'Settings' category lists the configuration entries in order: 'M Mux' (OpenMuxConfigDialog), 'P Providers' (OpenProviderSettingsView). The 'P' bare-key arm is NEW — added to views/board/keys.rs modifier-free-only (Ctrl+P falls through), consistent with the a/c/f/d/m guards; it emits OpenProviderSettingsView from the board and from the focused Board pane in mux mode (Board-pane key routing parity).
  #   4. R3: The 'Tools' category lists the git tools in order: 'F Changed Files' (OpenChangedFilesView), 'C Checkpoints' (OpenCheckpointsView).
  #   5. R6: Every consuming surface updates to the four categories: the continuous ring walks Kanban → Tools → Settings → Help → Zone B cells (MENU-002 R8 ring math is index-based, so it picks up the new length automatically); the 'u' MenuBarHelpDialog lists all 10 registry rows (Kanban 4 + Tools 2 + Settings 2 + Help 2) in registry order; the R6 truncation ladder keeps painting the bar within area width (the absolute minimum is now 'Kanban Tools Settings Help'); the agent view (MENU-007) still shows its single 'Board View' item only.
  #   6. R5: The 'Help' category is UNCHANGED: '? Help' (OpenBoardHelp) and 'Esc Exit' (OpenBoardExitConfirmation). All existing bare-key bindings keep working exactly as before (., /, c, f, m, d, a, u); no key is re-assigned or removed — only 'P' is added (R4).
  #
  # EXAMPLES:
  #   1. The bar row reads (after 1-cell pad): 'Kanban Tools Settings Help │ #1 ●' — 4 Zone A items each separated by a 1-cell gap, then the dim separator and chips
  #   2. Focusing item 0 highlights 'Kanban'; pressing Enter opens its dropdown with 4 rows: '. New Agent', '/ Search', 'D FOUNDATION.md', 'A Attachments'
  #   3. Opening the 'Tools' dropdown shows 2 rows: 'F Changed Files' and 'C Checkpoints'; executing row 0 emits OpenChangedFilesView (the dual-pane view opens, same as the bare 'F' key)
  #   4. Opening the 'Settings' dropdown shows 2 rows: 'M Mux' (OpenMuxConfigDialog) and 'P Providers' (OpenProviderSettingsView); executing the 'P Providers' row opens the ProviderSettings view from the board
  #   5. Walking the ring right from the 'Help' item lands on the first Zone B cell (chip), exactly as before — the ring now covers Kanban → Tools → Settings → Help
  #   6. Pressing 'P' on the board (or in the focused Board pane of mux mode) opens the ProviderSettings view, just like the existing 'M' key opens the Mux config dialog
  #   7. The 'u' menu-bar help dialog now lists 10 rows across the four categories: the 4 Kanban rows, then the 2 Tools rows, then the 2 Settings rows, then the 2 Help rows
  #
  # ========================================
  Background: User Story
    As a board user
    I want to see the menu bar organized as Kanban, Tools, Settings and Help
    So that each top-level item groups related commands, so I can find kanban actions, git tools and configuration without scanning one flat list

  @menu-registry
  Scenario: The registry defines the four categories in Kanban, Tools, Settings, Help order
    Given the MenuCategories registry
    When the Zone A category list is read
    Then it has 4 categories in order: "Kanban", "Tools", "Settings", "Help"
    And the first category's id is "kanban"

  @menu-registry
  Scenario: The Kanban category lists the work-unit workflow entries in order
    Given the MenuCategories registry
    When the Kanban category is read
    Then it has 4 entries in order: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    And each entry's action matches the bare board key (OpenAgentView, OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker)

  @menu-registry
  Scenario: The Tools category lists the git tools in order
    Given the MenuCategories registry
    When the Tools category is read
    Then it has 2 entries in order: "F Changed Files" and "C Checkpoints"
    And they emit OpenChangedFilesView and OpenCheckpointsView

  @menu-registry
  Scenario: The Settings category lists the configuration entries in order
    Given the MenuCategories registry
    When the Settings category is read
    Then it has 2 entries in order: "M Mux" and "P Providers"
    And they emit OpenMuxConfigDialog and OpenProviderSettingsView

  @menu-registry
  Scenario: The Help category is unchanged
    Given the MenuCategories registry
    When the Help category is read
    Then it has 2 entries in order: "? Help" and "Esc Exit"
    And they emit OpenBoardHelp and OpenBoardExitConfirmation

  @menu-registry
  Scenario: Every registry entry has a key hint, label, description and a valid action
    Given the MenuCategories registry
    When every entry of every category is inspected
    Then none of its key hints, labels or descriptions are empty
    And every action resolves to an existing Action variant

  @menu-bar-paint
  Scenario: The bar row paints the four Zone A items
    Given a MenuSnapshot with 1 open session (Idle)
    When the bar is rendered into a 120-column row
    Then Zone A reads "Kanban Tools Settings Help"
    And the row still carries the dim separator and the session chips

  @menu-dropdown
  Scenario: Focusing the Kanban item opens a 4-row dropdown
    Given the board's menu bar with focus on the 0th menu item
    When Enter is pressed
    Then the Kanban dropdown is open with 4 rows: ". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments"
    And the Tools and Settings items are not highlighted

  @menu-dropdown
  Scenario: Executing the Tools row opens the changed-files view
    Given the Tools dropdown open with the cursor on row 0
    When Enter is pressed
    Then the Changed Files dual-pane view opens (OpenChangedFilesView)
    And the dropdown closes and the bar highlight clears (BUG-196 R2: execute de-selects the bar)

  @menu-dropdown
  Scenario: Executing the Settings Providers row opens the provider settings view
    Given the Settings dropdown open with the cursor on row 1
    When Enter is pressed
    Then the ProviderSettings view opens (OpenProviderSettingsView)
    And the dropdown closes and the bar highlight clears (BUG-196 R2: execute de-selects the bar)

  @menu-ring
  Scenario: The ring walks across all four items before reaching Zone B
    Given a MenuSnapshot with 1 open session and MenuFocus on the "Help" item
    When the ring is advanced right
    Then the focus lands on the first Zone B cell (the session chip)
    And advancing right from the first item walks "Tools", "Settings", "Help" in order

  @bare-key
  Scenario: The P key opens the provider settings view from the board
    Given the board view is active
    When the modifier-free "P" key is pressed
    Then OpenProviderSettingsView is emitted
    And Ctrl+P falls through to App-level handling

  @menu-help
  Scenario: The u help dialog lists all ten registry rows in registry order
    Given the "u" menu-bar help dialog is open
    When its body rows are rendered
    Then it shows 10 rows across the four categories: the 4 Kanban rows, then the 2 Tools rows, then the 2 Settings rows, then the 2 Help rows
    And the first row reads ". New Agent" and the last row reads "Esc Exit"

  @menu-geometry
  Scenario: Tight width still degrades safely with the four items
    Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    When the bar is rendered into a 20-column area
    Then the row still fits within the area width
    And it shows at least the "Kanban" and "Help" items
