@BUG-116
Feature: Codex facade maps request_user_input to HITL tool
  """
  Follow the existing facade pattern: HitlToolFacade trait + InternalHitlParams in traits.rs, CodexRequestUserInputFacade in codex.rs, HitlToolFacadeWrapper in wrapper.rs
  The wrapper calls execute_hitl() directly (not through RequestUserInputTool) — the wrapper IS the rig::tool::Tool, there's no inner tool to delegate to
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexRequestUserInputFacade must implement a new HitlToolFacade trait and map the Codex-native request_user_input schema (questions array with id/header/question/options) to InternalHitlParams
  #   2. The facade must pass the questions array through to execute_hitl unchanged — the Codex schema is structurally identical to the HITL tool schema
  #   3. Cancellation from the HITL handler (HitlResponse::Cancelled) must be converted to a tool error with message 'request_user_input was cancelled before receiving a response' — not returned as a JSON success response
  #   4. Mode-gated: when no HITL handler is registered (headless/non-interactive), the wrapper returns error 'request_user_input is unavailable in the current session mode'
  #   5. The facade tool definition schema must have additionalProperties: false to match the existing Codex facade convention
  #   6. The facade must be registered in Codex create_rig_agent using HitlToolFacadeWrapper, replacing the direct RequestUserInputTool registration
  #   7. Response for successful answers must return JSON with answers keyed by question id, each containing selected (array of chosen labels) and optional other (freeform text)
  #
  # EXAMPLES:
  #   1. Codex model calls request_user_input with 2 questions each having 2 options → facade passes questions to execute_hitl → handler returns answers → wrapper returns JSON with answers keyed by question id
  #   2. Codex model calls request_user_input in headless mode (no handler registered) → wrapper immediately returns tool error about unavailable session mode
  #   3. User cancels the HITL modal → handler returns HitlResponse::Cancelled → wrapper converts to tool error 'request_user_input was cancelled before receiving a response'
  #   4. Codex model sends question with header 'This Is Too Long' (15 chars) → execute_hitl validation rejects with error about header length
  #   5. Facade schema inspection shows additionalProperties: false on the top-level object
  #   6. HitlToolFacadeWrapper is registered in Codex create_rig_agent and replaces the direct RequestUserInputTool registration
  #   7. Codex model calls request_user_input with question without options (freeform only) → facade passes through → handler returns freeform answer → wrapper returns JSON response
  #
  # ========================================
  Background: User Story
    As a Codex LLM agent
    I want to call request_user_input with the Codex-native schema
    So that the tool resolves to the provider-agnostic HITL tool with proper Codex response formatting

  Scenario: CodexRequestUserInputFacade maps questions to InternalHitlParams and returns answers
    Given a HITL handler is registered for the current session
    And the handler will return user-selected answers
    When the Codex model calls request_user_input with 2 questions each having 2 options
    Then the facade passes questions to execute_hitl unchanged
    And the wrapper returns JSON with answers keyed by question id
    And each answer contains selected labels and optional freeform text

  Scenario: Headless mode returns tool error about unavailable session mode
    Given no HITL handler is registered for the current session
    When the Codex model calls request_user_input with valid questions
    Then the wrapper returns a tool error
    And the error message contains "request_user_input is unavailable in the current session mode"

  Scenario: Cancellation converts to Codex-specific tool error
    Given a HITL handler is registered for the current session
    And the handler will return a cancellation
    When the Codex model calls request_user_input with valid questions
    Then the wrapper returns a tool error
    And the error message is "request_user_input was cancelled before receiving a response"

  Scenario: Validation rejects invalid questions via execute_hitl
    Given a HITL handler is registered for the current session
    When the Codex model calls request_user_input with a question header "This Is Too Long"
    Then the wrapper returns a tool error about header length exceeding 12 characters

  Scenario: Facade schema has additionalProperties false
    Given a CodexRequestUserInputFacade instance
    When the tool definition schema is inspected
    Then the schema has additionalProperties set to false
    And the schema has "questions" in the required array

  Scenario: Facade is registered in Codex create_rig_agent
    Given a Codex agent built with create_rig_agent
    When the agent tool definitions are inspected
    Then the tool list contains "request_user_input"
    And request_user_input uses HitlToolFacadeWrapper with CodexRequestUserInputFacade

  Scenario: Freeform-only question without options returns answer
    Given a HITL handler is registered for the current session
    And the handler will return a freeform-only answer
    When the Codex model calls request_user_input with a question without options
    Then the wrapper returns JSON with an answer containing empty selected array
    And the answer contains populated freeform text in the other field
