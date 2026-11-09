@done
@PROV-166
@PROV-172
@providers
@context-window
@rpc
Feature: Session model limits travel on the wire as null when unavailable
  """
  BUG-445: session getters return Result; an unknown session or a missing
  session manager is an error (session-getters-report-lookup-failures).

  PROV-165 made unavailable model limits None inside the daemon and left one transitional adapter
  (sessions model_resolution::wire_limit) that encoded None as the wire's 0. PROV-166 removes it:
  SessionModel.context_window / max_output_tokens / compaction_threshold are Option<i64> and
  ModelInfo.context_window / compaction_threshold are Option<u32> (rust/rpc-types). The sessions
  handle (get_session_model, get_model_info) publishes the cached ResolvedModelLimits as they are,
  and a session that does not exist (or no session manager at all) reports null limits.

  PROV-172: resolving a session's limits for the wire converts each one to u32 with a checked
  conversion; a context window or output limit that does not fit is an error naming the limit,
  the model and the value, never a truncated number.
  """

  Background: User Story
    As a daemon client showing a session's model
    I want to receive an unavailable context window, output limit or compaction threshold as null
    So that a missing limit is never shown or computed as 0

  Scenario: A session without limits reports null limits
    Given a session whose custom provider model declares no limits
    When its session model and model info are read through the session manager handle
    Then the session model's context window, output limit and compaction threshold are null
    And the model info's context window and compaction threshold are null

  Scenario: A catalogued session reports its limits
    Given a session whose Codex model is in the catalog with a 400000 context window and a 128000 output limit
    When its session model is read through the session manager handle
    Then the context window is 400000 and the output limit is 128000
    And the compaction threshold is present

  Scenario: Reading the model of a session that does not exist fails naming it
    Given a session manager without the requested session
    When its session model is read
    Then the read fails naming the session id

  Scenario: A context window too large for the wire fails limit resolution
    Given an OpenAI selection whose context window override is 5000000000
    When its model limits are resolved for the session
    Then resolution fails with an error naming the context window, the model and 5000000000
    And no truncated context window 705032704 is produced
