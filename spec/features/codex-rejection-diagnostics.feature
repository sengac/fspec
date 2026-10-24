@done
@providers
@CMPCT-076
@provider-abstraction
@codex
@compaction
@bug
@phase-1
Feature: Codex rejection diagnostics
  Codex failures must retain bounded evidence without guessing that every rejection needs compaction.

  """
  CMPCT-076 preserves typed adapter diagnostics through the real HTTP and SSE client.
  Only the declared context_length_exceeded code identifies context rejection.
  Unknown, incomplete and malformed evidence remains unknown; status or message text
  never substitutes for a declared code. Evidence collection is byte- and deadline-bounded.
  Credential values and non-allowlisted headers are not exposed by diagnostics.
  Opaque header bytes, proxy authorization and explicitly sensitive request headers
  receive the same protection as ordinary credentials. Allowlisted response metadata
  must not reintroduce JSON-escaped credential echoes.
  Transport whitespace around header values or authentication schemes must not
  prevent redaction of the actual credential echoed by the provider.
  Rendered errors escape untrusted body controls while retaining sanitized evidence.
  This includes individual quoted and unquoted cookie values and JSON-escaped
  credential echoes, even in malformed or incomplete evidence.
  Evidence limits must not disclose credential prefixes when they split Unicode
  characters or escape sequences, including a secret starting at an incomplete escape.
  Existing bounded same-transport setup retries remain unchanged. No alternate request,
  transport or memory algorithm is introduced. Broad caller classifiers belong to CMPCT-082.
  """

  Scenario: HTTP context rejection retains typed evidence through the model stream
    Given Codex HTTP rejection responses with declared context codes
    When the real model consumes each rejected stream
    Then status code message and request identity survive in a typed error without success or replay

  Scenario: Unrelated and malformed rejection evidence never implies context overflow
    Given unrelated malformed and oversized Codex rejection bodies
    When the real streaming client receives each rejection
    Then diagnostics remain bounded and only complete declared codes classify the failure
    And rendered errors cannot execute terminal control sequences

  Scenario: Exhausted setup retries preserve final rejection evidence
    Given retryable Codex rejection responses with retry advice
    When the declared setup retry budget is exhausted
    Then the final typed error retains status retry advice identity and attempt count

  Scenario: SSE failures retain typed bounded diagnostics without completion
    Given failed and top-level Codex SSE error events
    When the real model consumes each terminal failure
    Then declared codes and bounded messages survive without completion or tool dispatch

  Scenario: Rejection diagnostics do not disclose credentials or private headers
    Given a rejection echoing request credentials and private response headers
    When the real streaming client captures diagnostic evidence
    Then credential values are redacted and private headers are absent

  Scenario: A stalled rejection body terminates within its diagnostic deadline
    Given an HTTP rejection whose body never finishes
    When the real model consumes the rejected stream
    Then typed incomplete evidence returns before the harness deadline without replay

  Scenario: Sensitive header credentials stay private throughout rejection evidence
    Given credentials in opaque proxy and explicitly sensitive request headers
    When a rejection echoes those credentials in its body and allowlisted metadata
    Then neither typed fields nor formatted diagnostics disclose the credential echoes

  Scenario: A broken rejection body preserves typed unknown evidence
    Given a context rejection body followed by a transport read failure
    When the real model consumes the broken rejection
    Then status identity and bounded evidence survive but incomplete evidence cannot classify overflow

  Scenario: Direct completion rejections retain bounded private typed evidence
    Given direct Codex HTTP rejections with context unrelated malformed and oversized evidence
    When the real model requests a nonstreaming completion
    Then typed bounded diagnostics preserve status identity and declared classification without credential disclosure
    And the rejected request is not replayed

  Scenario: Direct completion rejection capture obeys its deadline
    Given a direct completion rejection whose body never finishes
    When the real model requests a nonstreaming completion
    Then typed incomplete evidence returns before the harness deadline without replay
