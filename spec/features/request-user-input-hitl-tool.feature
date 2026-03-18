@TOOL-017
Feature: Request User Input HITL Tool
  """
  Create codelet/tools/src/request_user_input.rs following the InjectSummaryTool pattern: HitlHandler type alias (Arc<dyn Fn(Uuid, HitlRequest) -> Result<HitlResponse, String> + Send + Sync>), global HITL_HANDLERS: RwLock<HashMap<Uuid, HitlHandler>>, pub fn set_hitl_handler/has_hitl_handler/execute_hitl/clear_all_hitl_handlers, and RequestUserInputTool implementing rig::tool::Tool
  HitlRequest contains: questions: Vec<HitlQuestion>. HitlQuestion contains: id: String, header: String, question: String, options: Option<Vec<HitlOption>>. HitlOption contains: label: String, description: String. HitlResponse is an enum: Answered { answers: HashMap<String, HitlAnswer> } | Cancelled. HitlAnswer contains: selected: Vec<String>, other: Option<String>.
  The tool schema presented to the LLM uses the exact JSON schema from the attachment (questions array with id/header/question/options). The tool is registered as 'request_user_input' for all providers. BUG-116 creates the Codex-specific facade wrapper separately.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The tool MUST accept a questions array (1-3 items) where each question has id (snake_case), header (≤12 chars), question (single sentence), and optional options (2-3 mutually exclusive choices with label and description)
  #   2. The tool MUST use the per-session handler pattern (like InjectSummaryHandler, SessionSearchHandler, FspecHandler) with a global RwLock<HashMap<Uuid, Handler>> registry
  #   3. The handler MUST block the tool call (synchronously) until the TUI sends back the user's answers, following the same blocking pattern as pause_for_user
  #   4. Mode-gated: when no HITL handler is registered for the session (headless/non-interactive), return error 'request_user_input is unavailable in the current session mode'
  #   5. Response MUST return JSON with answers keyed by question id, each containing selected (array of chosen labels) and optional other (freeform text)
  #   6. Cancellation MUST return JSON with cancelled: true instead of answers
  #   7. Validation MUST reject: empty questions array, questions with empty id/header/question, header longer than 12 chars, id not in snake_case, fewer than 2 or more than 3 options per question, more than 3 questions
  #   8. The tool module MUST live in codelet/tools/src/request_user_input.rs following the established single-file handler pattern (like inject_summary.rs)
  #   9. The tool MUST NOT depend on PauseHandler - it uses its own separate handler type (HitlHandler) to avoid mixing structured question/answer data with the simple continue/confirm pause semantics
  #   10. The tool MUST be registered in lib.rs with pub use exports and in facade/mod.rs for facade support
  #
  # EXAMPLES:
  #   1. Agent calls request_user_input with two questions (each with options) → handler blocks → TUI renders modal → user selects options and adds freeform text → tool returns answers with selected labels and other text
  #   2. Agent calls request_user_input in headless mode (no handler registered) → tool immediately returns error message about unavailable session mode
  #   3. Agent sends question with header 'This Is Too Long' (15 chars) → validation rejects with error about header length exceeding 12 characters
  #   4. Agent sends question with id 'camelCase' → validation rejects because id must be snake_case
  #   5. User cancels the modal without answering → tool returns {cancelled: true}
  #   6. Agent sends 4 questions → validation rejects because maxItems is 3
  #   7. Agent sends question with 1 option → validation rejects because minItems for options is 2
  #   8. Agent sends question without options (just freeform) → tool accepts it, TUI shows only freeform input for that question, response has empty selected array and populated other field
  #   9. set_hitl_handler registers handler for session → has_hitl_handler returns true → clear_all_hitl_handlers removes all → has_hitl_handler returns false
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to request structured input from the user mid-conversation
    So that make informed decisions based on user preferences without guessing

  Scenario: Request user input with questions and options returns answers
    Given a HITL handler is registered for the current session
    And the handler will return user-selected answers
    When the agent calls request_user_input with 2 questions each having 2 options
    Then the tool should block until the handler returns
    And the response should contain answers keyed by question id
    And each answer should contain selected labels and optional freeform text

  Scenario: Request user input in headless mode returns error
    Given no HITL handler is registered for the current session
    When the agent calls request_user_input with valid questions
    Then the tool should return error "request_user_input is unavailable in the current session mode"

  Scenario: Validation rejects header longer than 12 characters
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question header "This Is Too Long"
    Then the tool should return a validation error about header length exceeding 12 characters

  Scenario: Validation rejects non-snake_case question id
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question id "camelCase"
    Then the tool should return a validation error about id not being snake_case

  Scenario: User cancellation returns cancelled response
    Given a HITL handler is registered for the current session
    And the handler will return a cancellation
    When the agent calls request_user_input with valid questions
    Then the response should contain "cancelled" set to true
    And the response should not contain "answers"

  Scenario: Validation rejects more than 3 questions
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with 4 questions
    Then the tool should return a validation error about exceeding the maximum of 3 questions

  Scenario: Validation rejects fewer than 2 options per question
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question having 1 option
    Then the tool should return a validation error about options requiring at least 2 items

  Scenario: Question without options accepts freeform-only input
    Given a HITL handler is registered for the current session
    And the handler will return a freeform-only answer
    When the agent calls request_user_input with a question without options
    Then the response should contain an answer with empty selected array
    And the answer should contain populated freeform text in the other field

  Scenario: Handler registry lifecycle management
    Given no HITL handler is registered for session "abc-123"
    When set_hitl_handler is called for session "abc-123"
    Then has_hitl_handler should return true for session "abc-123"
    When clear_all_hitl_handlers is called
    Then has_hitl_handler should return false for session "abc-123"

  Scenario: Validation rejects empty questions array
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with an empty questions array
    Then the tool should return a validation error about questions being required

  Scenario: Validation rejects question with empty id
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question having an empty id
    Then the tool should return a validation error about id being required

  Scenario: Validation rejects question with empty header
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question having an empty header
    Then the tool should return a validation error about header being required

  Scenario: Validation rejects question with empty question text
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question having an empty question text
    Then the tool should return a validation error about question text being required

  Scenario: Validation rejects more than 3 options per question
    Given a HITL handler is registered for the current session
    When the agent calls request_user_input with a question having 4 options
    Then the tool should return a validation error about options exceeding the maximum of 3 items
