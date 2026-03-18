@done
@CMPCT-010
Feature: SessionSearch Trimming Integration
  """
  Modify codelet/napi/src/session_search_handler.rs: extend create_handler() to accept Arc<AtomicBool>, add conditional trimming in handle_show() and handle_search() after resolve_message_content() calls
  Modify codelet/napi/src/session_manager.rs: add compaction_in_progress: Arc<AtomicBool> field to BackgroundSession (alongside compaction_progress), pass to create_handler() at line 5365
  Trimmer (codelet_core::compaction::Trimmer) is stateful — must create a new instance per handle_show/handle_search call and process messages in order for tool_use_id correlation. Uses trim_message(&mut self, role, content, metadata).
  StoredMessage has metadata: HashMap<String, Value> field which is passed directly to Trimmer::trim_message(). The role field is a String (user/assistant).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BackgroundSession must have a compaction_in_progress: Arc<AtomicBool> field, defaulting to false
  #   2. When compaction_in_progress is false, SessionSearch returns full untrimmed content (current behavior, regression safety)
  #   3. When compaction_in_progress is true, SessionSearch handle_show() applies Trimmer::trim_message() to resolved content before building SessionMessage results
  #   4. When compaction_in_progress is true, SessionSearch handle_search() applies Trimmer::trim_message() to resolved content before building SearchMatch results
  #   5. Trimming is applied AFTER blob resolution via resolve_message_content() — never on raw blob references
  #   6. create_handler() signature must accept Arc<AtomicBool> as second parameter for compaction_trimming flag
  #   7. The handler closure captures both project_path and compaction_trimming Arc<AtomicBool>
  #   8. AtomicBool uses Ordering::Relaxed — single writer (compaction trigger), single reader (SessionSearch handler)
  #   9. Trimmer must process messages in order within handle_show/handle_search to maintain tool_use_id correlation
  #   10. Registration in session_manager.rs must pass session.compaction_in_progress.clone() to create_handler()
  #
  # EXAMPLES:
  #   1. Flag is false. Agent calls SessionSearch show. Session has a 500-line Read tool result. Result includes full 500 lines in content — no trimming applied.
  #   2. Flag is true. Agent calls SessionSearch show. Session has a 500-line Read tool result. Result shows '[file: src/main.rs, 500 lines, 12500 tok — use Read to retrieve]' instead of full content.
  #   3. Flag is true. Agent calls SessionSearch search. A user message 'please fix the login bug' matches the query. Content is returned unchanged (user messages never trimmed).
  #   4. Flag is true. Agent calls SessionSearch show. Session has an assistant message with reasoning text. The reasoning text passes through unchanged (only tool outputs are trimmed).
  #   5. Flag is true. Session has 3 messages: assistant with Write tool_use, user with tool_result, user with plain text. Trimmer processes them in order: registers Write tool in registry from assistant msg, trims Write result in user tool_result msg, passes plain text through.
  #   6. Flag is true. Message content is a blob reference. resolve_message_content() dereferences the blob first, then Trimmer trims the resolved content. Trimmer never sees raw blob:sha256:xxx references.
  #   7. BackgroundSession is created. compaction_in_progress field is Arc<AtomicBool> initialized to false. All SessionSearch calls return untrimmed content.
  #
  # ========================================
  Background: User Story
    As an agent performing DAG construction
    I want to receive trimmed SessionSearch results when compaction is in progress
    So that I can build a compact summary DAG without wasting context on raw tool output bloat

  @regression
  Scenario: Flag is false — SessionSearch show returns full untrimmed content
    Given the compaction_in_progress flag is false
    And the session contains a user tool result message with 500 lines of Read output
    When the agent calls SessionSearch with action "show"
    Then the result includes the full 500-line content with no trimming applied

  @core
  Scenario: Flag is true — SessionSearch show returns trimmed Read tool results
    Given the compaction_in_progress flag is true
    And the session contains a user tool result message with 500 lines of Read output for "src/main.rs"
    When the agent calls SessionSearch with action "show"
    Then the result shows a compact reference like "[file: src/main.rs, 500 lines, {tok} tok — use Read to retrieve]"

  @core
  Scenario: Flag is true — SessionSearch search preserves user messages unchanged
    Given the compaction_in_progress flag is true
    And the session contains a user message "please fix the login bug"
    When the agent calls SessionSearch with action "search" and query "login"
    Then the matched content includes "please fix the login bug" unchanged

  @core
  Scenario: Flag is true — SessionSearch show preserves assistant reasoning text
    Given the compaction_in_progress flag is true
    And the session contains an assistant message with reasoning text and no tool use
    When the agent calls SessionSearch with action "show"
    Then the assistant reasoning text is returned unchanged

  @core
  Scenario: Trimmer processes messages in order for tool_use_id correlation
    Given the compaction_in_progress flag is true
    And the session contains an assistant message with a Write tool_use block
    And the session contains a user message with the corresponding tool_result
    And the session contains a plain user message
    When the agent calls SessionSearch with action "show"
    Then the Write tool result is trimmed to a compact persistence reference
    And the plain user message passes through unchanged

  @core
  Scenario: Trimming is applied after blob resolution
    Given the compaction_in_progress flag is true
    And the session contains a message with blob-referenced content
    When the agent calls SessionSearch with action "show"
    Then the blob is resolved before trimming is applied
    And the trimmed output reflects the resolved content, not the raw blob reference

  @regression
  Scenario: BackgroundSession defaults compaction_in_progress to false
    Given a new BackgroundSession is created
    Then the compaction_in_progress field is initialized to false
    And all SessionSearch calls return untrimmed content

  @integration
  Scenario: create_handler accepts compaction_in_progress parameter
    Given a project path and an Arc<AtomicBool> compaction_trimming flag
    When create_handler is called with both parameters
    Then the returned SessionSearchHandler captures both values
    And the handler uses the flag to conditionally apply trimming

  @integration
  Scenario: Handler is registered with compaction_in_progress from BackgroundSession
    Given a BackgroundSession with a compaction_in_progress field
    When the agent loop registers the SessionSearch handler
    Then session.compaction_in_progress.clone() is passed to create_handler()
    And the handler is set via set_session_search_handler()
