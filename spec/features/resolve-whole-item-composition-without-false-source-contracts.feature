@REFAC-019
@tui
@integration
@code-quality
@phase-1
Feature: Resolve whole item composition without false source contracts
  """
  Resolve actual literal includes and the actual accumulator chain. Fail closed
  for unsupported semantic attributes on includes, macro definitions and entry
  invocations. Preserve attributes on ordinary noncomposition items. Reject
  cycles, disconnected definitions, lossy or duplicated forwarding and ambiguous
  definitions. Never let dead source satisfy a positive or negative shape check.
  This preserves the existing malformed-chain scenario and its test-file lineage.
  """

  Background: User Story
    As a maintainer verifying decomposed source contracts
    I want source inspection to follow only valid actual composition
    So that disabled or disconnected declarations cannot falsely prove compatibility

  Scenario: Source composition rejects ambiguous or broken macro chains
    Given isolated whole-item composition fixtures with valid and invalid chains
    When includes and accumulator stages are resolved
    Then valid members retain order and malformed disconnected cyclic or lossy chains fail
