@done
@BUG-280
@BUG-281
@tui
@session-management
@integration
@rust
@phase-1
Feature: Story creation completion preserves captured intent
  """
  The shared creation dialog captures optional story intent. Confirmation
  consumes it into the async request; cancellation discards it. Completion
  explicitly targets the returned session ID once, independent of observation
  ordering or currently focused panes. Revisioned focus remains authoritative.
  """

  Background: User Story
    As a developer using concurrent agents
    I want story creation to preserve my explicit intent
    So that unrelated agents and later creations are never reassigned

  Scenario: Story creation retains its target across observation and focus races
    Given a story-specific creation dialog and an unrelated open agent
    When normal or isolated creation is observed before acknowledgement and focus changes
    Then only the returned new session is assigned the requested story once
    And unrelated focus and mux layout remain unchanged

  Scenario: Cancelling story creation cannot leak intent
    Given a story-specific creation dialog over an unrelated agent
    When the user cancels and subsequently creates a generic agent
    Then the new agent has no story and the original agent keeps its story

  Scenario: Declined or failed story creation cannot leak intent
    Given a story-specific creation dialog over an unrelated agent
    When creation is declined or fails and a generic creation is subsequently confirmed
    Then the failure is visible and the later agent has no story
    And the original agent keeps its story

  Scenario: Duplicate story completion cannot overwrite newer authoritative focus
    Given a completed story creation whose session later clears its focus
    When duplicate story completion and older focus acknowledgements arrive
    Then the newer cleared focus remains authoritative without another attachment
