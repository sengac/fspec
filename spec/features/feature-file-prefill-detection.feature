@COV-030
@COV-029
@COV-028
@COV-027
@done
@workflow
@system-reminder
@example-mapping
@cli
Feature: Feature File Prefill Detection and CLI Enforcement

  Background: User Story
    As a AI agent (LLM) using fspec
    I want to detect and fix feature file prefill using CLI commands
    So that proper workflow enforcement and no direct file editing

  Scenario: System-reminder appears after create-feature with prefill
    Given I run fspec create-feature command
    When the generated feature file contains placeholder text
    Then a system-reminder should appear suggesting CLI commands to fix prefill

  Scenario: System-reminder appears after generate-scenarios with prefill
    Given I run fspec generate-scenarios command
    When the generated scenarios contain placeholder steps
    Then a system-reminder should suggest using fspec add-step commands

  Scenario: User story from Example Mapping generates complete Background
    Given a work unit has complete user story fields in Example Mapping
    When I run fspec generate-scenarios
    Then the Background section should contain the complete user story without placeholders

  Scenario: Workflow blocking prevents status change with prefill
    Given a linked feature file contains prefill placeholders
    When I try to update work unit status to testing
    Then the command should fail with an error listing the prefill placeholders

  Scenario: Hybrid step extraction attempts to parse example text
    Given examples have action-oriented text like 'User logs in with credentials'
    When I run fspec generate-scenarios
    Then steps should be intelligently extracted with minimal placeholders
