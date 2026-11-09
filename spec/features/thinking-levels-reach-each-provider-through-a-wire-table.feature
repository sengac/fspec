@BUG-582
@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-010
Feature: Thinking levels reach each provider through an explicit wire table
  """
  Architecture notes:
  - Lives in codelet-agent-loop, rust/agent-loop/src/thinking_config.rs. ThinkingWire is chosen from the dispatched provider (ProviderManager current_provider_type, facade_override and profile name), never from the model id: claude is anthropic, openai and github-copilot are openai-chat, an openai local-server profile is local-openai, codex is openai-responses, gemini is gemini, zai is zai and a Rhai custom provider (no facade) is rhai. A custom provider with a facade uses its facade's wire; an unknown facade has no wire.
  - ThinkingTarget carries the wire, the model ("provider/model") and its ReasoningCapability, read per turn from the ProviderManager's selected ModelInfo reasoning_options (MODEL-009). Local profiles and custom providers have no declared capability until MODEL-016, and neither has a model missing from the catalog: their level is sent unchecked (product owner, 2026-11-04). MODEL-017: a custom provider model's capability is the thinkingLevels its config entry declares; one that declares none has no reasoning control and its level is refused (rhai-custom-provider-models-declare-thinking-levels.feature, which replaces this feature's former Rhai scenario).
  - The turn's level is the session's chosen level: Off, or the named efforts low / medium / high for Low / Medium / High. Budgets and other effort names reach the same table when MODEL-011 lets the session hold them. A per-turn raw thinking_config is refused.
  - Effort and budget levels are checked against the capability before anything is built; nothing is remapped. Off is the model's own off form: effort none (openai-chat, local-openai, codex), the toggle (gemini thinkingBudget 0, zai thinking disabled) or the BUG-439 Claude forms (claude_off_config in codelet-tools).
  - codelet-tools loses the facade request_config tables (Claude budgets 4096/16000/32000, Gemini 2.5 budgets 2048/4096/8192, the Gemini 3 and Claude adaptive level maps and request_config_for_model): no legacy level mapping remains (product owner, 2026-11-04).
  - The bundled codex.rhai template builds its reasoning object from the level it receives and declares Codex's levels (MODEL-017).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The wire form is chosen by the dispatched provider, never by substrings of the model id: claude is anthropic, openai (API key) and github-copilot are openai-chat, an openai local-server profile is local-openai, codex is openai-responses, gemini is gemini, zai is zai, and a Rhai custom provider is rhai; a custom provider with a facade uses its facade's form
  #   2. openai-chat sends reasoning_effort with the level's effort name and openai-responses (codex) sends reasoning {effort, summary: auto}; the name is sent unchanged, including xhigh, max and names outside the canonical list
  #   3. anthropic: a named effort sends thinking {type: adaptive, display: summarized} with output_config.effort; a token budget sends thinking {type: enabled, budget_tokens}; Off keeps the BUG-439 forms
  #   4. gemini: a named effort sends thinkingConfig {includeThoughts: true, thinkingLevel}; a token budget sends thinkingConfig {includeThoughts: true, thinkingBudget}
  #   5. zai has only the toggle's off form; a named effort or budget for zai is refused because no zai effort form is known
  #   6. rhai: the script receives request.thinking_config as {level, capability} and owns the wire; the bundled codex.rhai template builds its reasoning object from the level
  #   7. A level is sent unchanged or refused; nothing is remapped and no form has a catch-all that silently sends nothing
  #   8. A session with no chosen level sends no thinking parameter on any provider (MODEL-013)
  #   9. A level the model's capability does not advertise is refused before the request is sent, naming the model and its supported levels (MODEL-009)
  #   10. A model with no declared reasoning capability gets the chosen level unchecked through its provider's form (product owner, 2026-11-04). Superseded: a model whose levels fspec cannot know (custom-provider models that declare none, cloud models missing from models.dev) has no reasoning control and a held level is refused at the turn (product owner, 2026-11-05, MODEL-011; MODEL-017)
  #   11. Strict checking, no legacy mapping: fspec's budget tables and the Z.AI enabled mapping are removed (product owner, 2026-11-04)
  #   12. local-openai sends chat_template_kwargs thinking and enable_thinking, both (level != none), alongside reasoning_effort (product owner, 2026-11-04)
  #   13. Off is expressed in the model's own off form or refused: effort none, the toggle, or the BUG-439 Claude forms
  #   14. A per-turn raw thinking_config is refused; levels are chosen with the thinking dialog or /thinking
  #
  # EXAMPLES:
  #   1. openai/gpt-5.5 on the openai provider at xhigh sends reasoning_effort xhigh
  #   2. gpt-5.5 on the codex provider at high sends reasoning {effort: high, summary: auto}
  #   3. claude-opus-4-6 at effort max sends adaptive thinking and output_config.effort max; at a 8000-token budget it sends enabled thinking with budget_tokens 8000
  #   4. claude-sonnet-4-6 served by github-copilot at high sends reasoning_effort high, not a Claude thinking object
  #   5. gemini-3.1-pro-preview at low sends thinkingLevel low; gemini-2.5-pro at a 4096-token budget sends thinkingBudget 4096
  #   6. A Rhai provider at high receives thinking_config {level: high, capability}
  #   7. claude-opus-4-6 at xhigh is refused naming the model and listing low, medium, high, max
  #   8. claude-haiku-4-5 (budget only) at High is refused naming its token budget
  #   9. zai glm-4.6 (toggle only) at High is refused; at Off it sends thinking {type: disabled}
  #   10. A local Strata profile model at High sends reasoning_effort high with both kwargs true; at Off reasoning_effort none with both false
  #   11. openai/gpt-5.5 at Off sends reasoning_effort none; openai/gpt-5 (no none) at Off is refused
  #   12. zai glm-5.3 at High (it advertises high) is refused: no effort form is known for zai
  #
  # QUESTIONS (ANSWERED, product owner, 2026-11-04):
  #   Q: Models with no reasoning capability: refuse, or send unchecked?
  #   A: Send unchecked until MODEL-016 declares capabilities; Strata keeps reasoning_effort.
  #
  #   Q: Budget-only and toggle-only models while the session holds only Off/Low/Medium/High?
  #   A: NO LEGACY - ONLY NEW SYSTEM: strict checking now; the budget tables and the Z.AI toggle mapping are removed.
  #
  #   Q: local-openai chat_template_kwargs?
  #   A: Send thinking and enable_thinking, both (level != none), alongside reasoning_effort, without a declaration.
  #
  # ========================================
  Background: User Story
    As a fspec user who has chosen a thinking level
    I want to have that level reach each provider in the wire form that provider and model take
    So that the model thinks at the level I chose, and a level a provider cannot carry is refused instead of being dropped or remapped

  Scenario: The serving provider decides the wire form, not the model id
    Given claude-sonnet-4-6 is served by the github-copilot provider and advertises efforts "low, medium, high"
    When the session's level is High
    Then the request carries reasoning_effort "high"
    And it carries no Claude thinking object

  Scenario: OpenAI Chat Completions receives the effort name unchanged
    Given openai/gpt-5.5 on the openai provider advertises efforts "none, low, medium, high, xhigh"
    When the chosen level is the effort "xhigh"
    Then the request carries reasoning_effort "xhigh"

  Scenario: Codex receives the effort with an automatic summary
    Given gpt-5.5 is served by the codex provider and advertises efforts "none, low, medium, high, xhigh"
    When the session's level is High
    Then the request carries reasoning effort "high" with summary "auto"

  Scenario: Anthropic efforts become adaptive thinking and budgets become extended thinking
    Given anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max" and a token budget of at least 1024
    When the chosen level is the effort "max"
    Then the request carries adaptive thinking with summarized display and output effort "max"
    When the chosen level is a budget of 8000 tokens
    Then the request carries enabled thinking with a budget of 8000 tokens

  Scenario: Gemini efforts become a thinking level and budgets a thinking budget
    Given google/gemini-3.1-pro-preview advertises efforts "low, medium, high" and google/gemini-2.5-pro a token budget from 128 to 32768
    When gemini-3.1-pro-preview is at the effort "low" and gemini-2.5-pro at a budget of 4096 tokens
    Then gemini-3.1-pro-preview's request carries thinkingLevel "low" with thoughts included
    And gemini-2.5-pro's request carries thinkingBudget 4096 with thoughts included

  Scenario: A local server gets its engine's form for the declared levels
    Given a model on a local Strata profile that declares efforts "none, low, medium, high" and no template toggle (MODEL-016)
    When the session's level is High
    Then the request carries reasoning_effort "high" and no chat_template_kwargs
    When the session's level is Off
    Then the request carries reasoning_effort "none" and no chat_template_kwargs

  Scenario: An effort the model does not advertise is refused, not remapped
    Given anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max"
    When the chosen level is the effort "xhigh"
    Then the turn is refused with an error naming anthropic/claude-opus-4-6 and listing "low, medium, high, max"

  Scenario: Budget-only and toggle-only models get no invented level
    Given anthropic/claude-haiku-4-5 advertises only a token budget of at least 1024 and zai/glm-4.6 only a toggle
    When the session's level is High
    Then the turn on claude-haiku-4-5 is refused with an error naming its token budget
    And the turn on glm-4.6 is refused
    When the session's level is Off
    Then glm-4.6's request carries thinking type "disabled"

  Scenario: Off is the model's own none effort or is refused
    Given openai/gpt-5.5 advertises efforts "none, low, medium, high, xhigh" and openai/gpt-5 efforts "minimal, low, medium, high"
    When the session's level is Off
    Then gpt-5.5's request carries reasoning_effort "none"
    And the turn on gpt-5 is refused saying thinking cannot be turned off on openai/gpt-5

  Scenario: A level no wire form can carry is refused naming the provider
    Given zai/glm-5.3 advertises efforts "low, high, max" and a custom provider uses the unknown facade "mystery"
    When the session's level is High
    Then the turn on glm-5.3 is refused saying no effort form is known for zai
    And the turn on the custom provider is refused naming the facade "mystery"

  Scenario: A per-turn raw thinking config is refused, not sent
    Given a session at High on openai/gpt-5.5
    When a turn arrives carrying a per-turn thinking config
    Then the turn is refused saying levels are chosen with the thinking dialog or /thinking
