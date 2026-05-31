@done
@RPC-022
@rust
@tui
@agent-view
@header
Feature: RoleBanner — inline one-row banner above scrollback when a session role is active
  """
  RoleBanner lives at codelet/fspec-tui/src/views/agent/role_banner.rs
  and is the Rust port of src/tui/components/RoleBanner.tsx (TUI-081).

  Unlike ModelSelectorDialog / ThinkingLevelDialog, RoleBanner is NOT a
  Compositor layer — it is an inline Widget owned by AgentView's render
  path. AgentView's `render_with_store` reads
  `store.role_for(current_session)` per frame; when `Some(text)` it
  carves a 1-row strip above the scrollback Block and paints the
  banner, when `None` the layout collapses (zero rows allocated).

  Multi-line role text is collapsed to a single line by replacing any
  whitespace run (including newlines) with a single space, then
  truncated to the terminal width — mirroring the RoleBanner.tsx
  `singleLineRole = roleText.replace(/\\s+/g, ' ').trim()` behaviour.

  RoleBanner is SUPPRESSED entirely when AgentView.resume_view or
  AgentView.search_view is open — those mode views paint into the full
  area Rect (RPC-026) so a banner row would be invisible / wasteful.
  """

  Background: User Story
    As a Rust fspec TUI user
    I want a visible "Role: <text>" banner above the conversation when a custom role is active
    So that I can tell at a glance which agent persona is bound to the session

  @visibility
  @hidden
  Scenario: RoleBanner renders zero rows when no role is set on the focused session
    Given an AgentViewStore with one open session "s-1" and role_for("s-1") = None
    When AgentView.render_with_store paints into a 80x24 area
    Then no row in the rendered buffer starts with "Role:"
    And the scrollback Block consumes the entire flex region between header and input

  @visibility
  @visible
  Scenario: RoleBanner renders one row when a role is set on the focused session
    Given an AgentViewStore with one open session "s-1" and role_for("s-1") = Some("You are a security reviewer")
    When AgentView.render_with_store paints into a 80x24 area
    Then exactly one row in the rendered buffer starts with "Role:"
    And the substring "You are a security reviewer" appears on that row
    And the scrollback Block height shrinks by exactly 1 row compared to the no-role layout

  @multi-line
  @collapse
  Scenario: Multi-line role text is collapsed to a single line
    Given an AgentViewStore with role_for("s-1") = Some("You are a security reviewer.\nAnalyze code for vulnerabilities.")
    When AgentView.render_with_store paints into a 100x24 area
    Then the rendered "Role:" row contains "You are a security reviewer. Analyze code for vulnerabilities."
    And the rendered "Role:" row contains NO newline characters

  @truncation
  Scenario: Long role text is truncated to terminal width
    Given an AgentViewStore with role_for("s-1") = Some("X".repeat(500))
    When AgentView.render_with_store paints into a 40x24 area
    Then exactly one row contains the "Role:" prefix
    And that row occupies exactly 40 columns and does not wrap to a second row

  @multi-session
  @per-session-state
  Scenario: RoleBanner reflects the focused session only, not background sessions
    Given an AgentViewStore with two open sessions "s-1" and "s-2"
    And role_for("s-1") = Some("Reviewer A") and role_for("s-2") = None
    And current_session_index = 0
    When AgentView.render_with_store paints
    Then the "Role:" row reads "Role: Reviewer A"
    When current_session_index is set to 1
    And AgentView.render_with_store paints
    Then no "Role:" row appears in the rendered buffer

  @mode-view-suppression
  Scenario: RoleBanner is suppressed while resume_view is active
    Given an AgentViewStore with role_for("s-1") = Some("Reviewer A")
    And AgentView.resume_view is Some(default ResumeSessionView)
    When AgentView.render_with_store paints into a 80x24 area
    Then no row in the rendered buffer starts with "Role:"

  @mode-view-suppression
  Scenario: RoleBanner is suppressed while search_view is active
    Given an AgentViewStore with role_for("s-1") = Some("Reviewer A")
    And AgentView.search_view is Some(default SearchHistoryView)
    When AgentView.render_with_store paints into a 80x24 area
    Then no row in the rendered buffer starts with "Role:"

  @line-budget
  @source-shape
  Scenario: role_banner.rs stays under 300 lines
    Given the file codelet/fspec-tui/src/views/agent/role_banner.rs after RPC-022 lands
    When a test counts the line-count of the file
    Then the file has fewer than 300 lines
