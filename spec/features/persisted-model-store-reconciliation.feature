@done
@config-management
@persistence
@providers
@rust
@PROV-120
Feature: Persisted Model Store Reconciliation

  """
  PROV-120 persistence reconciliation. Source of truth = tui.lastUsedModel in ~/.fspec/fspec-config.json (TS parity: modelInitializationService.ts loadPersistedModelString / modelSelectionService.ts writeConfig). Rust reader added in codelet/sessions/src/last_used_model_persistence.rs (path-injectable _from(&Path) core + env-resolved convenience). Legacy default-model.json (PROV-119) is read once for back-compat when fspec-config.json has no tui.lastUsedModel; new writes go to fspec-config.json.
  """

  Background: User Story
    As a fspec TUI user
    I want my last-used model read from and written to fspec-config.json (tui.lastUsedModel), with a one-time back-compat read of the legacy default-model.json
    So that my model choice persists across restarts in the same store the TypeScript reference uses

  Scenario: Persisted model is restored from fspec-config.json tui.lastUsedModel
    Given the user fspec-config.json records tui.lastUsedModel
    And that model matches a reachable credentialed section that still contains it
    When startup model initialization runs
    Then the persisted model string is read from fspec-config.json tui.lastUsedModel
    And the default model resolves to that persisted model

  Scenario: Legacy default-model.json is read for continuity when fspec-config.json has no tui.lastUsedModel
    Given the user fspec-config.json records no tui.lastUsedModel
    And a legacy default-model.json records a model that matches a reachable credentialed section
    When startup model initialization runs
    Then the persisted model is read once from the legacy default-model.json
    And the default model resolves to that legacy model
