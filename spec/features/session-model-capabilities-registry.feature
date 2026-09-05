@done
@model-selection
@bug-168
@tools
@session
@multimodal
@high
Feature: Session model capabilities registry consulted by the Read tool
  """
  Today the session's model vision capability (ModelDef.supports_vision /
  ModelInfo.has_capability(Capability::Vision)) is tracked only for TUI
  badges (header [V], model-selector [V]). Nothing in the tool layer consults
  it, so the Read tool defaults PDF reads to visual mode even when the active
  model cannot see images — the provider then replaces every page PNG with an
  [Image] placeholder (or drops it) and the context window is burned for
  nothing.

  BUG-168 design (this feature covers the plumbing half):
  - codelet-tools gains a session-scoped capability registry
  (model_capabilities.rs): set_session_model_vision / session_supports_vision /
  session_has_capabilities, following the codelet-tools session-registry
  patterns already used by tools (done.rs armed/goal registries, tool_pause).
  - codelet-sessions resolves supports_vision for a model selection via
  model_resolution::resolve_model_vision:
  * cloud/codex models  -> models.dev registry (selected_model_info)
  * custom providers    -> custom provider config ModelDef.supports_vision
  * profile models      -> profile customModels[].hasVision (fspec-config.json)
  * unresolvable        -> false (conservative: prefer not to burn images)
  - The registry is populated at session creation (all paths) and on every
  model switch (handle_impl set_model + NAPI session_set_model*), and cleared
  on session destroy.
  - Tool-layer consult rule (tested in pdf-read-pagination feature):
  entry absent -> historical visual default; entry present false -> text
  fallback with a one-line notice; entry present true -> visual.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Session model capabilities (at least supports_vision) MUST be resolvable in-process from the sessions layer: resolved at session creation and on every set_model, stored in a codelet-tools session-scoped registry that tools consult
  #   2. Cloud/codex models resolve vision from the models.dev registry; custom providers from their config ModelDef.supports_vision; profile models from the profile customModels hasVision flag; anything unresolvable resolves to false
  #   3. Every session-creation path and every mid-session model switch MUST populate (or update) the registry entry; session destroy clears it
  #   4. An absent registry entry (unknown session) MUST preserve the historical visual default — only a PRESENT false entry triggers the text fallback
  #
  # EXAMPLES:
  #   1. Session created with anthropic/claude-opus (registry: image input) -> registry entry true -> PDF default stays visual
  #   2. Session created with a custom provider whose ModelDef.supports_vision=false -> registry entry false -> PDF default becomes text with a notice
  #   3. Profile model with customModels[{id, hasVision: true}] -> registry entry true -> PDF default stays visual
  #   4. Mid-session switch to a non-vision model -> registry entry updated to false -> next PDF read defaults to text
  #
  # ========================================
  Background: User Story
    As an agent session whose active model cannot see images
    I want PDF reads to default to a mode that actually uses the model
    So that context is not burned on page images that will be dropped

  Scenario: Registry entry set at session creation for a vision model
    Given a session is created with a cloud model the registry marks as image-capable
    When the session creation path resolves the model capabilities
    Then the tool-layer registry reports the session model supports vision

  Scenario: Registry entry set at session creation for a non-vision custom model
    Given a session is created with a custom provider model whose config sets supports_vision=false
    When the session creation path resolves the model capabilities
    Then the tool-layer registry reports the session model does not support vision

  Scenario: Profile model vision flag flows from fspec-config.json
    Given an openai profile custom model declares hasVision=true in fspec-config.json
    When the session is created with that profile model
    Then the tool-layer registry reports the session model supports vision

  Scenario: Model switch updates the registry entry
    Given a session whose registry entry currently reports vision support
    When the session switches mid-session to a model that cannot see images
    Then the registry entry is updated to report no vision support

  Scenario: Unresolvable models resolve conservatively to no vision
    Given a provider/model pair that cannot be resolved against the registry, custom config, or profiles
    When the vision capability is resolved
    Then it resolves to false rather than guessing true
