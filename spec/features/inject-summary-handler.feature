@BUG-582
@CMPCT-009
@session
@context-management
Feature: inject_summary NAPI Handler and Agent Loop Registration
  """
  File: rust/napi/src/inject_summary_handler.rs — contains create_handler() returning InjectSummaryHandler closure. Captures Arc<Mutex<Session>> and context_window u64. Uses codelet_cli::session::system_reminders::partition_for_compaction and codelet_common::token_estimator::count_tokens.
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

  @integration
  Scenario: Handler is registered alongside SessionSearch handler in agent loop setup
    Given a BackgroundSession is being set up for an agent run
    When the agent loop registers handlers
    Then set_inject_summary_handler should be called with the session ID and a handler closure
    And it is registered alongside the GenerateCompaction and SessionSearch handlers for the same session

  @integration
  Scenario: Handler is cleaned up on session teardown
    Given a BackgroundSession has completed its agent run
    When the session teardown cleanup executes
    Then set_inject_summary_handler should be called with the session ID and None
    And the cleanup occurs alongside the compactor sub-agent and GenerateCompaction handler cleanup

  @unit
  Scenario: create_handler returns correct InjectSummaryHandler type
    Given a pending-DAG slot Arc<Mutex<Option<String>>>, a context_window of 200000 and a compaction flag
    When create_handler is called with those arguments
    Then it should return an InjectSummaryHandler (Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>)
    And the handler should be callable with a session_id and content string
