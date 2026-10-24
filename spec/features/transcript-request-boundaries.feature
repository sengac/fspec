@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Validate explicit transcript request boundaries
  """
  The request boundary identifies an authoritative original prefix by length and
  final message identity. It is metadata on the first delta, not an extra record
  or conversation turn, so bounded admission retains the authentic delta itself.
  Missing historical anchors retain the existing strict compatibility policy;
  explicit malformed boundaries never fall back to inferred coordinates.
  """

  Scenario: Explicit boundaries order journal-only requests before later originals
    Given consecutive transcript-only requests with authentic original-prefix boundaries
    When later originals are archived with backward request timestamps
    Then journal outcomes retain causal order and boundary records remain private

  Scenario: Invalid explicit boundaries never become inferred coordinates
    Given an explicit request boundary naming an incorrect original-prefix identity
    When the canonical archive is requested
    Then export fails explicitly without altering originals or transcript evidence

  Scenario: Request boundaries cannot change after the first delta
    Given an acknowledged request with an authentic original-prefix boundary
    When a later delta attempts to replace that request boundary
    Then admission refuses the changed boundary without modifying acknowledged evidence
