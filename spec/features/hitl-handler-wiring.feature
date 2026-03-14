@done
Feature: HITL request_user_input handler wired via pause pattern

  """
  Rust: HITL handler closure follows pause pattern — set hitl_request state (RwLock<Option<HitlRequestState>>), set_status(Paused), wait_for_hitl_response (blocks on mpsc), on response clear state + set_status(Running). No StreamChunk emitted.
  NAPI: session_get_hitl_request(session_id) returns Option<NapiHitlRequestState> with questions array. session_send_hitl_response already exists for the response path. Remove HitlRequest StreamChunk variant and all GlobalSessionStreamManager intercept code.
  TypeScript: rustStateSource.ts adds getHitlRequest(sessionId) wrapping NAPI getter. useRustSessionState adds hitlRequest to RustSessionSnapshot, fetched when isPaused. InputTransition renders inline HITL UI (like pause renders inline). AgentView adds useInputCompat handler for HITL keyboard navigation.
  HITL rendering is multi-step inline: shows [1/N] question header, question text, selected/unselected options (up/down), Enter advances to next question, on last question Enter submits all. Freeform-only questions show text input. Esc cancels at any point.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BackgroundSession MUST have hitl_response_tx/hitl_response_rx channel pair (std::sync::mpsc) for blocking the handler until the TUI sends answers, mirroring fspec_response_tx/rx
  #   2. The HITL handler closure MUST follow the pause pattern: store hitl_request state on BackgroundSession, set_status(Paused), wait_for_hitl_response (block), then clear state and set_status(Running) on response
  #   3. BackgroundSession MUST have hitl_request: RwLock<Option<HitlRequestState>> storing the questions while waiting for user response
  #   4. A session_get_hitl_request NAPI function MUST exist for TypeScript to poll the current HITL questions when session is paused — following the same pattern as session_get_pause_state
  #   5. useRustSessionState MUST add hitlRequest field to RustSessionSnapshot, fetched via getHitlRequest(sessionId) in rustStateSource when isPaused is true
  #   6. InputTransition MUST render HITL questions inline when isPaused and hitlRequest is present — showing current question with selectable options and freeform input, same rendering location as pause UI
  #   7. AgentView MUST have a useInputCompat handler for HITL that captures up/down to navigate options, Enter to select/advance, Esc to cancel — and calls sessionSendHitlResponse with the collected answers
  #   8. Pressing Escape during HITL MUST call sessionSendHitlResponse with cancelled=true to unblock the Rust handler
  #   9. The HitlRequest StreamChunk variant, GlobalSessionStreamManager intercept, setHitlHandler/clearHitlHandler, and handleHitlRequest MUST be removed — these were the wrong pattern
  #
  # EXAMPLES:
  #   1. LLM calls request_user_input with 2 questions → handler stores questions in hitl_request state → sets status to Paused → React re-renders → InputTransition shows first question → user navigates and selects → advances → submits all → handler unblocks
  #   2. User presses Escape during HITL → sessionSendHitlResponse called with cancelled=true → handler unblocks → facade converts to ToolError
  #   3. Session paused with hitl_request → useRustSessionState polls sessionGetHitlRequest → snapshot.hitlRequest has questions → InputTransition renders inline
  #   4. Session ends → cleanup clears hitl_request state → blocked recv returns Cancelled fallback
  #   5. No HITL handler registered (headless mode) → execute_hitl returns error immediately — no pause, no blocking
  #   6. Question with options renders inline with selected/unselected indicators and navigation hints
  #   7. Question without options (freeform only) renders text input area
  #
  # ========================================

  Background: User Story
    As a user
    I want to use the request_user_input tool in a real TUI session
    So that the LLM can ask me structured questions mid-turn and receive my answers

  # === Rust: Handler closure + session state ===

  @BUG-118
  Scenario: HITL handler stores questions in session state and pauses
    Given a BackgroundSession with hitl_request state and hitl_response channel pair
    When the HITL handler closure is invoked with a request containing 2 questions
    Then the handler should store the questions in hitl_request state
    And the handler should set session status to Paused
    And the handler should block on wait_for_hitl_response
    When a response is sent via send_hitl_response
    Then the handler should clear the hitl_request state
    And the handler should set session status back to Running
    And the handler should return the response to the caller

  @BUG-117
  Scenario: HITL handler cleanup on session end
    Given a session has a registered HITL handler and hitl_request state
    When the agent loop finishes and session cleanup runs
    Then set_hitl_handler should be called with None
    And hitl_request state should be cleared
    And if the handler was blocked, recv should return Cancelled fallback

  @BUG-117
  Scenario: Headless mode returns error without blocking
    Given no HITL handler is registered for the session
    When execute_hitl is called
    Then it should return an error immediately
    And it should NOT set session status to Paused
    And it should NOT block

  @BUG-118
  Scenario: BackgroundSession has HITL request state and response channel pair
    Given a new BackgroundSession is created
    Then it should have a hitl_request field of type RwLock Option HitlRequestState
    And it should have a hitl_response_tx sender and hitl_response_rx receiver
    And set_hitl_request should store questions for TypeScript to poll
    And get_hitl_request should return the stored questions
    And clear_hitl_request should remove the stored questions

  # === NAPI: Getter + response sender ===

  @BUG-118
  Scenario: NAPI getter returns HITL request when session is paused
    Given a session is paused with hitl_request state containing questions
    When TypeScript calls session_get_hitl_request with the session ID
    Then it should return the questions array with id, header, question, and options
    And when the session is not paused or has no hitl_request it should return null

  @BUG-117
  Scenario: NAPI binding converts TypeScript response to Rust HitlResponse
    Given a session is waiting for a HITL response
    When TypeScript calls session_send_hitl_response with answers
    Then the NAPI function should convert the answers to HitlResponse Answered
    And send the response via the session hitl_response_tx channel

  @BUG-117
  Scenario: NAPI binding converts TypeScript cancellation to Rust HitlResponse
    Given a session is waiting for a HITL response
    When TypeScript calls session_send_hitl_response with cancelled true
    Then the NAPI function should convert to HitlResponse Cancelled
    And send the cancellation via the session hitl_response_tx channel

  # === TypeScript: State polling ===

  @BUG-118
  Scenario: useRustSessionState includes hitlRequest in snapshot when paused
    Given a session is paused and has HITL request state
    When useRustSessionState fetches the snapshot
    Then snapshot.hitlRequest should contain the questions array
    And snapshot.isPaused should be true

  @BUG-118
  Scenario: useRustSessionState returns null hitlRequest when not paused
    Given a session is running with no HITL request
    When useRustSessionState fetches the snapshot
    Then snapshot.hitlRequest should be null

  # === TypeScript: Inline rendering ===

  @integration @BUG-118
  Scenario: InputTransition renders HITL question with options inline
    Given isPaused is true and hitlRequest contains a question with options
    When InputTransition renders
    Then it should show the question header and question text
    And it should show selectable options with selected and unselected indicators
    And it should show navigation hints for up down Enter and Esc

  @integration @BUG-118
  Scenario: InputTransition renders freeform-only HITL question
    Given isPaused is true and hitlRequest contains a question without options
    When InputTransition renders
    Then it should show the question text
    And it should show a text input area for freeform response

  @integration @BUG-118
  Scenario: Multi-step HITL advances through questions
    Given isPaused is true and hitlRequest contains 2 questions
    And the user is on question 1 of 2
    When the user selects an option and presses Enter
    Then InputTransition should advance to question 2 of 2
    And the first question answer should be stored

  # === TypeScript: Keyboard handling ===

  @integration @BUG-118
  Scenario: AgentView HITL keyboard handler navigates options
    Given a session is paused with HITL questions containing options
    When the user presses up arrow
    Then the selected option should move up
    When the user presses down arrow
    Then the selected option should move down

  @integration @BUG-118
  Scenario: AgentView HITL keyboard handler submits all answers
    Given a session is paused with HITL questions and all questions answered
    When the user presses Enter on the last question
    Then sessionSendHitlResponse should be called with all collected answers
    And cancelled should be false

  @integration @BUG-118
  Scenario: User cancels HITL with Escape
    Given a session is paused with HITL questions
    When the user presses Escape
    Then sessionSendHitlResponse should be called with cancelled true
    And the handler should unblock and return Cancelled

  # === Cleanup: Remove wrong pattern ===

  @BUG-118
  Scenario: HitlRequest StreamChunk variant removed
    Given the codebase previously had a HitlRequest StreamChunk variant
    Then the HitlRequest variant should not exist in StreamChunk
    And GlobalSessionStreamManager should not have setHitlHandler method
    And GlobalSessionStreamManager should not have clearHitlHandler method
    And GlobalSessionStreamManager should not have handleHitlRequest method
