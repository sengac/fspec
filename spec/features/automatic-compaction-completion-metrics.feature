@BUG-210
@phase-1
@agent-core
@compaction
@context-management
Feature: Automatic compaction completion reports authoritative reduction and coverage
  """
  Architecture notes:
  Exercise the automatic DAG builder against real temporary persistence stores,
  through the background output adapter to the structured chunks consumed by
  the Rust TUI. Completion uses the committed generation's post-compaction
  context and durable covered-prefix and retained-suffix counts. Context
  publication and completion must not produce two success notices. Preserve
  compatibility for existing completion emitters that have no durable coverage.
  """

  Background: User Story
    As an agent operator
    I want automatic compaction completion to describe the committed context accurately
    So that the reported reduction and history coverage reflect actual work

  Scenario: Completion reports the percentage removed rather than retained
    Given automatic compaction begins with 166750 input tokens
    And its durable committed context contains 90336 input tokens
    When the automatic builder publishes completion through the background output adapter
    Then the TUI-compatible completion reports 166750 original tokens and 90336 compacted tokens
    And the reduction rounds to 45.8 percent rather than 54.2 percent

  Scenario: Completion coverage matches the durable generation
    Given an automatic builder commits a DAG covering 196 persisted messages
    And the committed generation retains no verified suffix messages
    When completion reaches the originating session through the background output adapter
    Then completion reports 196 summarized messages and zero retained messages
    And the coverage counts equal the committed covered prefix and verified suffix counts

  Scenario: Successful publication produces only one completion notice
    Given an automatic builder has a valid DAG candidate for a durable session
    When the candidate commits and its context is published successfully
    Then the background adapter publishes the committed context before completion
    And exactly one user-visible completion is published for that generation
    And reconciling the same generation does not publish another completion notice

  Scenario: Failed publication produces no successful completion
    Given an automatic builder has committed a valid DAG candidate
    When publication of its committed context fails
    Then no successful completion is published
    And the committed generation remains pending reconciliation
    And retrying the blocked operation does not fabricate a successful completion

  Scenario: Self-compaction completion waits for durable reconciliation
    Given an active session submits a valid summary of its own persisted history
    When its submitted summary is applied and published
    Then every completion observer sees a durably reconciled generation
    And exactly one successful completion describes that generation
