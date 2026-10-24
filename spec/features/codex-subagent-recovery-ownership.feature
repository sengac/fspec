@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex subagent recovery ownership
  """
  Codex owns network recovery in every session, including DeepSearch and compaction subagents. Their streaming collectors must propagate terminal Codex errors without generic string-based retries, including wrapped typed exhaustion and permanent errors containing network-like text. Ordinary providers retain their bounded generic recovery. Existing subagent hard wall-clock budgets remain authoritative.
  """

  Background: User Story
    As a Codex session owner
    I want subagents to respect the provider's recovery decision
    So that terminal failures remain visible without duplicate generic recovery

  Scenario: Subagent collectors preserve Codex terminal failures and ordinary recovery
    Given Codex subagents with wrapped typed exhaustion or permanent errors containing network-like text
    When DeepSearch or compaction collects the streaming result
    Then the original terminal failure is returned without polling further or generic backoff
    And ordinary provider recovery retains its retry budget and subagent hard deadlines remain enforced
