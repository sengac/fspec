@done
@rlcd-002
@rlcd
@tools
@RLCD-002
Feature: Decision() first-class rig tool for typed RLCD decisions (choice/score/noul)
  """
  Public API surface contract that tests may link against (specifying phase, 2026-09-24): client.rs — `RlcdClient` async trait { classify(base_url: &str, model: &str, state: serde_json::Value, questions: Vec<RlcdQuestion>, budget: Duration) -> Result<RlcdResponse, RlcdError> } + `HttpRlcdClient` (reqwest, POST {base}/v1/classifier, Jev/Simple-Jev v1 shape: {model, state, questions: {qid: {question, criteria?}}}); error mapping 422->RlcdError::Validation(detail), 429->RlcdError::Busy{retry_after}, 500/timeout->RlcdError::Server, connect->RlcdError::Unreachable; RlcdQuestion{qid, question, kind: choice|score|noul, criteria: serde_json::Value}; decision.rs — `DecisionTool` (impl rig::Tool, ToolError Blocked/Validation/Execution, hand-written JSON schema in definition()), `DecisionArgs{state, questions, model, url}`, validate_args() (state non-empty, 1..=100 questions, choice criteria map 2..=50, score criteria array 2..=50, noul optional {true,false}); render: choice => argmax+confidence+probabilities, score => scores+legend, noul => P(true); call() pipeline = pre_tool_hook_check -> validate -> supervisor.ensure_ready (RLCD-001 shared snapshot, bounded ~10s wait; unreachable => ToolError fail-open structured error with hint) -> classify -> render. Registration: all 7 create_rig_agent sites (claude.rs:553/557, openai.rs:465/469, gemini.rs:211/215, codex/mod.rs:423/427, zai.rs:281/285, custom_provider.rs:305/309+330/334, copilot/rig_agent.rs:102/106); NOT in DeepSearch sub-agent lists. Prompt design (AUDIT-001): state plain string, most-important content FIRST (tail-truncated at 1024), criteria <= ~8 words. Files <300 lines each.
  """

  Background: User Story
    As a AI agent
    I want to ask typed decision questions (choice/score/noul) to the RLCD engine via a first-class Decision tool
    So that I get calibrated, non-hallucinatory decisions on intent/urgency/risk instead of guessing from my own reasoning

  # ========================================
  # QUESTION TYPES (rule 5; examples 1, 2, 4)
  # ========================================
  Scenario: A choice question returns the argmax criterion with confidence and full probabilities
    Given a mock RLCD server whose /health reports model "typed-decisions"
    And the state "customer requests refund of order 123"
    When the Decision tool is called with choice question "should we refund?" and criteria {proceed, hold, ask_user}
    Then the response reports the argmax criterion "proceed"
    And the response reports the confidence for "proceed"
    And the response reports the full probability distribution for all three criteria

  Scenario: A score question returns per-criterion scores with a legend
    Given a mock RLCD server whose /health reports model "typed-decisions"
    When the Decision tool is called with a score question whose criteria are an array in rubric order
    Then the response reports a score for each criterion
    And the response includes a legend mapping each qid to its label

  Scenario: A noul question returns P(true) as a single probability
    Given a mock RLCD server whose /health reports model "typed-decisions"
    When the Decision tool is called with a noul question without criteria
    Then the response reports P(true) as a single probability

  # ========================================
  # BATCH QUESTIONS (rule 1; example 5)
  # ========================================
  Scenario: One request carrying both a choice and a score question returns both results
    Given a mock RLCD server whose /health reports model "typed-decisions"
    And the state "customer requests refund of order 123"
    When the Decision tool is called with one choice question and one score question in a single request
    Then the response renders the choice result with argmax and probabilities
    And the response renders the score result with scores and legend

  # ========================================
  # ARG VALIDATION (rules 4, 5; examples 6, 8)
  # ========================================
  Scenario: A call without state is rejected before any network call
    Given a mock RLCD server that records every request it receives
    When the Decision tool is called with an empty state and one valid question
    Then the tool returns a Validation error naming the state field
    And the mock server received zero requests

  Scenario: A call with zero questions is rejected before any network call
    Given a mock RLCD server that records every request it receives
    When the Decision tool is called with a valid state and no questions
    Then the tool returns a Validation error naming the questions field
    And the mock server received zero requests

  Scenario: A call with more than 100 questions is rejected before any network call
    Given a mock RLCD server that records every request it receives
    When the Decision tool is called with a valid state and 101 questions
    Then the tool returns a Validation error explaining the 1 to 100 questions bound
    And the mock server received zero requests

  Scenario: A choice question with a single criterion is rejected with the bound explained
    Given a mock RLCD server that records every request it receives
    When the Decision tool is called with a choice question whose criteria map has exactly one entry
    Then the tool returns a Validation error explaining the 2 to 50 criteria bound
    And the mock server received zero requests

  Scenario: A score question with a single-entry criteria array is rejected
    Given a mock RLCD server that records every request it receives
    When the Decision tool is called with a score question whose criteria array has exactly one entry
    Then the tool returns a Validation error explaining the 2 to 50 criteria bound
    And the mock server received zero requests

  # ========================================
  # ERROR MAPPING (rule 3; examples 3, 9, 10)
  # ========================================
  Scenario: A 422 response surfaces the first detail verbatim
    Given a mock RLCD server that answers /v1/classifier with 422 and detail "Loaded model is 'typed-decisions'"
    When the Decision tool is called with a valid choice question
    Then the tool returns a Validation error containing the detail "Loaded model is 'typed-decisions'"

  Scenario: A 429 response with Retry-After reports the busy wait
    Given a mock RLCD server that answers /v1/classifier with 429 and Retry-After "5"
    When the Decision tool is called with a valid choice question
    Then the tool returns an Execution error matching "RLCD queue busy, retry in 5s"

  Scenario: A 500 response maps to an Execution error
    Given a mock RLCD server that answers /v1/classifier with 500
    When the Decision tool is called with a valid choice question
    Then the tool returns an Execution error with the server failure detail

  Scenario: A timed-out request maps to an Execution error
    Given a mock RLCD server that delays /v1/classifier past the request budget
    When the Decision tool is called with a valid choice question
    Then the tool returns an Execution error with the timeout detail

  # ========================================
  # FAIL-OPEN (rule 3; example 7)
  # ========================================
  Scenario: An unreachable RLCD returns a fail-open structured error with a hint
    Given no RLCD server is reachable at the configured url
    When the Decision tool is called with a valid choice question
    Then the tool returns a structured error identifying RLCD as unreachable
    And the error includes a hint to start the backend or set rlcd.url
    And the caller is not hard-blocked

  Scenario: The request echoes the server-reported model name
    Given a mock RLCD server whose /health reports model "custom-variant"
    When the Decision tool is called with a valid choice question
    Then the classifier request body carries model "custom-variant"
    And the request never hardcodes a model name different from the /health report

  # ========================================
  # INTEGRATION (rule 2; example 11)
  # ========================================
  Scenario: The Decision tool is registered in every provider rig agent
    Given the RLCD supervisor is available
    When each provider's rig agent is created
    Then the tool list includes the Decision tool alongside DeepSearch and Schedule
    And the DeepSearch sub-agent tool lists are unchanged
