@done
@querying
@rust
@cli
@foundation-management
@DISC-003
Feature: foundation-status read-only progress report command

  """
  DISC-003 (part 1/5): the new `foundation-status` command — a Rust-only, read-only progress report for the foundation discovery workflow. Reports the phase (none | draft | final), an 8-row per-field progress table with previews, the remaining fields with fix commands and examples, and the finalize next-action. Args {json?: bool}. Performs no writes.
  """

  Background: User Story
    As a AI agent
    I want to discover and fill the project foundation with clear per-step guidance and progress
    So that I always know what fields remain, what to do next, and what content is appropriate, without guessing or re-reading raw JSON

    Scenario: foundation-status reports missing phase when no foundation exists
    Given an empty project root with no spec/foundation.json and no spec/foundation.json.draft
    When I dispatch foundation-status
    Then the dispatcher returns success=true
    And the returned status reports phase 'none'
    And the returned status tells me to run 'fspec discover-foundation' to start

  Scenario: foundation-status on a fresh draft lists every remaining field with a fix command and an example
    Given a project root whose spec/foundation.json.draft is the canonical 8-field placeholder draft
    When I dispatch foundation-status
    Then the dispatcher returns success=true
    And the returned status reports phase 'draft' with 'Progress: 0/8 fields complete'
    And the status shows all 8 fields as incomplete with a per-field preview
    And the 'Remaining' section lists all 8 fields, each with its fix command
    And the 'Remaining' section includes an example for the problemDefinition field
    And the status ends with 'When complete: fspec discover-foundation --finalize'

  Scenario: foundation-status on a partially filled draft reports correct per-field status
    Given a project root whose spec/foundation.json.draft has project.name, project.vision, and project.projectType filled and the other 5 fields still placeholder
    When I dispatch foundation-status
    Then the dispatcher returns success=true
    And the returned status reports 'Progress: 3/8 fields complete'
    And the 3 filled fields are marked complete with their current values as previews
    And the 5 unfilled fields are marked incomplete

  Scenario: foundation-status on a finalized foundation reports FINAL phase with no remaining fields
    Given a project root with a fully filled spec/foundation.json and no draft
    When I dispatch foundation-status
    Then the dispatcher returns success=true
    And the returned status reports phase 'final'
    And the returned status reports 'Progress: 8/8 fields complete'
    And the returned status has no remaining fields

  Scenario: foundation-status in json mode returns the machine-readable envelope
    Given a project root whose spec/foundation.json.draft has project.name filled and the rest placeholder
    When I dispatch foundation-status with json=true
    Then the dispatcher returns success=true
    And the returned data parses as JSON with keys phase, progress, fields, remaining, and nextAction
    And the fields array has 8 entries each carrying path, alias, status, and preview
    And the remaining array lists exactly the 7 incomplete fields
