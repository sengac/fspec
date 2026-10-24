@agent-core
@phase-1
@context-management
@compaction
@CMPCT-052
Feature: Compaction inference handoff
  """
  An accepted summary submission is queued, not an acknowledgement of a durable
  commit. The streaming tool loop hands control back to the commit owner after
  recording the actual tool call and result, before another provider request.
  A failed commit cannot permit ordinary inference or publish completion.
  """

  Scenario: Accepted summary submission stops inference at the tool boundary
    Given a streaming provider that requests summary injection
    When the registered handler accepts the summary for commitment
    Then the tool loop returns before making another provider request
    And the actual summary tool call and result remain paired
    And a later stream invocation remains blocked until durable commit reconciliation

  Scenario: Queued summary does not claim durable application
    Given a registered summary handler that accepts a candidate
    When the summary tool serializes its accepted result
    Then the result explicitly reports queued rather than committed
    And the queued result does not publish an unmeasured remaining request budget

  Scenario: Rejected summary submission blocks subsequent inference
    Given a streaming provider that requests summary injection
    When the registered handler rejects the candidate
    Then the tool loop returns a typed blocked outcome before another provider request
    And no compaction completion is emitted
    And a later stream invocation remains blocked until explicit reconciliation

  Scenario: Self-target generation yields before ordinary inference
    Given a streaming provider that requests self-target compaction generation
    When the generation handler queues its candidate
    Then the tool loop yields before another provider request
    And the generated candidate result reports queued application
    And a later stream invocation remains blocked until durable commit reconciliation

  Scenario: Rejected self-target generation blocks ordinary inference
    Given a streaming provider that requests self-target compaction generation
    When the generation handler rejects its candidate
    Then ordinary inference stops with a typed blocked outcome
    And no compaction completion is emitted
    And a later stream invocation remains blocked until explicit reconciliation
