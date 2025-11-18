@done
@validation
@example-mapping
@event-storm
@cli
@critical
@BUG-089
Feature: Generic and unhelpful examples auto-generated from Event Storm domain events

  """
  Architecture notes:
  - Fix is in generate-example-mapping-from-event-storm command
  - Remove or comment out event-to-example transformation logic (lines 103-121)
  - Preserve policy-to-rule and hotspot-to-question transformations
  - Examples list must remain empty after transformation
  - This allows humans to add concrete, contextual examples manually
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System must NOT auto-generate examples from domain events during Event Storm to Example Mapping transformation
  #   2. Example Mapping examples list must remain empty after transformation, allowing humans to add concrete examples
  #   3. Transformation should preserve rules and questions from policies and hotspots, but skip event-to-example conversion
  #
  # EXAMPLES:
  #   1. Event Storm with domain event 'TrackPlayed' generates generic example 'User track played and is logged in' (unhelpful)
  #   2. After transformation, examples list should be empty (0 examples added) even when domain events exist
  #   3. Policies still generate rules and hotspots still generate questions (only event-to-example conversion is removed)
  #
  # ========================================

  Background: User Story
    As a developer transforming Event Storm to Example Mapping
    I want to avoid auto-generated generic examples from domain events
    So that Example Mapping provides value through concrete, specific examples

  Scenario: Transform Event Storm without generating examples from domain events
    Given a work unit with Event Storm containing domain event "TrackPlayed"
    When I transform Event Storm to Example Mapping
    Then 0 examples should be added
    And the examples list should remain empty

  Scenario: Preserve policy and hotspot transformations while skipping event transformation
    Given a work unit with Event Storm containing:
      | type     | data                                      |
      | policy   | when: "UserAuthenticated" then: "LoadDashboard" |
      | hotspot  | concern: "What happens if session expires?"      |
      | event    | text: "TrackPlayed"                              |
    When I transform Event Storm to Example Mapping
    Then 1 rule should be added from the policy
    And 1 question should be added from the hotspot
    And 0 examples should be added from the event

  Scenario: Verify no generic examples are generated
    Given a work unit with Event Storm containing domain event "PlaylistSaved"
    When I transform Event Storm to Example Mapping
    Then the transformation result should show "Examples added: 0"
    And the work unit should have an empty examples array
