@done
@rpc
@ui-enhancement
@input
@agent-view
@ui
@rust
@tui
@RPC-020
Feature: Slash command palette + @file search popup in AgentView

  """
  Popups are owned directly by AgentView as `Option<SlashCommandPopup>` and `Option<FileSearchPopup>` fields (presentation state). Critical-priority dialogs still live on the Compositor and overlay these popups. New FspecBackend method `search_files(prefix, limit)` is additive; both transports delegate to a new `codelet_core::file_search::search` helper using `ignore::WalkBuilder` + `globset::GlobBuilder` (case-insensitive). Three new Action variants: `SlashCommandSelected(SlashCommandAction)`, `SearchFiles(String)`, `FileSearchResults(Vec<String>)`. File-size discipline: every new file under `views/agent/` stays under 300 LoC — split via submodules if needed. Filter sync is presentation-only: `AgentView::sync_popups()` runs after each input event and decides open/close/refilter based on the joined buffer; no Action dispatch is involved in filter changes. The `/help`, `/clear`, `/quit` handlers are wired live in this card; every other command emits a `[notice]` scrollback line referring the user to the future RPC card.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Slash command registry mirrors the TS SLASH_COMMANDS list (model, provider, debug, clear, compact, thinking, resume, detach, search, blocklist, role, merge-worktree, schedule, loop) plus a few from the dossier (help, quit, isolation, providers) — filter_commands uses three-tier matching (prefix → name substring → description substring), matching the TS filterCommands() helper.
  #   2. The slash popup intercepts ↑/↓ (navigate, with wrap-around), Enter (select+execute), Tab (fill into input WITHOUT execute), Esc (dismiss). Other keys propagate so the user can keep typing into the input to refine the filter.
  #   3. The file search popup intercepts ↑/↓ (navigate, with wrap-around), Enter (select+insert path + trailing space + dismiss), Tab (insert path without trailing space + dismiss), Esc (dismiss). Other keys propagate.
  #   4. Selecting a file via Enter replaces the substring `@<filter>` in the input with `@<path> ` (literal selected path plus a single trailing space); Tab inserts `@<path>` without the trailing space. The anchor_offset captured at OpenFileSearch is used to locate the `@` to splice over.
  #   5. FspecBackend gains async fn search_files(prefix, limit) -> Result<Vec<String>>; the shared service delegates to a NEW helper `codelet_core::file_search::search(cwd, prefix, limit)` that wraps `ignore::WalkBuilder` + `globset::GlobBuilder` (case-insensitive, **/*<prefix>* pattern) and returns at most `limit` paths sorted by modification time desc, mirroring the existing codelet_tools::glob::GlobTool behaviour.
  #   6. Implemented slash handlers in this card: /help → pushes HelpDialog onto compositor; /clear → reset AgentView scrollback + next_seq; /quit → emit Action::Quit. ALL other commands (/resume, /search, /model, /thinking, /role, /isolation, /blocklist, /compact, /debug, /providers, /detach, /merge-worktree, /schedule, /loop, /provider) emit a [notice] line into the scrollback: 'Not yet implemented (see RPC-NNN)' or 'Not yet implemented in Rust TUI' — they do NOT crash, they DO consume the dispatch.
  #   7. File search RPC method is additive — both EmbeddedFspecBackend and WebSocketFspecBackend implement it, and a cross-transport-parity test scenario asserts both backends return identical results for the same prefix against the same shared service cwd.
  #   8. File-size discipline (RPC-002 rule [10]) is preserved: every new file under codelet/fspec-tui/src/views/agent/ and any new file added by this card stays under 300 LoC; if the agent module risks crossing 300 LoC, split into submodules instead of mutating the orchestrator.
  #   9. Decision: while a slash/file popup is open AND a key matches its registered chord (↑↓/Enter/Tab/Esc), it consumes. For OTHER keys including 'q' and Ctrl+C, the popup returns Ignored so the chord drops through to AgentView's normal dispatch (which is the desired behaviour — user can keep typing characters into the filter). Only Esc/Enter/Tab/↑/↓ are consumed; all other keys propagate to MultiLineInput.handle_key so the input itself keeps recording filter characters AND ESC always closes the popup (not BackToBoard).
  #   10. Owned by AgentView as Option<SlashCommandPopup> and Option<FileSearchPopup> fields. Rationale: (a) tight coupling with MultiLineInput state for filter sync, (b) no need to add a Compositor accessor for borrowing layers by id, (c) matches the existing `pub scrollback: ScrollbackList` direct-ownership pattern. AgentView paints the popup on top of its own area (centered tui-popup) AFTER painting its base widgets. Critical-priority dialogs (HelpDialog/DisconnectDialog) still live on the Compositor at Priority::Critical and overlay everything including these popups, because Compositor renders AFTER AgentView in App::render's order.
  #   11. App::dispatch routes Action::SlashCommandSelected(action) by case: Help → push HelpDialog onto compositor; Clear → reset navigator.agent's scrollback + next_seq + clear input buffer; Quit → set should_quit=true; All other variants → navigator.agent.push_line(format!('[notice] /{} not yet implemented (see RPC-NNN or future card)', name)). File search RPC is triggered by Action::SearchFiles(prefix) — App::dispatch spawns a tokio task calling backend.search_files(prefix, 20) and dispatches Action::FileSearchResults(Vec<String>) on success.
  #   12. Three new Action variants land in components/mod.rs: SlashCommandSelected(SlashCommandAction), SearchFiles(String), FileSearchResults(Vec<String>). The SlashCommandAction enum is re-exported from views::agent::slash_commands so the Action enum can reference it. ClearScrollback semantics are folded into SlashCommandSelected(Clear) — no separate variant.
  #   13. ScrollbackList gains a `reset()` method that drops all chunks AND resets the scroll_state to default (offset=0, stick_to_bottom=true). Called from App::dispatch on SlashCommandSelected(Clear). AgentView gains a sibling `reset_scrollback()` helper that also resets next_seq to 0.
  #   14. After each MultiLineInput event, AgentView::sync_popups() inspects the joined buffer to decide whether to open/close/refilter the popups: a leading '/' on the first line opens slash popup (uses text after '/' as filter); a '@' followed by zero-or-more non-space chars opens file popup at the last '@' anchor (text after '@' as filter). Buffer state '/' followed by space, or '@' followed by space, closes the respective popup. Empty buffer closes both. This is presentation-only state owned by AgentView — no Action dispatch is required for filter sync.
  #
  # EXAMPLES:
  #   1. User types '/' on an empty AgentView input → SlashCommandPopup appears centred above the input, showing all SLASH_COMMANDS with /help highlighted as the first match (no filter yet).
  #   2. User types 'h' after '/' → popup filter is 'h', list filters to '/help' first (prefix match) then '/thinking' (substring match in description).
  #   3. Slash palette is open; user presses Down twice then Enter on '/quit' → AgentView emits Action::Quit and the App exits cleanly.
  #   4. Slash palette is open with /clear highlighted; user presses Enter → AgentView's scrollback is cleared (chunk_count returns 0) and the input box is empty.
  #   5. Slash palette is open; user presses Enter on '/help' → HelpDialog modal appears at Priority::Critical above everything; ESC returns to the prior empty AgentView state.
  #   6. Slash palette is open; user presses Enter on '/model' → a scrollback notice line appears reading '[notice] Not yet implemented (see RPC-022)' and the palette closes; the AgentView remains usable.
  #   7. User types '/' then Esc → palette dismisses, input still contains '/'; user can keep typing or delete it.
  #   8. User types 'hello @rea' → file search popup opens at the '@' anchor, shows 'README.md' (and others) sorted by mtime; arrow keys move selection; Enter on 'README.md' replaces '@rea' with '@README.md ' so the input reads 'hello @README.md ' with cursor at the end.
  #   9. User types '@' then types a space → file search popup auto-dismisses (the space ends the file-reference token) and the input is unchanged ('@ ').
  #   10. FspecBackend::search_files("README", 10) against a workspace where the project root is the codelet/ fixture returns at most 10 paths and includes 'README.md' as the first entry; identical lists are returned from both EmbeddedFspecBackend and WebSocketFspecBackend wrapping the same SharedFspecService.
  #   11. User types '/' on an empty AgentView input, presses Down, then Tab → input fills with '/clear' (no execute), popup closes; user can edit further or press Enter to send '/clear' as ordinary text.
  #   12. AgentView scrollback has 5 chunks of conversation; user types '/clear' then Enter from palette → scrollback is empty afterward and the input box is reset to empty.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Slash command popup is open while user is in AgentView. The popup intercepts ↑/↓/Enter/Tab/Esc; pressing 'q' or Ctrl+C while the popup is on top is NOT routed as Quit/Interrupt — it propagates to AgentView's MultiLineInput which treats them as plain typed characters (so 'q' fills into the input).
  #   A: Owned by AgentView as Option<SlashCommandPopup> and Option<FileSearchPopup> fields. Rationale: (a) tight coupling with MultiLineInput state for filter sync, (b) no need to add a Compositor accessor for borrowing layers by id, (c) matches the existing `pub scrollback: ScrollbackList` direct-ownership pattern. AgentView paints the popup on top of its own area (centered tui-popup) AFTER painting its base widgets. Critical-priority dialogs (HelpDialog/DisconnectDialog) still live on the Compositor at Priority::Critical and overlay everything including these popups, because Compositor renders AFTER AgentView in App::render's order.
  #
  #   Q: How should the popup keep its filter in sync with MultiLineInput's buffer? Two options: (a) the popup snapshots filter text on each render by reading from a shared store, OR (b) AgentView re-emits Action::SlashFilterChanged(filter) / FileSearchFilterChanged(filter) after every keystroke that touches the input.
  #   A: AgentView owns the popups, so filter sync happens via direct mutator calls inside AgentView::sync_popups() after each MultiLineInput event. No new Action variant for filter changes is required — the popup's `filter` field is presentation state co-owned with the input buffer.
  #
  #   Q: Are the slash + file popups owned by AgentView (as Option<...> fields) or pushed onto the Compositor as separate Components? Affects how filter-sync, event consumption, and rendering layering work.
  #   A: AgentView-owned (Option<SlashCommandPopup>, Option<FileSearchPopup>) — captured in rule [13].
  #
  # ========================================

  Background: User Story
    As a Rust fspec TUI developer
    I want to trigger a slash-command palette by typing '/' on a new line and a @file search popup by typing '@' inside the AgentView's MultiLineInput, with /help, /clear, /quit handlers wired live
    So that I can pick a slash command or file path from a floating overlay instead of typing the full text manually, matching the Ink TS AgentView UX

  Scenario: Typing '/' on an empty input opens the slash command palette
    Given an AgentView with an empty MultiLineInput and no popup visible
    When the user types "/"
    Then AgentView.slash_popup is Some
    And the slash popup's filter is ""
    And the slash popup's match count equals the SLASH_COMMANDS registry length
    And the slash popup's selected index is 0

  Scenario: Typing characters after '/' filters the slash command list
    Given an AgentView whose MultiLineInput contains "/"
    When the user types "he"
    Then the slash popup's filter is "he"
    And the slash popup's first match is the command named "help"

  Scenario: Pressing Enter on /quit emits Action::Quit
    Given an AgentView whose slash popup is open with "/quit" highlighted
    When the user presses Enter
    Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Quit)
    And the slash popup is closed
    And dispatching that action sets App.should_quit to true

  Scenario: Pressing Enter on /clear resets the AgentView scrollback and input
    Given an AgentView whose scrollback has 5 chunks
    And the slash popup is open with "/clear" highlighted
    When the user presses Enter
    Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Clear)
    And dispatching that action makes the AgentView's chunk_count equal 0
    And the MultiLineInput's buffer is empty
    And the slash popup is closed

  Scenario: Pressing Enter on /help pushes the HelpDialog onto the Compositor at Priority::Critical
    Given an App with an AgentView whose slash popup is open with "/help" highlighted
    When the user presses Enter
    Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Help)
    And dispatching that action pushes a HelpDialog Component with id "help-dialog" onto the Compositor
    And the topmost compositor layer reports priority Priority::Critical
    And the slash popup is closed

  Scenario: Pressing Enter on an unimplemented command emits a scrollback notice
    Given an AgentView whose slash popup is open with "/model" highlighted
    When the user presses Enter
    Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Model)
    And dispatching that action appends one scrollback chunk whose text contains "[notice]"
    And that scrollback chunk's text contains "model"
    And that scrollback chunk's text contains "not yet implemented"
    And the slash popup is closed

  Scenario: Pressing Tab on a selected slash command fills the input without executing
    Given an AgentView whose MultiLineInput contains "/c"
    And the slash popup is open with "/clear" highlighted
    When the user presses Tab
    Then the MultiLineInput's buffer is exactly "/clear"
    And the slash popup is closed
    And no Action::SlashCommandSelected variant was emitted

  Scenario: Pressing Esc on the slash popup dismisses it without clearing the input
    Given an AgentView whose MultiLineInput contains "/"
    And the slash popup is open
    When the user presses Esc
    Then the slash popup is closed
    And the MultiLineInput's buffer is still exactly "/"
    And no Action::BackToBoard was emitted

  Scenario: Typing '@' inside any input opens the file search popup at the '@' anchor
    Given an AgentView whose MultiLineInput contains "hello "
    When the user types "@rea"
    Then AgentView.file_popup is Some
    And the file popup's filter is "rea"
    And the file popup's anchor_offset equals the byte offset of '@' in the joined buffer
    And AgentView emits Action::SearchFiles("rea")

  Scenario: Pressing Enter on a file search result splices the chosen path with a trailing space
    Given an AgentView whose MultiLineInput contains "hello @rea"
    And the file popup is open with matches ["README.md", "src/reader.ts"] and selected index 0
    When the user presses Enter
    Then the MultiLineInput's buffer is exactly "hello @README.md "
    And the file popup is closed

  Scenario: Pressing Tab on a file search result splices the chosen path without a trailing space
    Given an AgentView whose MultiLineInput contains "hello @rea"
    And the file popup is open with matches ["README.md"] and selected index 0
    When the user presses Tab
    Then the MultiLineInput's buffer is exactly "hello @README.md"
    And the file popup is closed

  Scenario: Typing a space after '@' auto-dismisses the file search popup
    Given an AgentView whose MultiLineInput contains "hello @"
    And the file popup is open with filter ""
    When the user types " "
    Then the file popup is closed
    And the MultiLineInput's buffer is exactly "hello @ "

  Scenario: Pressing Esc on the file search popup dismisses it without modifying the input
    Given an AgentView whose MultiLineInput contains "hello @rea"
    And the file popup is open with matches ["README.md"]
    When the user presses Esc
    Then the file popup is closed
    And the MultiLineInput's buffer is still exactly "hello @rea"

  Scenario: While the slash popup is open, plain Enter is intercepted (not submitted to the chat)
    Given an AgentView whose slash popup is open with "/help" highlighted
    When the user presses plain Enter
    Then AgentView emits Action::SlashCommandSelected(SlashCommandAction::Help)
    And NO Action::InputSubmitted is emitted

  Scenario: While the slash popup is open, 'q' is treated as a typed character not as Quit
    Given an AgentView whose MultiLineInput contains "/"
    And the slash popup is open
    When the user types "q"
    Then the MultiLineInput's buffer is exactly "/q"
    And the slash popup is still open
    And the slash popup's filter is "q"
    And no Action::Quit was emitted
