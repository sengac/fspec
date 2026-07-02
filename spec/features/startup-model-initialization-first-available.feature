@done
@providers
@model-selection
@tui
@rust
@PROV-120
Feature: Restore TS-parity first-available model initialization removed by PROV-101
  """
  TS reference: src/tui/services/modelInitializationService.ts -> initializeModels() (resolution chain), selectDefaultModel() (first-available), restorePersistedModel() (persisted match), loadPersistedModelString() (reads tui.lastUsedModel)
  Rust side: a startup init step must run before the bootstrap create_session (combined mode bootstrap in codelet rpc-server / fspec-tui app start). It should build sections via the existing list_providers/profile_sections + cloud_model_entries path, apply persisted->first-available, then call set_default_model.
  PROV-101's removals to KEEP: no anthropic .unwrap_or_else in handle_impl create_session; resolve_unambiguous_provider stays (ambiguous multi-cred = Err). What changes: a default IS proactively set at startup from reachable sections, so the decline path becomes the genuine zero-models edge case rather than the normal launch path.
  Persistence store reconciliation: TS reads/writes tui.lastUsedModel in fspec-config.json; Rust PROV-119 persists to default-model.json:model. Decide whether to read tui.lastUsedModel for restore parity or keep default-model.json. (Open question Q0)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. At startup, model resolution order is: restore persisted model (tui.lastUsedModel) if it matches a credentialed section, ELSE first-available reachable model with >=1 model
  #   2. First-available means the first section (in order profiles -> custom -> cloud) that has at least one model; take its first model
  #   3. The resolved model is committed via set_default_model BEFORE the bootstrap create_session, so create_session sees a populated default on a normal launch
  #   4. Sections that are unreachable AND have zero models are excluded from selection (cannot become the default)
  #   5. PRESERVE PROV-101: no hardcoded anthropic/claude substitution is ever used as a fallback
  #   6. PRESERVE PROV-101: provider resolution still refuses to silently pick on an ambiguous multi-provider credential state; first-available is over reachable model-bearing sections, NOT a provider-priority chain
  #   7. When there are genuinely zero reachable model-bearing sections, no default is set and create_session declines (the true edge case PROV-101 intended) -- not the normal-launch path
  #   8. tui.lastUsedModel in ~/.fspec/fspec-config.json (user scope) is the single source of truth in TS. READ: loadPersistedModelString() -> loadConfig() -> config.tui.lastUsedModel (modelInitializationService.ts:73). WRITE: selectModel() -> writeConfig('user', { tui.lastUsedModel }) (modelSelectionService.ts:179-186), only when the session update succeeded. For parity Rust MUST read/restore from fspec-config.json tui.lastUsedModel; PROV-119's default-model.json diverges and should be reconciled to fspec-config.json as source of truth.
  #   9. TS runs initializeModels() on AgentView mount (TUI startup), once, idempotent via modelsInitialized guard, BEFORE any session creation. Session creation (AgentView.tsx:3772 createSession) happens later and RECEIVES the already-resolved model; sessionService.createSession does NOT initialize models. There is NO headless/CLI model init — it is purely a TUI concern (commands/reverse.ts 'session' is unrelated ACDD state). Therefore Rust init belongs in the fspec-tui app startup/mount path, running before the bootstrap create_session; it does NOT belong in a headless rpc-server path.
  #   10. model-selector-no-auto-select.feature (PROV-101) stays UNCHANGED and is NOT contradicted. In TS the selector seeds its cursor from currentModel: ModelSelectorScreen.tsx:94-119 auto-expands the section and highlights the row matching currentModel.apiModelId. If there is NO current model it returns early (no highlight, cursor on first header, modelIdx=-1) — exactly PROV-101's no-auto-select. PROV-120 simply means currentModel is now usually non-null at startup, so the selector legitimately highlights the resolved default. Restoring a default DOES highlight the selector cursor (because currentModel is set), but the 'nothing selected when no current model' invariant is preserved.
  #   11. SCOPE (folded in): reconcile the persistence store onto fspec-config.json tui.lastUsedModel as source of truth (TS parity). PROV-119's default-model.json is itself a non-parity divergence; PROV-120 reads/restores and writes tui.lastUsedModel. Migration/back-compat: if default-model.json exists and fspec-config.json has no tui.lastUsedModel, read it once for continuity, but new writes go to fspec-config.json.
  #
  # EXAMPLES:
  #   1. Fresh launch, no persisted model, one reachable cloud provider with models: startup selects that provider's first model and create_session succeeds without opening /model
  #   2. Launch with a persisted tui.lastUsedModel that still matches a credentialed section: startup restores exactly that model (not first-available)
  #   3. Launch with a persisted model whose provider no longer has credentials (or model gone): startup falls back to first-available reachable model
  #   4. Two local profiles unreachable (zero models) but a reachable cloud provider exists: unreachable profiles are skipped, cloud model is selected as first-available
  #   5. Order check: a reachable local profile with models AND a reachable cloud provider both exist; first-available picks the profile model because profiles precede cloud
  #   6. Genuinely zero reachable model-bearing sections (all profiles down, no cloud creds): no default committed, create_session declines with empty SessionId (PROV-101 edge case preserved)
  #   7. Anthropic is NOT credentialed and is NOT the only reachable provider: startup never substitutes anthropic/claude as a hardcoded default
  #
  # QUESTIONS (ANSWERED):
  #   Q: Persistence store — should Rust read/restore from tui.lastUsedModel in fspec-config.json for true TS parity, or keep PROV-119's default-model.json (and which is source of truth)?
  #   A: tui.lastUsedModel in ~/.fspec/fspec-config.json (user scope) is the single source of truth in TS. READ: loadPersistedModelString() -> loadConfig() -> config.tui.lastUsedModel (modelInitializationService.ts:73). WRITE: selectModel() -> writeConfig('user', { tui.lastUsedModel }) (modelSelectionService.ts:179-186), only when the session update succeeded. For parity Rust MUST read/restore from fspec-config.json tui.lastUsedModel; PROV-119's default-model.json diverges and should be reconciled to fspec-config.json as source of truth.
  #
  #   Q: Where exactly should the startup init run — in the Rust combined-mode bootstrap (rpc-server) before bootstrap create_session, or in the fspec-tui app mount path mirroring TS AgentView? Does it need to also work for headless/CLI session creation?
  #   A: TS runs initializeModels() on AgentView mount (TUI startup), once, idempotent via modelsInitialized guard, BEFORE any session creation. Session creation (AgentView.tsx:3772 createSession) happens later and RECEIVES the already-resolved model; sessionService.createSession does NOT initialize models. There is NO headless/CLI model init — it is purely a TUI concern (commands/reverse.ts 'session' is unrelated ACDD state). Therefore Rust init belongs in the fspec-tui app startup/mount path, running before the bootstrap create_session; it does NOT belong in a headless rpc-server path.
  #
  #   Q: Should the model-selector-no-auto-select.feature (PROV-101 #4/#5) stay unchanged (selector still shows nothing pre-selected) while startup init sets the default separately, or does restoring a default also mean the selector cursor highlights it?
  #   A: model-selector-no-auto-select.feature (PROV-101) stays UNCHANGED and is NOT contradicted. In TS the selector seeds its cursor from currentModel: ModelSelectorScreen.tsx:94-119 auto-expands the section and highlights the row matching currentModel.apiModelId. If there is NO current model it returns early (no highlight, cursor on first header, modelIdx=-1) — exactly PROV-101's no-auto-select. PROV-120 simply means currentModel is now usually non-null at startup, so the selector legitimately highlights the resolved default. Restoring a default DOES highlight the selector cursor (because currentModel is set), but the 'nothing selected when no current model' invariant is preserved.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user launching the app
    I want to have a working model selected automatically at startup when I have any reachable credentialed model (persisted choice first, otherwise the first available reachable model)
    So that I can start a session immediately without being forced to manually open /model on every fresh launch, matching the TypeScript reference behavior

  # EXAMPLE 1
  Scenario: Fresh launch with no persisted model selects the first available reachable cloud model
    Given no persisted model is recorded
    And exactly one cloud provider section is reachable and credentialed with at least one model
    And no other reachable model-bearing sections exist
    When startup model initialization runs
    Then the default model resolves to that cloud provider's first model
    And the resolved model is committed before the bootstrap session is created
    And the bootstrap create_session succeeds without opening the model selector

  # EXAMPLE 2
  Scenario: Persisted model that still matches a credentialed section is restored exactly
    Given a persisted model is recorded
    And the persisted model matches a reachable credentialed section that still contains that model
    And another reachable model-bearing section exists earlier in build order
    When startup model initialization runs
    Then the default model resolves to the persisted model
    And the default model is not the first-available model from the earlier section
    And the bootstrap create_session succeeds without opening the model selector

  # EXAMPLE 3
  Scenario: Persisted model whose provider lost credentials falls back to first-available
    Given a persisted model is recorded
    And the persisted model's provider no longer has credentials or the model no longer exists
    And a different reachable credentialed section with at least one model exists
    When startup model initialization runs
    Then the persisted model is not restored
    And the default model resolves to the first-available reachable model
    And the bootstrap create_session succeeds without opening the model selector

  # EXAMPLE 4
  Scenario: Unreachable zero-model local profiles are skipped in favour of a reachable cloud model
    Given no persisted model is recorded
    And two local profile sections are unreachable and contain zero models
    And one cloud provider section is reachable and credentialed with at least one model
    When startup model initialization runs
    Then the unreachable zero-model profile sections are excluded from selection
    And the default model resolves to the cloud provider's first model
    And the bootstrap create_session succeeds without opening the model selector

  # EXAMPLE 5
  Scenario: First-available honours build order so a reachable profile precedes cloud
    Given no persisted model is recorded
    And a local profile section is reachable and credentialed with at least one model
    And a cloud provider section is reachable and credentialed with at least one model
    When startup model initialization runs
    Then the default model resolves to the profile section's first model
    And the default model is not the cloud provider's model
    And the bootstrap create_session succeeds without opening the model selector

  # EXAMPLE 6 (also Rule 7 — genuine zero-models edge case)
  Scenario: Genuinely zero reachable model-bearing sections leaves no default and declines
    Given no persisted model is recorded
    And all local profile sections are unreachable with zero models
    And no cloud provider section is credentialed
    And no reachable model-bearing section exists
    When startup model initialization runs
    Then no default model is committed
    And the bootstrap create_session declines with an empty SessionId

  # EXAMPLE 7 (also Rule 5 — preserve PROV-101: no hardcoded anthropic substitution)
  Scenario: Anthropic is never substituted as a hardcoded default when not credentialed
    Given no persisted model is recorded
    And anthropic is not credentialed
    And a different reachable credentialed provider section with at least one model exists
    When startup model initialization runs
    Then the default model resolves to the reachable credentialed provider's first model
    And no anthropic or claude model is substituted as a hardcoded default

  # RULE 6 — preserve PROV-101: no silent pick on ambiguous multi-provider credential state
  Scenario: Ambiguous multi-provider credential state is not silently resolved by provider priority
    Given no persisted model is recorded
    And multiple providers are credentialed in an ambiguous state with no reachable model-bearing section
    When startup model initialization runs
    Then no default model is committed by provider-priority preference
    And the bootstrap create_session declines with an empty SessionId
