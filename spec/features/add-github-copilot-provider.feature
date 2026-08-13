@done
@rust
@facade-pattern
@authentication
@providers
@PROV-053
Feature: Add GitHub Copilot provider
  """
  Facade pattern ref: see cache_optimization.rs CacheOptimizationFacade (PROV-051), thinking_config.rs ThinkingConfigFacade/select_claude_facade (PROV-005), system_prompt.rs SystemPromptFacade (TOOL-008), and traits.rs ToolFacade/FileToolFacade/BashToolFacade (TOOL-001..006)
  Module layout: rust/providers/src/copilot/{mod.rs (CopilotProvider impl LlmProvider), oauth.rs (device flow), auth.rs (credential persistence mirroring claude_auth.rs/codex_auth.rs), refreshing_client.rs (CopilotHttpClient middleware), behavior_facade.rs (CopilotBehaviorFacade trait + 3 impls + selector), header_facade.rs (CopilotHeaderFacade), classifier.rs (CopilotRequestClassifier), endpoint.rs (CopilotEndpointFacade), models.rs (CopilotModelCatalogService)}
  Provider registration: add ProviderType::GitHubCopilot variant in rust/providers/src/manager.rs alongside Claude, OpenAI, Codex, Gemini, Zai — the manager remains the single dispatch point and the facade selection happens inside CopilotProvider::new()
  Each facade trait implementation gets unit tests in #[cfg(test)] mod tests — mirrors cache_optimization.rs:143, system_prompt.rs:442+, thinking_config.rs — enabling isolated testing of header building, body classification, endpoint selection, and behavior selection without spinning up an HTTP client
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. OAuth device flow uses GitHub OAuth App client_id Ov23li8tweQw6odWQebz with scope read:user (mirroring opencode copilot.ts:11)
  #   2. github.com deployments POST to https://github.com/login/device/code then poll https://github.com/login/oauth/access_token
  #   3. Enterprise deployments POST to https://<enterprise-domain>/login/device/code and poll https://<enterprise-domain>/login/oauth/access_token
  #   4. Login prompts for deploymentType (github.com or enterprise) and conditionally prompts for enterpriseUrl only when enterprise is selected
  #   5. Enterprise URL is normalized by stripping scheme and trailing slash (copilot.ts normalizeDomain line 15)
  #   6. On authorization_pending response, sleep for (interval + 3s safety margin) then poll again
  #   7. On slow_down response, increase polling interval by 5 seconds per RFC 8628 §3.5 (use server-provided interval if present)
  #   8. On access_token response, persist credential with refresh and access fields set to the same token value and expires set to 0 (never expires)
  #   9. Credential is persisted in auth.json under key github-copilot with file permissions 0600
  #   10. CopilotProvider implements LlmProvider (rust/providers/src/lib.rs:84) and delegates all provider-specific behavior to facades following the same pattern as ClaudeProvider, CodexProvider, GeminiProvider, ZaiProvider, OpenAIProvider
  #   11. Copilot reuses existing OpenAI-compatible tool facades from codelet_tools::facade (OpenAIFspecFacade, openai_bridge_tool, etc.) rather than defining a new family because the Copilot wire format is always OpenAI-shaped (chat/completions or responses)
  #   12. New CopilotBehaviorFacade trait analogous to ThinkingConfigFacade and SystemPromptFacade encapsulates model-family-specific behavior: reasoning_effort variants (GPT vs Claude vs Gemini), chat.params mutations (unset maxOutputTokens for GPT), reasoning_opaque round-trip
  #   13. select_copilot_behavior_facade(model_id) returns CopilotGptBehaviorFacade, CopilotClaudeBehaviorFacade, or CopilotGeminiBehaviorFacade based on model name — mirrors select_claude_facade(is_oauth) pattern from system_prompt.rs:427
  #   14. CopilotHeaderFacade builds the exact header set (x-initiator, User-Agent, Authorization, Openai-Intent, conditional Copilot-Vision-Request) — mirrors CacheOptimizationFacade::build_headers pattern from cache_optimization.rs:96
  #   15. CopilotRequestClassifier is a pure function (body: &Value) -> RequestClassification that returns { is_vision: bool, is_agent: bool } by inspecting chat/completions, responses, or Anthropic messages body shape — mirrors opencode copilot.ts:78-130 logic
  #   16. CopilotEndpointFacade::select(model_id) returns ChatCompletions or Responses endpoint per the rule: GPT version >= 5 except gpt-5-mini goes to Responses, everything else to ChatCompletions — mirrors opencode provider.ts:63-67
  #   17. CopilotHttpClient implements rig::http_client::HttpClientExt as a middleware layer that composes CopilotRequestClassifier + CopilotHeaderFacade on every outgoing request — follows the RefreshingCodexClient / RefreshingClaudeClient pattern from rust/providers/src/
  #   18. Copilot system prompt uses the existing OpenAISystemPromptFacade for chat/completions models and a new CopilotResponsesSystemPromptFacade for /responses models — both plug into BoxedSystemPromptFacade from codelet_tools::facade::system_prompt
  #   19. Copilot model catalog is fetched by a CopilotModelCatalogService (single responsibility: GET /models + parse + filter + merge) — not a facade because there is no provider variation; analogous to models/ subdirectory pattern in rust/providers/src/
  #
  # EXAMPLES:
  #   1. User runs `codelet auth login github-copilot` → CLI prompts deploymentType (github.com); device code returned; user enters code at https://github.com/login/device; polling loop succeeds; credential persisted at ~/.fspec/credentials/copilot_auth.json with mode 0600
  #   2. User runs `codelet auth login github-copilot` with deploymentType enterprise; CLI prompts for enterpriseUrl (ghe.example.com); device code flow completes against ghe.example.com; credential persisted with enterpriseUrl field; subsequent API calls hit https://copilot-api.ghe.example.com
  #   3. User selects model gpt-4o-copilot in codelet TUI; codelet sends a /chat/completions request to api.githubcopilot.com with Bearer token, User-Agent rust/<version>, Openai-Intent conversation-edits, x-initiator user, and receives streamed response
  #   4. User selects model gpt-5 in codelet TUI; codelet routes the request to /responses endpoint (not /chat/completions) because shouldUseCopilotResponsesApi returns true for gpt-5; receives response with reasoning_opaque field that is round-tripped on the next turn
  #   5. User selects model gpt-5-mini for small-model usage (summarization); codelet routes to /chat/completions (not /responses) because gpt-5-mini is explicitly excluded from the Responses API rule
  #   6. User attaches an image and asks claude-sonnet-4.5 (via Copilot) to describe it; codelet detects image parts in the request body and adds Copilot-Vision-Request: true header; Copilot API returns vision response
  #   7. User runs `codelet auth logout github-copilot`; credential file ~/.fspec/credentials/copilot_auth.json is deleted; next TUI open shows github-copilot as unauthenticated
  #   8. User opens the model picker after a successful Copilot login; codelet fetches /models from api.githubcopilot.com, filters by model_picker_enabled=true, merges with static catalog, and displays the resulting list ordered with gpt-5-mini and claude-haiku-4.5 pushed to the top as small-model candidates
  #
  # ========================================
  Background: User Story
    As a fspec user with a GitHub Copilot subscription
    I want to sign in to GitHub Copilot via OAuth device flow and use Copilot-hosted models through fspec
    So that I can use my existing Copilot entitlement without managing another API key

  Scenario: Login to github.com Copilot deployment via OAuth device flow
    Given I have an active GitHub Copilot subscription on github.com
    And I have no existing github-copilot credential on disk
    When I run `codelet auth login github-copilot`
    And I select deploymentType "github.com" at the CLI prompt
    And I enter the displayed device code at https://github.com/login/device and approve the request
    Then the polling loop should succeed with an access_token response
    And a credential should be persisted at "~/.fspec/credentials/copilot_auth.json" with file mode 0600
    And the credential should contain access and refresh tokens set to the same GitHub OAuth token value
    And the credential expires field should be 0

  Scenario: Login to GitHub Enterprise Copilot deployment with enterprise URL
    Given I have an active GitHub Copilot subscription on a GitHub Enterprise instance
    And I have no existing github-copilot credential on disk
    When I run `codelet auth login github-copilot`
    And I select deploymentType "enterprise" at the CLI prompt
    And I enter "ghe.example.com" at the enterpriseUrl prompt
    And I complete the device code flow against ghe.example.com
    Then a credential should be persisted with the enterpriseUrl field set to "ghe.example.com"
    And subsequent Copilot API calls should be routed to "https://copilot-api.ghe.example.com"

  Scenario: Chat completion request to gpt-4o-copilot uses /chat/completions endpoint with Copilot headers
    Given I am logged in to github-copilot with a valid credential
    And I have selected model "gpt-4o-copilot" in the codelet TUI
    When I send a chat message
    Then codelet should send the request to "api.githubcopilot.com/chat/completions"
    And the request should include an "Authorization: Bearer <token>" header
    And the request should include a "User-Agent: rust/<version>" header
    And the request should include an "Openai-Intent: conversation-edits" header
    And the request should include an "x-initiator: user" header
    And the response should be streamed back to the TUI

  Scenario: gpt-5 model is routed to the /responses endpoint with reasoning_opaque round-trip
    Given I am logged in to github-copilot with a valid credential
    And I have selected model "gpt-5" in the codelet TUI
    When I send a chat message
    Then codelet should route the request to the "/responses" endpoint
    And the response should include a "reasoning_opaque" field
    And on the next turn the "reasoning_opaque" field should be round-tripped back in the request

  Scenario: gpt-5-mini is excluded from the Responses API rule and uses /chat/completions
    Given I am logged in to github-copilot with a valid credential
    And I have selected model "gpt-5-mini" for a summarization task
    When I send a chat message
    Then codelet should route the request to "/chat/completions" and not to "/responses"

  Scenario: Image attachment triggers Copilot-Vision-Request header on claude-sonnet-4.5
    Given I am logged in to github-copilot with a valid credential
    And I have selected model "claude-sonnet-4.5" in the codelet TUI
    When I attach an image and ask the model to describe it
    Then codelet should detect image parts in the request body
    And the request should include a "Copilot-Vision-Request: true" header
    And the Copilot API should return a vision response

  Scenario: Logout deletes the github-copilot credential file
    Given I am logged in to github-copilot with a credential at "~/.fspec/credentials/copilot_auth.json"
    When I run `codelet auth logout github-copilot`
    Then the file "~/.fspec/credentials/copilot_auth.json" should be deleted
    And opening the codelet TUI should show github-copilot as unauthenticated

  Scenario: Model picker fetches the live catalog from /models with no static merge
    Given I am logged in to github-copilot with a valid credential
    When I open the model picker in the codelet TUI
    Then codelet should fetch the model catalog from "api.githubcopilot.com/models"
    And the catalog should contain only models with "model_picker_enabled: true"
    And the catalog should be exactly what the endpoint returned, with no merging or static fallback
