@done
@querying
@rust
@cli
@foundation-management
@DISC-003
Feature: foundation mutation commands append universal next-step trailers

  """
  DISC-003 (part 3/5): every foundation-domain mutation (update-foundation draft path, add/remove-capability, add/remove-persona, and the event-storm add/remove commands) appends a compact nextSteps trailer (progress/eventStorm summary + next action) to its success result. No trailer on hard-error paths.
  """

  Background: User Story
    As a AI agent
    I want to discover and fill the project foundation with clear per-step guidance and progress
    So that I always know what fields remain, what to do next, and what content is appropriate, without guessing or re-reading raw JSON

    Scenario: add-capability on the draft appends a progress trailer
    Given a project root whose spec/foundation.json.draft has 5 of 8 fields filled including a non-empty capabilities array
    When I dispatch add-capability with name='Spec Validation' and description='Validate Gherkin features'
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'progress:' reporting fields complete and 'remaining:'
    And the result nextSteps contains a line starting 'next:' with the next field fix command

  Scenario: add-persona on the draft appends a progress trailer
    Given a project root whose spec/foundation.json.draft has 6 of 8 fields filled and real capabilities
    When I dispatch add-persona with name='Developer' and description='Builds features' and goals=['Ship quality code']
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'progress:' with 'remaining:'
    And the result nextSteps contains a line starting 'next:'

  Scenario: remove-capability on the draft appends a progress trailer
    Given a project root whose spec/foundation.json.draft has capabilities containing 'Spec Validation' and 5 of 8 fields filled
    When I dispatch remove-capability with name='Spec Validation'
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'progress:'

  Scenario: update-foundation on the draft keeps the field reminder and adds a progress trailer
    Given a project root whose spec/foundation.json.draft has only project.name filled
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the dispatcher returns success=true
    And the result systemReminder still contains 'Field 2/8: project.vision'
    And the result nextSteps contains a line starting 'progress:'

  Scenario: no trailer is emitted when a mutation fails
    Given an empty project root with no foundation files
    When I dispatch add-capability with name='X' and description='Y'
    Then the dispatcher returns success=false
    And the error message contains 'foundation.json not found'

  Scenario: add-foundation-bounded-context appends an event-storm trailer
    Given a project root with a finalized spec/foundation.json
    When I dispatch add-foundation-bounded-context with text='Auth'
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'eventStorm:' with context, aggregate, event, and command counts
    And the result nextSteps contains a line starting 'next:' suggesting an aggregate for 'Auth'

  Scenario: add-aggregate-to-foundation appends an event-storm trailer
    Given a project root with a finalized spec/foundation.json whose event storm contains bounded context 'Auth'
    When I dispatch add-aggregate-to-foundation with contextName='Auth' and aggregateName='Session'
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'eventStorm:' reporting 1 context and 1 aggregate
    And the result nextSteps contains a line starting 'next:' suggesting a domain event for 'Auth'

  Scenario: remove-foundation-bounded-context appends an event-storm trailer
    Given a project root with a finalized spec/foundation.json whose event storm contains bounded contexts 'Auth' and 'Billing'
    When I dispatch remove-foundation-bounded-context with contextName='Billing'
    Then the dispatcher returns success=true
    And the result nextSteps contains a line starting 'eventStorm:'
