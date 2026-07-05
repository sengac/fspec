@done
@RPC-056
@rpc
@agent-view
@tui
@slash-command
@rust
@multi-session
@session-management
Feature: /blocklist view + blocklist RPC surface
  """
  Phase 7.3 of the RPC-030 roadmap. Reaches TS-parity for the /blocklist
  slash command by:

  1. Adding a NEW `blocklist_list()` RPC method through the trait,
  FspecService, FspecBackend, and both transports. It returns
  Vec<BlocklistRuleInfo> where each entry carries the rule's
  provenance (source: "system" | "project") so the view can
  render the same source tag the TS BlocklistListView does.
  2. Replacing the `SlashCommandAction::Blocklist` notice fallback in
  dispatch_slash_commands.rs::handle_slash_command with a real
  `Action::OpenBlocklistView` dispatch routed through a new
  app/dispatch_blocklist.rs file, mirroring the dispatch_provider_settings
  (/provider) pattern.
  3. Adding a Navigator-owned `BlocklistView` child view that paints a
  two-pane layout (rule list + details), supports arrow-key navigation
  (plus PageUp/PageDown/Home/End per BLOCK-010),
  Enter/Space toggle, and Esc dismissal — full parity with
  BlocklistListView.tsx.
  4. Storing the session-disabled rule ids on `AgentViewStore.blocklist_disabled_by_session`
  so re-opening the view preserves disabled state across the
  session's lifetime — TS parity with `useState<Set<string>>` lifted
  to the AgentView component scope.

  TS reference: `AgentView.tsx` line 2757 — `handleBlocklistMode()`
  loads via `blocklistLoad(cwd)` and pushes the BlocklistListView
  overlay. `BlocklistListView.tsx` renders rows with ●/○ glyphs +
  source/action tags + a right-pane details panel.

  Out of scope: add / edit / delete / persist CRUD on the blocklist
  config file (deferred to a follow-up card); no enforcement wiring
  into the codelet_tools blocklist middleware (the session-disabled
  set is purely a UI affordance for now, matching TS).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle MUST expose a default-impl method blocklist_list(&self) -> Vec<BlocklistRuleInfo> so existing handles compile unchanged; default returns Vec::new()
  #   2. StubSessionManagerHandle overrides blocklist_list with a deterministic in-memory snapshot and exposes a per-call counter (blocklist_list_calls()) for cross-transport parity tests
  #   3. FspecService (tarpc) declares async fn blocklist_list() -> Vec<BlocklistRuleInfo> and FspecServiceImpl routes through self.inner.session_manager()
  #   4. FspecBackend trait exposes async blocklist_list() -> Result<Vec<BlocklistRuleInfo>> with a default Ok(Vec::new()) impl; both EmbeddedFspecBackend and WebSocketFspecBackend forward to the tarpc client
  #   5. BlocklistRuleInfo wire type carries id, pattern, action ("block"|"allow"|"prompt"), reason, guidance, source ("system"|"project")
  #   6. SlashCommandAction::Blocklist dispatches Action::OpenBlocklistView; Navigator flips active_view to ViewMode::Blocklist
  #   7. BlocklistView is a Navigator-owned child view with a left list pane + right details pane; arrow keys (plus PageUp/PageDown/Home/End per BLOCK-010) navigate, Enter/Space toggle, Esc closes
  #   8. The session-disabled set lives on AgentViewStore.blocklist_disabled_by_session (HashMap<SessionId, HashSet<String>>) so it persists across view open/close
  #   9. Category column derived from regex pattern shape: contains "/", starts with "~" or "./" → file_path; else bash
  #   10. Empty-state UX: when blocklist_list returns an empty Vec, the view paints "No blocklist rules configured." with config-path hints
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with one or more open AgentView sessions
    I want to open the /blocklist view to inspect every configured blocklist rule (system + project) and disable individual rules for the current session without restarting
    So that I can see exactly which rules will fire against my next tool call and temporarily relax a rule that's blocking legitimate work — with full TS-Ink parity in the Rust ratatui frontend

  Scenario: /blocklist dispatches OpenBlocklistView and triggers a blocklist_list fetch
    Given an App with an open session s-1 wired to a MockBackend whose blocklist_list returns two rules
    When SlashCommandSelected(SlashCommandAction::Blocklist) is dispatched
    Then within 1 second backend.blocklist_list is called exactly once
    And within 1 second the Navigator's active_view equals ViewMode::Blocklist
    And within 1 second Action::BlocklistRulesLoaded carrying the two rules is observed on the action bus

  Scenario: BlocklistView renders two configured rules with source tags
    Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "git-checkout-block"
    And the rendered text contains "cat-block"
    And the rendered text contains "system"
    And the rendered text contains "project"

  Scenario: Empty blocklist renders the placeholder text
    Given a BlocklistView seeded with an empty rule list
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "No blocklist rules configured"

  Scenario: Down advances the focused row
    Given a BlocklistView seeded with three rules with selected_index 0
    When the user presses Down
    Then selected_index equals 1
    When the user presses Down
    Then selected_index equals 2

  Scenario: Up retreats the focused row, clamped at 0
    Given a BlocklistView seeded with three rules with selected_index 1
    When the user presses Up
    Then selected_index equals 0
    When the user presses Up
    Then selected_index equals 0

  Scenario: Space toggles the focused rule into the session-disabled set
    Given a BlocklistView seeded with rule "git-checkout-block" focused (selected_index 0)
    And the session-disabled set is empty for the focused session
    When the user presses Space
    Then the focused session's blocklist_disabled set contains "git-checkout-block"
    When the user presses Space again
    Then the focused session's blocklist_disabled set no longer contains "git-checkout-block"

  Scenario: Enter behaves identically to Space for toggling
    Given a BlocklistView seeded with rule "cat-block" focused
    And the session-disabled set is empty for the focused session
    When the user presses Enter
    Then the focused session's blocklist_disabled set contains "cat-block"

  Scenario: A disabled rule paints the dimmed glyph and (disabled) suffix
    Given a BlocklistView seeded with rules [git-checkout-block(system), cat-block(project)]
    And the focused session's blocklist_disabled set contains "git-checkout-block"
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "○ git-checkout-block"
    And the rendered text contains "(disabled)"
    And the rendered text contains "● cat-block"

  Scenario: Esc closes the view and returns to the Agent view
    Given an App with an open session s-1 and the Navigator active_view set to ViewMode::Blocklist
    When the user presses Esc
    Then Action::CloseBlocklistView is observed on the action bus
    And the Navigator's active_view returns to ViewMode::Agent

  Scenario: Session-disabled set persists across close/reopen of the view
    Given an App with an open session s-1 and the BlocklistView open
    And the user toggles "git-checkout-block" to disabled then presses Esc
    When SlashCommandSelected(SlashCommandAction::Blocklist) is dispatched again
    Then the new BlocklistView reads the existing blocklist_disabled set from AgentViewStore
    And the row "git-checkout-block" renders with the dimmed glyph

  Scenario: Category column derives "file_path" for path-shaped patterns
    Given a BlocklistView with a rule whose pattern is "/etc/passwd"
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "file_path"

  Scenario: Category column derives "file_path" for tilde-prefixed patterns
    Given a BlocklistView with a rule whose pattern is "~/.aws/.*"
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "file_path"

  Scenario: Category column derives "bash" for command-shaped patterns
    Given a BlocklistView with a rule whose pattern is "^cat\\s+"
    When the view is rendered into a 120x24 buffer
    Then the rendered text contains "bash"
    And the rendered text does NOT contain "file_path"

  Scenario: derive_category returns deterministic strings
    Given the derive_category helper function
    Then derive_category("^cat\\s+") returns "bash"
    And derive_category("/etc/passwd") returns "file_path"
    And derive_category("~/.aws/.*") returns "file_path"
    And derive_category("./scripts/deploy.sh") returns "file_path"
