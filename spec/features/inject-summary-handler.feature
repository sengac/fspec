@CMPCT-009
Feature: inject_summary NAPI Handler and Agent Loop Registration

  """
  File: codelet/napi/src/inject_summary_handler.rs — contains create_handler() returning InjectSummaryHandler closure. Captures Arc<Mutex<Session>> and context_window u64. Uses codelet_cli::session::system_reminders::partition_for_compaction and codelet_common::token_estimator::count_tokens.
  Registration follows the exact pattern of SessionSearch handler registration at session_manager.rs:5363-5368. Cleanup follows pattern at session_manager.rs:5575-5578. Handler needs session.inner.clone() and provider_manager.context_window() captured at registration time.
  The handler is synchronous (Fn, not async) but needs to lock the async tokio::sync::Mutex<Session>. Must use tokio::task::block_in_place(|| { runtime_handle.block_on(async { session_inner.lock().await }) }) — same pattern as bridge_handler at session_manager.rs:5533-5536. The runtime_handle is captured at registration time via tokio::runtime::Handle::current().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Handler must partition session.messages using partition_for_compaction() — separates system reminders from all other messages
  #   2. Handler must clear session.messages entirely after partitioning
  #   3. Handler must restore only system reminder messages from partition step
  #   4. DAG content must be appended as Message::User wrapped in <system-reminder><!-- type:compaction-dag --> markers
  #   5. session.turns must be reset to empty Vec after injection
  #   6. session.token_tracker must be recalculated from actual post-injection messages using count_tokens()
  #   7. Handler must return InjectSummaryResult with accurate injected_tokens and remaining_budget values
  #   8. Handler must be registered in session_manager.rs agent loop setup alongside SessionSearch handler
  #   9. Handler must be cleaned up (set to None) on session teardown alongside SessionSearch handler cleanup
  #   10. Builder turns (SessionSearch calls during DAG construction) must NOT be deleted from on-disk persistence — only dropped from in-memory session.messages
  #   11. Handler closure must capture BackgroundSession's inner Arc<Mutex<Session>> — requires tokio runtime to block_on the async mutex lock
  #   12. create_handler() must return an InjectSummaryHandler (Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>)
  #
  # EXAMPLES:
  #   1. Session has 200 messages (5 system reminders + 195 conversation). Agent calls inject_summary with DAG content. After: session has 6 messages (5 system reminders + 1 DAG user message).
  #   2. DAG content '# D2: Architecture\n- JWT auth' is injected. Post-injection message content is '<system-reminder>\n<!-- type:compaction-dag -->\n# D2: Architecture\n- JWT auth\n</system-reminder>'
  #   3. Agent calls inject_summary with 1250-token DAG. Context window is 200000 tokens. Post-injection messages total 15000 tokens. Returns {injected_tokens: 1250, remaining_budget: 185000}.
  #   4. Session A and Session B both running. inject_summary called on Session A — only Session A's messages are modified. Session B is unaffected.
  #   5. After inject_summary, session.turns is empty Vec. Token tracker input_tokens reflects only system_reminders + DAG message. output_tokens is reset to 0.
  #   6. Handler registered at session_manager.rs ~line 5368, immediately after SessionSearch handler registration. Cleaned up at ~line 5576 alongside other handlers.
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to call inject_summary to pin a DAG summary as persistent context
    So that I can complete self-directed context compaction with zero LLM cost

  @unit
  Scenario: Partition, clear, restore, and inject DAG into session messages
    Given a session with 200 messages including 5 system reminders and 195 conversation messages
    When the inject_summary handler is called with DAG content
    Then session.messages should contain exactly 6 messages
    And the first 5 messages should be the original system reminders
    And the last message should be a Message::User containing the DAG content

  @unit
  Scenario: DAG content is wrapped in system-reminder compaction-dag markers
    Given a session with system reminders and conversation messages
    When the inject_summary handler is called with DAG content "# D2: Architecture\n- JWT auth"
    Then the injected message content should start with "<system-reminder>"
    And the injected message content should contain "<!-- type:compaction-dag -->"
    And the injected message content should end with "</system-reminder>"
    And the DAG text should be preserved verbatim between the markers

  @unit
  Scenario: Returns accurate injected_tokens and remaining_budget
    Given a session with a context window of 200000 tokens
    And the session has system reminders and conversation messages
    When the inject_summary handler is called with a 1250-token DAG
    Then the result should contain injected_tokens equal to the DAG token count
    And remaining_budget should equal context_window minus total post-injection message tokens

  @unit
  Scenario: Concurrent sessions have isolated handler execution
    Given Session A and Session B each have an inject_summary handler registered
    When inject_summary is called on Session A with DAG content
    Then only Session A's messages are partitioned, cleared, and reconstructed
    And Session B's messages remain completely unmodified

  @unit
  Scenario: Turns and token tracker are reset after injection
    Given a session with non-empty turns and accumulated token counts
    When the inject_summary handler is called with DAG content
    Then session.turns should be an empty Vec
    And session.token_tracker.input_tokens should reflect only the post-injection messages
    And session.token_tracker.output_tokens should be reset to 0

  @integration
  Scenario: Handler is registered alongside SessionSearch handler in agent loop setup
    Given a BackgroundSession is being set up for an agent run
    When the agent loop registers handlers
    Then set_inject_summary_handler should be called with the session ID and a handler closure
    And the registration should occur immediately after SessionSearch handler registration

  @integration
  Scenario: Handler is cleaned up on session teardown
    Given a BackgroundSession has completed its agent run
    When the session teardown cleanup executes
    Then set_inject_summary_handler should be called with the session ID and None
    And the cleanup should occur alongside SessionSearch and Fspec handler cleanup

  @unit
  Scenario: create_handler returns correct InjectSummaryHandler type
    Given a Session wrapped in Arc<Mutex<Session>> and a context_window of 200000
    When create_handler is called with the session and context_window
    Then it should return an InjectSummaryHandler (Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>)
    And the handler should be callable with a session_id and content string
