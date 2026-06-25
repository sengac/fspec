@done
@model-selection
@providers
@PROV-122
Feature: Model selection never persists tui.lastUsedModel to fspec-config.json (live-session writes nothing; no-session writes legacy default-model.json only)

  """
  Add writer to last_used_model_persistence.rs: save_persisted_model_string_to(user_dir, model) + env-resolved save_persisted_model_string(model). Reuse read_config_value/write_config_value/fspec_user_dir from profile_sections.rs (key-preserving, preserve_order serde). Mirror save_custom_model_at read-merge-write pattern.
  Call sites: (1) handle_impl.rs::set_model after success near line 1050 using model=format!("{provider_id}/{model_id}"); (2) session_manager.rs::set_default_model line 227 (or FspecBackend wrapper) alongside existing save_default_model. Both best-effort with tracing::warn on failure.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selecting a model with an ACTIVE session persists tui.lastUsedModel to fspec-config.json after the in-memory switch succeeds
  #   2. Selecting a model with NO active session persists tui.lastUsedModel in addition to the legacy default-model.json (back-compat retained)
  #   3. The write is a key-preserving read-merge-write: only tui.lastUsedModel changes; all other config keys (providers, research, other tui.* fields) are left untouched
  #   4. An empty or whitespace-only model string is never persisted (PROV-101 invariant preserved); no hardcoded fallback model is ever written
  #   5. Persistence is best-effort and non-fatal: a write failure is logged (tracing::warn) but never panics, blocks, or fails the model switch
  #   6. The persisted string round-trips: a value written by the selection path is re-readable by load_persisted_model_string and yields the same provider/model (including profile-qualified ids)
  #
  # EXAMPLES:
  #   1. fspec-config.json missing entirely: writer creates it with {"tui":{"lastUsedModel":"anthropic/claude-opus-4-8"}} (fresh-install path)
  #   2. Config already has providers + research + tui.fallbackImageModel: after selecting anthropic/claude-opus-4-8, those keys are unchanged and only tui.lastUsedModel is updated
  #   3. Active session: user picks a different model; after the switch succeeds, fspec-config.json tui.lastUsedModel equals the newly selected provider/model; on restart that model is restored
  #   4. No active session: user picks anthropic/claude-opus-4-8; both tui.lastUsedModel (fspec-config.json) and default-model.json are written
  #   5. Empty model string passed to the persist path: fspec-config.json is left untouched (no tui.lastUsedModel written)
  #   6. Profile-qualified selection (e.g. openai:qwen/Qwen3-80B) round-trips: written to tui.lastUsedModel and read back by load_persisted_model_string as the same string
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to have my model selection saved to tui.lastUsedModel in fspec-config.json whether or not a session is active
    So that the model I picked is restored when I restart fspec

  Scenario: Persisting a model when no config file exists creates it
    Given the user directory has no fspec-config.json
    When the model "anthropic/claude-opus-4-8" is persisted as the last used model
    Then fspec-config.json is created
    And tui.lastUsedModel equals "anthropic/claude-opus-4-8"

  Scenario: Persisting a model preserves all other config keys
    Given fspec-config.json already has providers, research, and tui.fallbackImageModel keys
    When the model "anthropic/claude-opus-4-8" is persisted as the last used model
    Then tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    And the providers, research, and tui.fallbackImageModel keys are unchanged

  Scenario: Selecting a model with an active session persists the choice
    Given an active session is using model "anthropic/claude-sonnet-4"
    When the user selects model "anthropic/claude-opus-4-8" and the switch succeeds
    Then fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    And reloading the persisted model returns "anthropic/claude-opus-4-8"

  Scenario: Selecting a model with no active session writes both stores
    Given there is no active session
    When the user selects model "anthropic/claude-opus-4-8" as the default
    Then fspec-config.json tui.lastUsedModel equals "anthropic/claude-opus-4-8"
    And default-model.json records model "anthropic/claude-opus-4-8"

  Scenario: An empty model string is never persisted
    Given fspec-config.json has no tui.lastUsedModel key
    When an empty model string is passed to the persist path
    Then fspec-config.json is left untouched
    And no tui.lastUsedModel key is written

  Scenario: A profile-qualified model selection round-trips
    Given the user directory has no fspec-config.json
    When the model "openai:qwen/Qwen3-80B" is persisted as the last used model
    Then tui.lastUsedModel equals "openai:qwen/Qwen3-80B"
    And reloading the persisted model returns "openai:qwen/Qwen3-80B"
