@TUI-096
@tui
@resume
@session-display
Feature: Resume view rich session display

  """
  Architecture notes:
  - Add updated_at_ms: Option<i64> to SessionInfo in rust/rpc-types/src/lib.rs
  - Populate updated_at_ms from SessionManifest.updated_at in list_sessions (session_manager.rs)
  - Populate updated_at_ms from Utc::now() in BackgroundSession::get_info() (background_session.rs)
  - Rewrite render_session_rows to display 2-line format: name line + detail line
  - Implement format_time_ago helper for human-readable timestamp display
  - Adjust scroll math in ResumeSessionView for 2-line rows (visible_sessions = visible_rows / 2)
  - Match TypeScript AgentView.tsx resume mode rendering (lines 5162-5200)
  """

  Background: User Story
    As a developer using the Rust TUI
    I want to see rich session information in the /resume view
    So that I can quickly identify which session to resume by name, recency, and message count instead of opaque UUIDs

  @rendering
  @selected
  Scenario: Selected session renders name and detail lines with rich information
    Given the resume view has a session with name "Project Alpha", 12 messages, provider "openai/gpt-4", and updated 2 hours ago
    And the selected index is 0
    When the view renders the session rows
    Then the first visual row shows "▸ Project Alpha" with REVERSED background
    And the second visual row shows "    12 messages | openai/gpt-4 | 2h ago" with REVERSED background

  @rendering
  @unselected
  Scenario: Unselected session renders name and detail lines without selection marker
    Given the resume view has a session with name "Project Beta", 5 messages, provider "anthropic/claude-3", and updated 1 day ago
    And the selected index is 0
    And there is a second session at index 1
    When the view renders the session rows
    Then the third visual row shows "   Project Beta" with plain style
    And the fourth visual row shows "    5 messages | anthropic/claude-3 | 1d ago" with plain style

  @rendering
  @no-provider
  Scenario: Session without provider displays unknown in detail line
    Given the resume view has a session with name "Test Session", 3 messages, no provider, and updated 30 minutes ago
    And the selected index is 0
    When the view renders the session rows
    Then the detail line shows "    3 messages | unknown | 30m ago"

  @rendering
  @no-timestamp
  Scenario: Session without timestamp displays unknown in detail line
    Given the resume view has a session with name "Old Session", 1 message, provider "openai/gpt-4", and no timestamp
    And the selected index is 0
    When the view renders the session rows
    Then the detail line shows "    1 messages | openai/gpt-4 | unknown"

  @rendering
  @empty
  Scenario: Empty session list renders centered placeholder
    Given the resume view is open
    When the session list is empty
    Then the body shows the centered placeholder "(no sessions to resume)"

  @scrolling
  @two-line
  Scenario: Scroll offset accounts for 2 visual rows per session
    Given the resume view has 10 sessions
    And the body area height is 10 rows
    When the user presses Down to select session at index 3
    Then the scroll offset adjusts so the 2-line rows for session 3 are visible
    And the visible session count is approximately half the body height

  @time-ago
  Scenario: Time ago formatting handles various intervals
    Given a session updated 30 seconds ago
    When the time ago string is computed
    Then it displays "just now"
    Given a session updated 45 minutes ago
    When the time ago string is computed
    Then it displays "45m ago"
    Given a session updated 5 hours ago
    When the time ago string is computed
    Then it displays "5h ago"
    Given a session updated 3 days ago
    When the time ago string is computed
    Then it displays "3d ago"
    Given a session updated 2 weeks ago
    When the time ago string is computed
    Then it displays "2w ago"
    Given a session updated 3 months ago
    When the time ago string is computed
    Then it displays "3mo ago"
