@done
@querying
@rust
@cli
@foundation-management
@DISC-003
Feature: foundation discovery extended: draft-aware show-foundation, full finalize report, event-storm context guard, validate --draft

  """
  DISC-003 (part 2/5): show-foundation draft auto-preference + --final; discover-foundation --finalize lists EVERY remaining field; show-foundation-event-storm unknown-context error; validate-foundation-schema --draft.
  """

  Background: User Story
    As a AI agent
    I want to discover and fill the project foundation with clear per-step guidance and progress
    So that I always know what fields remain, what to do next, and what content is appropriate, without guessing or re-reading raw JSON

    Scenario: show-foundation with a draft present shows the draft by default
    Given a project root with both spec/foundation.json and spec/foundation.json.draft where the draft project.name='draft-name' and the final project.name='final-name'
    When I dispatch show-foundation with no section
    Then the dispatcher returns success=true
    And the returned output starts with the banner 'Showing DRAFT (foundation.json.draft)'
    And the returned output contains a progress line 'progress:' with 'fields complete'
    And the returned output reflects the draft content 'draft-name'

  Scenario: show-foundation --final forces the finalized file when a draft exists
    Given a project root with both spec/foundation.json and spec/foundation.json.draft where the draft project.name='draft-name' and the final project.name='final-name'
    When I dispatch show-foundation with final=true and section='projectName'
    Then the dispatcher returns success=true
    And the returned output is exactly 'final-name'

  Scenario: show-foundation without a draft is byte-identical to today
    Given a project root with spec/foundation.json project.name='fspec' and no draft
    When I dispatch show-foundation with no section and format='text'
    Then the dispatcher returns success=true
    And the returned output contains the exact line '=== PROJECT ==='
    And the returned output does NOT contain the banner 'Showing DRAFT'

  Scenario: finalize failure lists every remaining field with its fix command
    Given a project root whose spec/foundation.json.draft still has project.vision, problemTitle, problemDefinition, solutionOverview, capabilities, and personas unfilled
    When I dispatch discover-foundation with finalize=true
    Then the dispatcher returns valid=false
    And the validationErrors starts with 'Cannot finalize: draft still has unfilled placeholder fields'
    And the validationErrors names each of the 6 remaining fields with its exact fix command
    And the validationErrors ends with 'Then re-run: fspec discover-foundation --finalize'

  Scenario: show-foundation-event-storm with an unknown context errors and lists available contexts
    Given a project root with a finalized spec/foundation.json whose event storm has bounded contexts 'Auth' and 'Specification'
    When I dispatch show-foundation-event-storm with context='Aut'
    Then the dispatcher returns success=false
    And the error message contains 'Unknown context'
    And the error message lists 'Auth' and 'Specification' as available contexts

  Scenario: show-foundation-event-storm with a matching context is unchanged
    Given a project root with a finalized spec/foundation.json whose event storm has bounded context 'Auth' and one aggregate inside it
    When I dispatch show-foundation-event-storm with context='Auth'
    Then the dispatcher returns success=true
    And the returned data contains the bounded context item 'Auth'
    And the returned data contains the aggregate item

  Scenario: validate-foundation-schema --draft validates the draft file
    Given a project root with a spec/foundation.json.draft that is empty in solutionSpace.capabilities and no spec/foundation.json
    When I dispatch validate-foundation-schema with draft=true
    Then the dispatcher returns success=true at the envelope level
    And the result reports success=false with the error 'Field solutionSpace.capabilities must have at least 1 items (found 0)'

  Scenario: validate-foundation-schema --draft on a valid draft reports valid
    Given a project root with a schema-valid spec/foundation.json.draft and no spec/foundation.json
    When I dispatch validate-foundation-schema with draft=true
    Then the dispatcher returns success=true at the envelope level
    And the result reports success=true with an output naming the draft as valid

  Scenario: validate-foundation-schema --draft with no draft file reports a friendly error
    Given an empty project root with no spec/foundation.json.draft
    When I dispatch validate-foundation-schema with draft=true
    Then the dispatcher returns success=true at the envelope level
    And the result reports success=false with an error naming foundation.json.draft
