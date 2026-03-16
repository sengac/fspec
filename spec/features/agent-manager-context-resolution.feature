@done
@AMGR-011
Feature: Message context resolution

  """
  Add optional context field to AgentManagerAction::Message variant: context: Option<Vec<ContextReference>>.
  Add ContextReference enum with three variants: Turns { session_id, turns }, TurnRange { session_id, start_turn, end_turn }, Query { session_id, query }.
  Context resolution logic lives in agent_manager_handler.rs as a new resolve_context() function.
  It uses persistence::load_session() and persistence::get_session_messages_full() — the same persistence layer as SessionSearch.
  For the search query variant, reuse grep-regex matching (same engine as SessionSearch's handle_search).
  The resolved context is appended to the IncomingMessage.message field before delivery — the existing receive_incoming_message() API stays unchanged.
  Add MessageDeliveredWithContext variant to AgentManagerResult for context_resolved count.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The message action accepts an optional context array alongside the existing session_id and message parameters. Each element is a context reference object.
  #   2. Three context reference variants are supported: (1) specific turns { session_id, turns: [42,43,44] }, (2) turn range { session_id, start_turn: 10, end_turn: 15 }, (3) search query { session_id, query: 'SQL injection' }. All three can be mixed in a single context array.
  #   3. Context references are resolved at send time in the handler — the persistence layer (load_session, get_session_messages_full) is accessed to fetch actual message content. Resolved content is appended after the sender's message text before delivery.
  #   4. Resolved context is formatted in XML-style blocks: <quoted-context><from session="id" turns="N-M">[N] role: content\n[N+1] role: content</from></quoted-context>. Each reference becomes a separate <from> block inside the <quoted-context> wrapper.
  #   5. Graceful degradation for errors: if a referenced session doesn't exist, the <from> block contains a warning '⚠ Session {id} not found' but the message still delivers. If a search query matches nothing, the block contains '⚠ No matches for query "{term}"'. The sender's message text is always delivered regardless of resolution failures.
  #   6. The MessageDelivered response includes a context_resolved count: { delivered: true, session_id: 'target', context_resolved: 2 }. This tells the sender how many context references were successfully resolved (vs. degraded with warnings).
  #   7. When context is omitted or empty, the message action behaves identically to AMGR-010 — plain text delivery with no context resolution. This is backward compatible.
  #   8. Turn indices are 0-based, matching the position in the session's ordered message list (same indexing as SessionSearch show). Out-of-range turn indices are silently skipped — if turns [1, 999] are requested and the session only has 5 messages, only turn 1 is included.
  #   9. Blob references in resolved messages are transparently resolved — if a message's content contains a blob:sha256: reference, it's fetched from BlobStore and the actual text content is used in the quoted context.
  #
  # EXAMPLES:
  #   1. Agent sends message with context=[{session_id: 'supervisor-id', turns: [1, 2]}] — receiver gets the message text plus <quoted-context><from session="supervisor-id" turns="1-2">[1] user: ...[2] assistant: ...</from></quoted-context>
  #   2. Agent sends message with context=[{session_id: 'worker-id', start_turn: 0, end_turn: 5}] — receiver gets turns 0 through 5 from the worker's session history inline with the message
  #   3. Agent sends message with context=[{session_id: 'peer-id', query: 'SQL injection'}] — receiver gets matching turns from the peer's session containing 'SQL injection' references
  #   4. Agent sends message referencing a session that was deleted — gets { delivered: true, context_resolved: 0 } and the receiver sees a warning in the quoted-context block
  #   5. Agent sends message with context=[{session_id: 'worker-id', query: 'nonexistent phrase'}] — query matches nothing, receiver sees '⚠ No matches for query' in the quoted block, but the sender's message text is still delivered
  #   6. Agent sends message with mixed context array — [{session_id: 'A', turns: [3]}, {session_id: 'B', query: 'auth'}] — both references are resolved and appear as separate <from> blocks inside a single <quoted-context>
  #   7. Agent sends message without context (or context=[]) — behaves exactly like AMGR-010 plain text delivery, response has no context_resolved field
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to attach session history references when sending messages to other agents
    So that the receiving agent gets self-contained context without needing follow-up SessionSearch calls

  Scenario: Message with specific turn references resolves content inline
    Given a sender session and a target session exist
    And a source session has conversation history with at least 3 turns
    When the sender sends a message with context referencing specific turns from the source session
    Then the message is delivered successfully
    And the delivered message contains the sender's text
    And the delivered message contains a quoted-context block with the referenced turns
    And each turn shows its index, role, and content
    And the response includes context_resolved count of 1

  Scenario: Message with turn range reference resolves consecutive turns
    Given a sender session and a target session exist
    And a source session has conversation history with at least 6 turns
    When the sender sends a message with context referencing a turn range from the source session
    Then the message is delivered successfully
    And the delivered message contains turns from the start through end of the range
    And the response includes context_resolved count of 1

  Scenario: Message with search query reference resolves matching turns
    Given a sender session and a target session exist
    And a source session has conversation history containing specific keywords
    When the sender sends a message with context referencing a search query against the source session
    Then the message is delivered successfully
    And the delivered message contains only the turns that matched the search query
    And the response includes context_resolved count of 1

  Scenario: Context reference to non-existent session degrades gracefully
    Given a sender session and a target session exist
    When the sender sends a message with context referencing a session that does not exist
    Then the message is still delivered successfully
    And the delivered message contains the sender's text
    And the quoted-context block contains a session not found warning
    And the response includes context_resolved count of 0

  Scenario: Search query with zero matches degrades gracefully
    Given a sender session and a target session exist
    And a source session exists with conversation history
    When the sender sends a message with context referencing a query that matches nothing in the source session
    Then the message is still delivered successfully
    And the delivered message contains the sender's text
    And the quoted-context block contains a no matches warning
    And the response includes context_resolved count of 0

  Scenario: Mixed context array resolves multiple references
    Given a sender session and a target session exist
    And two source sessions exist with different conversation histories
    When the sender sends a message with a context array containing both turn references and a search query from different sessions
    Then the message is delivered successfully
    And the delivered message contains separate from blocks for each context reference
    And the response includes context_resolved count matching the number of successful resolutions

  Scenario: Message without context behaves as plain text delivery
    Given a sender session and a target session exist
    When the sender sends a message without a context parameter
    Then the message is delivered as plain text
    And the response matches the AMGR-010 MessageDelivered shape with no context_resolved field

  Scenario: Out-of-range turn indices are silently skipped
    Given a sender session and a target session exist
    And a source session has conversation history with exactly 3 turns
    When the sender sends a message with context referencing turns including indices beyond the session length
    Then the message is delivered successfully
    And only the valid turn indices are included in the quoted-context block
    And invalid indices are silently omitted

  Scenario: Context reference type is dispatched through ContextReference enum
    Given the AgentManagerAction Message variant includes an optional context field
    When context references are deserialized from JSON
    Then specific turn references deserialize to the Turns variant
    And turn range references deserialize to the TurnRange variant
    And search query references deserialize to the Query variant
