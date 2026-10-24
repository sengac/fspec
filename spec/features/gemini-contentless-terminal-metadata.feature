@BUG-224
@done
@providers
@context-management
@streaming
@bug-fix
@phase-1
Feature: Gemini contentless terminal metadata
  """
  Architecture: the builtin Gemini SSE parser handles terminal metadata independently
  of optional candidate content. Physical stream tests exercise the public adapter.
  Preserve first-candidate selection and strict measured terminal usage validation.
  Candidate-less usage-only frames and agent-loop completion policy are out of scope.
  """

  Scenario: Contentless terminal candidates preserve finish reasons and measured usage
    Given Gemini streams emit text followed by terminal candidates with absent null or empty content
    When the builtin adapter consumes each supported terminal finish reason
    Then each stream retains the text and normalized finish reason with exactly one measured terminal response

  Scenario: Contentless terminal candidates accept explicitly measured zero
    Given Gemini terminal candidates have absent or null content and explicitly measured zero usage
    When the builtin adapter consumes the streams
    Then each stream completes once with measured zero and the normalized finish reason

  Scenario: Contentless terminal candidates reject missing or inconsistent usage
    Given Gemini streams have valid earlier usage but terminal candidates with absent null or empty content and invalid usage
    When the builtin adapter consumes missing and inconsistent terminal measurements
    Then each stream reports the appropriate usage error without a successful terminal response
