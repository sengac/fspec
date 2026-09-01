@done
@querying
@rust
@cli
@foundation-management
@DISC-003
Feature: workflow guidance doc documents correct foundation argument names

  """
  DISC-003 (part 5/5): the injected workflow guidance doc (Phase 0 FOUNDATION section) is rewritten with correct arg shapes, foundation-status in the flow, and generate-tags-md replacing the phantom derive-tags-from-foundation.
  """

  Background: User Story
    As a AI agent
    I want to discover and fill the project foundation with clear per-step guidance and progress
    So that I always know what fields remain, what to do next, and what content is appropriate, without guessing or re-reading raw JSON

    Scenario: the workflow guidance doc documents correct foundation argument names
    Given the injected workflow guidance constant
    When I inspect the Phase-0 FOUNDATION section
    Then it documents update-foundation with section and content keys
    And it documents add-capability with a name key and NOT a capability key
    And it documents add-persona with name, description, and goals keys
    And it documents add-foundation-bounded-context with a text key
    And it mentions foundation-status and generate-tags-md
    And it does NOT contain 'derive-tags-from-foundation'
    And it does NOT document update-foundation with key and value argument names
