@done
@context-window
@napi
@CTX-006
Feature: Rust-Authoritative Context Window — Single Source of Truth
  """
  Approach: Add model_limits (context_window, max_output_tokens) to SessionModel in Rust session state. This is the lightest-touch change — no new NAPI query function needed, just extend the existing snapshot that useRustSessionState already polls
  Rust side: ProviderManager already has context_window() and max_output_tokens() methods. After select_model()/set_model_direct()/override_model_limits(), emit these values into SessionModel state so the NAPI snapshot includes them
  TypeScript side: AgentView.tsx rustModelInfo useMemo switches from findModelInProviders(providerSections) lookup to reading rustSnapshot.model.contextWindow directly when available. This eliminates the models.dev dependency for the active session
  NAPI boundary: SessionModel struct (or a new ModelLimits sub-struct) gains context_window: Option<u32> and max_output_tokens: Option<u32> fields. The Option handles the 'no model selected yet' state
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After model selection, the TUI must display context_window from Rust's ProviderManager resolution, not from models.dev data in providerSections
  #   2. Rust session state must expose resolved context_window and max_output_tokens as part of the session model metadata
  #   3. The model selector list may continue showing models.dev context values for browsing — only the active session badge must use Rust-resolved values
  #   4. If the resolved context_window is not yet available (before model selection completes), the TUI should fall back to models.dev data or show 0
  #   5. Session resume must restore context_window from Rust session metadata, not re-derive from models.dev
  #   6. The ContextFillUpdate event already sends context_window from Rust — the SessionHeader badge and fill percentage must both ultimately derive from the same Rust authority
  #
  # EXAMPLES:
  #   1. User selects Claude Opus 4.6 (models.dev reports 1M context) but Rust resolves to 200k → SessionHeader badge shows [200k], not [1M]
  #   2. User selects a standard Claude Sonnet model with 200k in both models.dev and Rust → badge shows [200k] (no change, values agree)
  #   3. User selects Gemini model with 1M in both models.dev and Rust → badge shows [1M] (values agree)
  #   4. User creates a custom profile model with context_window=32000 → Rust stores 32000 → badge shows [32k]
  #   5. User resumes an existing session → context_window is restored from Rust session state without re-querying models.dev
  #   6. Model not found in models.dev (deleted or renamed) but session has Rust-resolved context_window → badge still shows correct value
  #   7. DeepSearch sub-agent inherits parent's Rust-resolved context_window → compaction uses inherited value
  #   8. Context fill percentage and SessionHeader badge both derive from the same Rust context_window — they can never disagree
  #
  # ========================================
  Background: User Story
    As a developer
    I want to see the correct context window size for the active model in the session header and fill percentage
    So that the displayed value matches what the compaction engine actually uses, preventing misleading UI

  @core
  Scenario: Display Rust-resolved context window when models.dev disagrees
    Given a Claude model where models.dev reports 1M context window
    And Rust ProviderManager resolves the context window to 200000 tokens
    When the model is selected for the active session
    Then the SessionHeader badge should display "[200k]"
    And the displayed context window should equal 200000

  @core
  Scenario: Display consistent context window when sources agree
    Given a Claude Sonnet model with 200000 context window in models.dev
    And Rust ProviderManager resolves the context window to 200000 tokens
    When the model is selected for the active session
    Then the SessionHeader badge should display "[200k]"

  @core
  Scenario: Display context window for Gemini model
    Given a Gemini model with 1000000 context window in models.dev
    And Rust ProviderManager resolves the context window to 1000000 tokens
    When the model is selected for the active session
    Then the SessionHeader badge should display "[1M]"

  @core
  Scenario: Display context window for custom profile model
    Given a custom profile model with context_window configured as 32000
    When the profile model is selected for the active session
    Then Rust session state should contain context_window of 32000
    And the SessionHeader badge should display "[32k]"

  @core
  Scenario: Restore context window from Rust state on session resume
    Given a session was previously active with a model resolved to 200000 context window
    When the session is resumed
    Then the context window should be restored from Rust session state
    And the SessionHeader badge should display "[200k]"
    And models.dev should not be re-queried for the context window

  @edge-case
  Scenario: Display context window when model is missing from models.dev
    Given a session with a Rust-resolved context window of 200000
    And the model is no longer present in the models.dev catalog
    When the SessionHeader renders
    Then the badge should display "[200k]" from Rust state
    And the display should not fall back to 0

  @integration
  Scenario: Sub-agent inherits Rust-resolved context window
    Given a parent session with Rust-resolved context window of 200000
    When a DeepSearch sub-agent is spawned from the parent session
    Then the sub-agent should inherit the context window of 200000
    And the sub-agent compaction should use the inherited value

  @core
  Scenario: SessionHeader badge and context fill derive from same source
    Given a session with Rust-resolved context window of 200000
    And the session has consumed 100000 tokens
    When the context fill percentage is calculated
    Then the fill percentage should use 200000 as the context window
    And the SessionHeader badge should display "[200k]"
    And both values should derive from the same Rust ProviderManager authority

  @edge-case
  Scenario: Fallback to models.dev before model selection completes
    Given a session where no model has been selected yet
    When the SessionHeader renders
    Then the context window should fall back to 0 or models.dev data
    And no error should occur from missing Rust-resolved values

  @napi
  Scenario: Rust session state exposes model limits via NAPI
    Given a session with an active model
    When the Rust session state snapshot is queried
    Then the snapshot should include context_window from ProviderManager resolution
    And the snapshot should include max_output_tokens from ProviderManager resolution
    And these values should be Optional to handle the no-model-selected state
