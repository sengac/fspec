@done
@menu-bar
@tui-component
@keyboard-navigation
@ui-enhancement
@MENU-001
Feature: MenuBar component — 2-zone bar (GUI menu items + anchored dropdown) with view/chip switcher, MenuCategories registry, pure painter
  """
  New module components/menu_bar/ (files: mod.rs = MenuSnapshot/MenuFocus/re-exports/entry fns; items.rs = MenuCategory/MenuEntry/MenuAction registry + CATEGORIES const; chips.rs = chip snapshot builder; dropdown.rs = dropdown geometry + row builder; paint.rs = 1-row bar painter + truncation ladder). All under the 300-LoC ceiling. views/board/board_shortcuts.rs becomes a re-export shim (TUI-106 spinner-shim precedent) so the BOARD-023 dialog + key arms keep compiling unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The MenuBar renders exactly one row. Zone A (menu items) is painted left-to-start, a dim '|' separator, then Zone B (view/chip switcher). The row paints a #333333 background on every cell and 1 cell of horizontal padding on each side (mirrors SessionHeader/SessionFooter).
  #   2. R2: The MenuCategories registry (components/menu_bar/items.rs) is the single source of truth. It defines four categories in this order (MENU-008 reorganization): 'Kanban' (entries: '.' New Agent, '/' Search, 'D' FOUNDATION.md, 'A' Attachments), 'Tools' (entries: 'F' Changed Files, 'C' Checkpoints), 'Settings' (entries: 'M' Mux, 'P' Providers), and 'Help' (entries: '?' Help, 'Esc' Exit). Each entry carries key hint, label, one-line description, and the existing Action emitted on execute (OpenAgentView(Option<SessionId>), OpenWorkUnitSearch, OpenFoundation, OpenAttachmentPicker, OpenChangedFilesView, OpenCheckpointsView, OpenMuxConfigDialog, OpenProviderSettingsView, OpenBoardHelp, OpenBoardExitConfirmation). The board's board_shortcuts.rs becomes a re-export shim over this registry.
  #   3. R3: The MenuSnapshot (owned, built per frame) carries: the active ViewMode, the MenuFocus (None | Item(i) | Chip(j)), the open dropdown Option<(category, cursor)>, the chip list (per open session: index (n,total), SessionStatus, optional WU id, is_focused_session), and clock_ms for the spinner frame. The painter takes (area, &MenuSnapshot, &Theme) and is stateless.
  #   4. R4: Session chips render as '#n' + status glyph (+ optional WU id suffix when width allows). '#n' paints only when n >= 1 (SessionHeader parity). Status glyphs: Running = animated braille from components::spinner::current_frame_glyph(clock_ms) in magenta; Compacting = U+21BB (cyan); Paused = '!' (yellow); Idle = U+25CF (green, dimmed); Interrupted = U+2715 (red). Cleared sessions get NO chip (the snapshot builder omits them).
  #   5. R5: A focused bar item (MenuFocus::Item or MenuFocus::Chip) renders inverse-video (bg Cyan, fg Black) over the item's cells. MenuFocus::None paints no highlight. The active-view item in Zone B (when Zone B renders view labels in mux mode) is additionally marked with a bold label (the non-mux chips-only variant has no view labels).
  #   6. R6: When Zone B has no chips AND is in chips-only (non-mux) mode, the separator '|' and Zone B are omitted entirely (the row is just the menu items). When a chip list exists but width is tight, the truncation ladder runs in order: (1) drop WU id suffixes, (2) fold chips beyond the 3rd into '+n', (3) drop non-active view labels (mux), (4) absolute minimum 'Kanban Tools Settings Help' (MENU-008). Every step must keep the painted row within area.width (a proptest pins this).
  #   7. R7: The MenuDropdown renders a rounded-border panel anchored directly under the owning menu item: x = item_x, y = bar_y + 1, width = max(24, widest_row + 4), height = entries + 2 (1 border row each side, NO footer, NO title row — GUI menu parity). Rows paint a 4-cell dim key-hint column, then the label (fg), then a dim description truncated with an ellipsis. The cursor row is inverse-video (bg Cyan/fg Black, bold). The panel left-clamps so it never paints past the terminal's right edge, and bottom-clips with a dim ellipsis row when the terminal is too short.
  #   8. R8: The dropdown row builder reuses the MenuCategories registry (R2) so the dropdown, the 'u' help dialog (MENU-005), and the bare-key arms can never drift. The 'New Agent' entry carries Action::OpenAgentView(None) as a placeholder; the caller (dialog/dropdown) substitutes the session target snapshot at open time (BOARD-023 R5 parity).
  #   9. Omit Cleared sessions entirely (no chip) — they are terminal-state sessions that no longer participate in the workflow; the chip list is built from open_sessions() which never includes cleared ones.
  #   10. Yes — include 'A' Attachments as a Kanban entry (OpenAttachmentPicker). The bare-key arm already exists and the Kanban menu should list every board workflow shortcut; the old SHORTCUTS table simply predated it.
  #
  # EXAMPLES:
  #   1. At 120 columns with 3 open sessions (Idle, Running, Idle) and no focus, the row reads (after 1-cell pad): 'Kanban Tools Settings Help │ #1 ●  #2 ⠋  #3 ●' with #2's glyph in magenta and a #333333 bg on every cell
  #   2. A developer building a board surface sees the bar paint menu items 'Kanban', 'Tools', 'Settings' and 'Help' in Zone A, then a dim '|' separator, then the three session chips in Zone B — the Running session's chip shows an animated braille glyph while Idle sessions show a solid dot
  #   3. A developer focuses the 'Kanban' menu item (MenuFocus::Item(0)) and the snapshot marks it: the 'Kanban' label paints inverse-video (cyan background, black foreground) while 'Tools', 'Settings', 'Help' and the chips stay normal
  #   4. A surface renders the 'Kanban' dropdown open (cursor on row 1 of 4): a rounded-border panel appears one row below the 'Kanban' item, listing the 4 entries with key hints ('.', '/', 'D', 'A'), row 2 ('/ Search') highlighted inverse-video
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Cleared sessions render a dimmed chip or be omitted from the bar entirely?
  #   A: Omit Cleared sessions entirely (no chip) — they are terminal-state sessions that no longer participate in the workflow; the chip list is built from open_sessions() which never includes cleared ones.
  #
  #   Q: Should the 'A' Attachments shortcut (key arm exists in views/board/keys.rs but was never in the SHORTCUTS table) be included in the Kanban category?
  #   A: Yes — include 'A' Attachments as a Kanban entry (OpenAttachmentPicker). The bare-key arm already exists and the Kanban menu should list every board workflow shortcut; the old SHORTCUTS table simply predated it.
  #
  # ========================================
  Background: User Story
    As a TUI developer
    I want to render a single-line menu bar + anchored dropdown
    So that the board, agent, mux surfaces share one menu component instead of duplicating the layout

  @menu-registry
  Scenario: The Kanban category lists the four board workflow shortcuts in order
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
  Scenario: The Help category lists the help and exit entries
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

  @menu-chips
  Scenario: Three sessions paint chips with per-status glyphs
    Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
    When the bar is rendered into a 120-column row
    Then Zone B reads "#1 ●  #2 ⠋  #3 ●"
    And the Running chip's glyph cell is styled magenta

  @menu-chips
  Scenario: Each session status maps to its own glyph and color
    Given a MenuSnapshot with sessions in statuses Compacting, Paused and Interrupted
    When the bar is rendered
    Then the chips render "↻", "!" and "✕"
    And the glyphs are styled cyan, yellow and red respectively

  @menu-chips
  Scenario: A single open session shows the #1 prefix
    Given a MenuSnapshot with exactly 1 open session (Idle)
    When the bar is rendered
    Then Zone B reads "#1 ●" (the prefix paints because n >= 1, SessionHeader parity)

  @menu-chips
  Scenario: The Running chip's braille glyph advances with the clock
    Given a MenuSnapshot with 1 open session (Running)
    When the bar is rendered at clock_ms 0 and again at clock_ms 80
    Then the chip's glyph is the first braille frame at 0ms and the second frame at 80ms

  @menu-chips
  Scenario: Cleared sessions produce no chip
    Given a MenuSnapshot whose open session list contains a session in the Cleared status
    When the chip list is built
    Then no chip is produced for the Cleared session

  @menu-chips
  Scenario: A chip shows its work-unit id when width allows
    Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    And a 200-column render area
    When the bar is rendered
    Then the chip reads "#1 MENU-001 ●"

  @menu-focus
  Scenario: The focused menu item paints inverse-video
    Given a MenuSnapshot with MenuFocus on the 0th menu item
    When the bar is rendered
    Then the "Kanban" item's cells are styled bg Cyan fg Black
    And the "Help" item's cells are not inverted

  @menu-focus
  Scenario: The focused chip paints inverse-video
    Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
    When the bar is rendered
    Then the second chip's cells are styled bg Cyan fg Black
    And the first chip's cells are not inverted

  @menu-focus
  Scenario: No focus paints no highlight
    Given a MenuSnapshot with MenuFocus None
    When the bar is rendered
    Then no cell in the row is styled bg Cyan

  @menu-geometry
  Scenario: The row paints 1-cell padding and a dark background on every cell
    Given a 40-column render area
    When the bar is rendered
    Then cell 0 and the last cell are background-only
    And every cell of the row has the #333333 background

  @menu-geometry
  Scenario: An empty chip list omits the separator and Zone B
    Given a MenuSnapshot with no open sessions
    When the bar is rendered
    Then the row contains the menu items but no "|" separator and no chips

  @menu-geometry
  Scenario: Tight width drops WU ids before folding chips
    Given a MenuSnapshot with 2 open sessions each bound to a long work-unit id
    When the bar is rendered into a 58-column area
    Then no chip shows a work-unit id suffix
    And the chips still paint

  @menu-geometry
  Scenario: Extremely tight width degrades to the absolute minimum
    Given a MenuSnapshot with 6 open sessions
    When the bar is rendered into a 30-column area
    Then the row still fits within the area width
    And it shows at least the "Kanban" and "Help" items

  @menu-geometry
  Scenario: A zero-width or zero-height area paints nothing
    Given a render area with width 0 or height 0
    When the bar is rendered
    Then the buffer is unchanged

  @menu-dropdown
  Scenario: The dropdown panel is anchored under its menu item
    Given a MenuSnapshot with the Kanban dropdown open at cursor 1
    When the dropdown is rendered
    Then the panel's x equals the Kanban item's x and its y is one row below the bar
    And its height is 6 rows (4 entries + 2 border rows)
    And it has no title row and no footer row

  @menu-dropdown
  Scenario: Dropdown rows show a key-hint column, label and dim description
    Given the Kanban dropdown rendered open
    When row 2 is inspected
    Then it starts with the dim 4-cell key hint "/ "
    And it shows the label "Search" in the primary foreground
    And it shows a dim description truncated with an ellipsis when the row is too wide

  @menu-dropdown
  Scenario: The dropdown cursor row is inverse-video
    Given the Kanban dropdown open with the cursor on row 1
    When the dropdown is rendered
    Then row 2 (index 1) is styled bg Cyan fg Black bold across its full inner width
    And all other rows are not inverted

  @menu-dropdown
  Scenario: The dropdown left-clamps at the terminal right edge
    Given a Help item positioned near the terminal's right edge
    When its dropdown is rendered
    Then the panel is shifted left so it never paints past the terminal width

  @menu-dropdown
  Scenario: The dropdown bottom-clips with an ellipsis when the terminal is too short
    Given a short render area (6 rows below the bar)
    When the Kanban dropdown (4 entries) is rendered
    Then only 3 entry rows plus a dim "⋯" indicator row are visible (4 content rows inside the borders)
    And the panel never paints below the area
