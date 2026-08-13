@done
@rust
@authentication
@providers
@PROV-055
Feature: GitHub Copilot HTTP middleware, facades & endpoint routing
  """
  Module layout: rust/providers/src/copilot/{refreshing_client.rs (CopilotHttpClient middleware), header_facade.rs (CopilotHeaderFacade), classifier.rs (CopilotRequestClassifier), endpoint.rs (CopilotEndpointFacade), behavior_facade.rs (CopilotBehaviorFacade trait + 3 impls + selector)}
  Facade pattern references: CacheOptimizationFacade::build_headers (cache_optimization.rs:96) as template for CopilotHeaderFacade; ThinkingConfigFacade + select_claude_facade(is_oauth) (thinking_config.rs, system_prompt.rs:427) as template for CopilotBehaviorFacade + select_copilot_behavior_facade(model_id); RefreshingClaudeClient / RefreshingCodexClient as template for CopilotHttpClient middleware
  Each facade/middleware gets #[cfg(test)] mod tests with pure unit tests (no HTTP client needed): header building (given-classification-then-headers), classifier (given-body-then-classification), endpoint selection (given-model-id-then-endpoint), behavior selector (given-model-id-then-facade-type)
  Integration test uses wiremock or httpmock to stand up a mock Copilot API server and exercises the full CopilotHttpClient middleware pipeline: sends a real rig request, mock server captures headers + URL, assertion checks headers present and endpoint routing correct — mirrors pattern used in PROV-054 oauth integration tests
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CopilotProvider implements the LlmProvider trait and is registered as ProviderType::GitHubCopilot in rust/providers/src/manager.rs
  #   2. CopilotHttpClient implements rig::http_client::HttpClientExt as a middleware layer that wraps every outgoing request, mirroring the RefreshingClaudeClient / RefreshingCodexClient pattern
  #   3. CopilotHeaderFacade injects the required header set on every request: x-initiator (user|agent), User-Agent (rust/<version>), Authorization (Bearer <access_token>), Openai-Intent (conversation-edits), and conditional Copilot-Vision-Request (true only when vision content is present)
  #   4. CopilotRequestClassifier is a pure function fn classify(body: &serde_json::Value) -> RequestClassification that returns { is_vision: bool, is_agent: bool } by inspecting chat/completions, responses, or Anthropic messages body shape — no IO, no state
  #   5. CopilotEndpointFacade::select(model_id) returns Endpoint::Responses when the model is gpt-N where N >= 5 AND the model is not gpt-5-mini; otherwise returns Endpoint::ChatCompletions
  #   6. CopilotBehaviorFacade is a trait analogous to ThinkingConfigFacade that encapsulates model-family-specific behavior (reasoning_effort variants, chat.params mutations, reasoning_opaque round-trip) with three implementations: CopilotGptBehaviorFacade, CopilotClaudeBehaviorFacade, CopilotGeminiBehaviorFacade
  #   7. select_copilot_behavior_facade(model_id: &str) -> Box<dyn CopilotBehaviorFacade> dispatches by model name prefix: gpt-* → Gpt, claude-* → Claude, gemini-* → Gemini — mirrors select_claude_facade(is_oauth) pattern from system_prompt.rs
  #   8. Copilot reuses existing OpenAI-compatible tool facades from codelet_tools::facade (OpenAIFspecFacade, openai_bridge_tool, etc.) without defining a new tool facade family, because Copilot wire format is always OpenAI-shaped regardless of underlying model family
  #   9. CopilotResponsesSystemPromptFacade is a new implementation of BoxedSystemPromptFacade used only for /responses endpoint models; chat/completions models use the existing OpenAISystemPromptFacade
  #   10. The endpoint base URL is api.githubcopilot.com for github.com deployments and copilot-api.<enterprise-domain> for enterprise deployments; CopilotProvider reads the deployment type from the persisted credential file written by PROV-054
  #
  # EXAMPLES:
  #   1. User selects gpt-4o-copilot in the TUI → CopilotEndpointFacade::select returns ChatCompletions → request is sent to https://api.githubcopilot.com/chat/completions with headers { x-initiator: user, User-Agent: rust/<version>, Authorization: Bearer <token>, Openai-Intent: conversation-edits } and no Copilot-Vision-Request header
  #   2. User picks gpt-5 in the TUI model menu → sends a chat message → response streams back from the /responses endpoint and includes a reasoning_opaque blob; follow-up turn echoes that reasoning_opaque so GPT-5 can continue its chain of thought
  #   3. User picks gpt-5-mini for a quick summarization task → request goes to /chat/completions (not /responses) because gpt-5-mini is explicitly excluded from the Responses-API rule → user gets a streamed summary
  #   4. User attaches a screenshot and asks claude-sonnet-4.5 (via Copilot) to describe it → codelet detects image parts in the outgoing request → the Copilot-Vision-Request: true header is added automatically → user receives a description of the image
  #   5. Enterprise user has logged in with deploymentType=enterprise and enterpriseUrl=ghe.example.com → sends a gpt-4o chat request → codelet routes the request to https://copilot-api.ghe.example.com/chat/completions (not the default github.com base) → user gets the response
  #   6. User runs codelet as an autonomous agent workflow against gpt-5-codex → each outgoing request carries x-initiator: agent (not user) so GitHub can correctly bill/rate-limit as agent traffic
  #
  # ========================================
  Background: User Story
    As a codelet user authenticated with GitHub Copilot
    I want to send chat requests to Copilot models from the codelet TUI
    So that every request is correctly authorized, classified, and routed to the right Copilot API endpoint based on the selected model

  Scenario: Chat completion request to gpt-4o-copilot uses /chat/completions endpoint with required Copilot headers
    Given I am logged in to github-copilot with a github.com deployment credential
    And the credential contains a valid access_token
    When I select the model "gpt-4o-copilot" in the TUI and send a text-only chat message
    Then CopilotEndpointFacade::select("gpt-4o-copilot") should return Endpoint::ChatCompletions
    And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    And the request should include header "x-initiator: user"
    And the request should include header "User-Agent" starting with "rust/"
    And the request should include header "Authorization: Bearer <access_token>"
    And the request should include header "Openai-Intent: conversation-edits"
    And the request should NOT include the "Copilot-Vision-Request" header

  Scenario: gpt-5 model is routed to the /responses endpoint with reasoning_opaque round-trip
    Given I am logged in to github-copilot with a github.com deployment credential
    When I select the model "gpt-5" in the TUI and send a chat message
    Then CopilotEndpointFacade::select("gpt-5") should return Endpoint::Responses
    And the outgoing request URL should be "https://api.githubcopilot.com/responses"
    And the selected behavior facade should be CopilotGptBehaviorFacade
    And the system prompt facade should be CopilotResponsesSystemPromptFacade
    When the Copilot API returns a response containing a "reasoning_opaque" field
    Then the next turn request should include the previous "reasoning_opaque" value unchanged

  Scenario: gpt-5-mini is excluded from the Responses API rule and uses /chat/completions
    Given I am logged in to github-copilot with a github.com deployment credential
    When I select the model "gpt-5-mini" in the TUI and send a chat message
    Then CopilotEndpointFacade::select("gpt-5-mini") should return Endpoint::ChatCompletions
    And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    And the system prompt facade should be OpenAISystemPromptFacade
    And the request should include header "Openai-Intent: conversation-edits"

  Scenario: Image attachment triggers Copilot-Vision-Request header on claude-sonnet-4.5
    Given I am logged in to github-copilot with a github.com deployment credential
    When I select the model "claude-sonnet-4.5" in the TUI
    And I attach an image and send a message asking to describe the image
    Then CopilotRequestClassifier::classify should detect is_vision = true on the request body
    And the outgoing request should include header "Copilot-Vision-Request: true"
    And the outgoing request URL should be "https://api.githubcopilot.com/chat/completions"
    And the selected behavior facade should be CopilotClaudeBehaviorFacade

  Scenario: Enterprise deployment routes requests to the copilot-api enterprise subdomain
    Given I am logged in to github-copilot with an enterprise deployment credential
    And the credential has enterpriseUrl set to "ghe.example.com"
    When I select the model "gpt-4o" in the TUI and send a chat message
    Then the outgoing request URL should be "https://copilot-api.ghe.example.com/chat/completions"
    And the request should include header "Authorization: Bearer <access_token>"
    And the request should include header "Openai-Intent: conversation-edits"

  Scenario: Agent-mode workflow sets x-initiator header to agent instead of user
    Given I am logged in to github-copilot with a github.com deployment credential
    And codelet is running in autonomous agent mode
    When codelet sends a chat request to the model "gpt-5-codex"
    Then CopilotRequestClassifier::classify should detect is_agent = true on the request body
    And the outgoing request should include header "x-initiator: agent"
    And the outgoing request should NOT include header "x-initiator: user"
