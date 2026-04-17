@PROV-059
Feature: Copilot x-initiator header always set to 'user' — metadata.mode agent never injected, burning premium quota on every request
  """
  The existing PROV-055 three-layer architecture (CopilotRequestClassifier → CopilotHeaderFacade → CopilotHttpClient) is correct and does not need changes. The fix injects metadata.mode='agent' into the request body at the call sites that construct API requests.
  Fix points: (1) CopilotProvider needs an is_agent parameter on complete_with_tools to inject additional_params metadata. (2) The LlmProvider trait needs to propagate an agent-mode flag. (3) The stream loop / agent loop must track iteration number (0=user-initiated, 1+=agent). (4) DeepSearch, AgentManager, and scheduler must mark their sessions as agent-initiated.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Only the first API call in a user-initiated conversation turn should have x-initiator: user; all subsequent tool-call follow-up iterations must use x-initiator: agent
  #   2. DeepSearch sub-agent sessions must always use x-initiator: agent since they are never user-initiated
  #   3. Subordinate agent sessions (AgentManager spawn) must always use x-initiator: agent
  #   4. Compaction continuation requests (the 'Continue' prompt after DAG construction) must use x-initiator: agent
  #   5. Scheduled job sessions must always use x-initiator: agent
  #   6. The metadata.mode field must be injected into the request body JSON — the existing classifier/facade/middleware pipeline reads it and sets headers accordingly
  #   7. The fix must not change the classifier, facade, or middleware layers — only the upstream callers that construct request bodies
  #
  # EXAMPLES:
  #   1. User types 'fix the login bug' → first API call has x-initiator: user (premium); agent calls Read tool, gets result, sends follow-up → x-initiator: agent (free); agent calls Edit tool, gets result, sends follow-up → x-initiator: agent (free)
  #   2. DeepSearch sub-agent spawned for code investigation → all its API calls have x-initiator: agent (free); user's quota not affected by sub-agent work
  #   3. Compaction triggers mid-session → the compaction API call and the continuation 'Continue' prompt both use x-initiator: agent (free)
  #   4. User spawns subordinate agent via AgentManager → all subordinate's API calls use x-initiator: agent (free)
  #   5. Scheduled agent job runs automatically → all its API calls use x-initiator: agent (free)
  #   6. Request body JSON includes metadata.mode='agent' field → CopilotRequestClassifier.classify() returns is_agent=true → CopilotHeaderFacade sets x-initiator: agent
  #
  # ========================================
  Background: User Story
    As a Copilot user
    I want to have agent-initiated requests (tool calls, subagents, compaction) correctly flagged with x-initiator: agent
    So that my premium quota is only consumed by genuine user-initiated messages

  @unit
  Scenario: First user message uses x-initiator user, tool follow-ups use agent
    Given a Copilot provider session with the agent loop
    When the user sends their first message "fix the login bug"
    Then the first API request should have x-initiator set to "user"
    When the agent calls the Read tool and receives a tool result
    And the agent loop sends the follow-up API request with tool results
    Then the follow-up API request should have x-initiator set to "agent"
    When the agent calls the Edit tool and receives a tool result
    And the agent loop sends the next follow-up API request
    Then that API request should also have x-initiator set to "agent"

  @unit
  Scenario: DeepSearch sub-agent requests are always agent-initiated
    Given a Copilot provider session
    When a DeepSearch sub-agent is spawned for code investigation
    Then the DeepSearch request config should include metadata.mode set to "agent"
    And all API calls made by the sub-agent should have x-initiator set to "agent"

  @unit
  Scenario: Compaction continuation uses agent initiator
    Given a Copilot provider session that has triggered compaction
    When the DAG construction completes and a "Continue" prompt is sent
    Then the continuation API request should have x-initiator set to "agent"

  @unit
  Scenario: Subordinate agent sessions use agent initiator
    Given a Copilot provider session
    When a subordinate agent is spawned via AgentManager
    Then all API requests made by the subordinate should have x-initiator set to "agent"

  @unit
  Scenario: Scheduled job sessions use agent initiator
    Given a scheduled agent job configured with the Copilot provider
    When the scheduled job session makes API requests
    Then all API requests should have x-initiator set to "agent"

  @unit
  Scenario: metadata.mode agent field flows through classifier to header
    Given a Copilot API request body with metadata.mode set to "agent"
    When CopilotRequestClassifier.classify() processes the body
    Then is_agent should be true
    And CopilotHeaderFacade should set x-initiator to "agent"

  @unit
  Scenario: Request without metadata.mode defaults to user initiator
    Given a Copilot API request body without a metadata.mode field
    When CopilotRequestClassifier.classify() processes the body
    Then is_agent should be false
    And CopilotHeaderFacade should set x-initiator to "user"
