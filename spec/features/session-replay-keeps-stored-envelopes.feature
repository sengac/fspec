@BUG-582
@done
@BUG-327
@BUG-332
@session-resume
@persistence
Feature: Session replay returns the stored message envelopes
  """
  get_session_message_envelopes (core/src/persistence/session_replay.rs envelopes_for_manifest) rebuilt every envelope from scratch with parentUuid and requestId null, the row's created_at as timestamp and a message holding only role and content, then merged the stored metadata with or_insert, so the persisted parentUuid, timestamp, requestId and the message's id, model, stop_reason and usage never reached replay. Replay now starts from the stored envelope: uuid is the row id, parentUuid/timestamp/type/provider/requestId are taken from the stored envelope and only supplied from the row when it has none (rows written without an envelope, such as the compaction summary), and the stored message object is kept with only its role and its blob-rehydrated content set by replay. Native-metadata stripping, the compaction-summary overrides and blob rehydration are unchanged. BUG-332: the stored envelope is required — a conversation row whose metadata lacks parentUuid, timestamp, type, provider, message or requestId fails replay naming the row, instead of replay inventing null parents, the row time, the role or provider "unknown". Rows not written by the agent loop's typed commit carry a real envelope: append_message/append_message_with_metadata (fixture writers) store a plain-text envelope with the session's provider under the caller's metadata, and the synthesized compaction summary row carries a user envelope from the "compaction" provider at the compaction time (message_envelope::text_envelope_metadata).
  """

  Background: User Story
    As a client replaying a persisted session
    I want each replayed envelope to carry the identity and response metadata it was saved with
    So that parent chains, request ids and per-message usage survive a resume

  Scenario: Replayed envelopes keep the stored envelope and message metadata
    Given a user message and an assistant reply persisted through the production write path with known uuid, parentUuid, timestamp and requestId
    And the assistant reply carries message id "msg_327", model "claude-sonnet-4", stop reason "end_turn" and usage
    When the session's message envelopes are read for replay
    Then each envelope has its stored uuid, parentUuid, timestamp and requestId
    And the assistant envelope's message has id "msg_327", model "claude-sonnet-4", stop reason "end_turn" and the stored usage
    And each envelope's message content is the stored content with blobs rehydrated

  Scenario: A stored row without a message envelope is refused by replay
    Given a conversation row stored without a message envelope
    When the session's message envelopes are read for replay
    Then replay fails naming the row and the missing envelope

  Scenario: Rows written by the append helpers replay with a full envelope
    Given a codex session with a message written by append_message
    When the session's message envelopes are read for replay
    Then its envelope names the session's provider with a null parent and request id
    And its message is the stored text

  Scenario: The compaction summary row carries a real envelope
    Given a session compacted with a summary
    When its messages are loaded
    Then the summary row's metadata is a user message envelope from the compaction provider
