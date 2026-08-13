@done
@facade
@spec-alignment
@rust
@documentation
@providers
@PROV-068
Feature: Reconcile RhaiToolFacadeAdapter spec with implementation
  """
  Reconcile via docs/spec updates (Option A) rather than introducing a ToolDyn wrapper, because Rhai tools have dynamic names incompatible with rig::Tool's const NAME requirement
  A Rust integration test in rust/providers/tests/ pins the adapter's getters-only contract (public methods, types, values) so future regressions are caught
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. RhaiToolFacadeAdapter is a thin getters-only adapter (not a rig::Tool impl), because rig::Tool requires a const NAME that is incompatible with runtime-defined Rhai tool names
  #   2. The adapter exposes name(), parameters_schema(), maps_to(), def(), loader(), and config() getters so downstream code can bridge RhaiToolDef into rig-compatible request builders
  #   3. PROV-066 Rule 0, architecture note [0], and PROV-061 architecture note [1] must describe the getters-only adapter design instead of claiming rig::Tool implementation
  #   4. The PROV-066 feature file (custom-provider-rhai-scriptable-tool-facades.feature) background doc string and scenario step text must not reference rig::Tool semantics
  #   5. tool_facade.rs module docs already correctly describe the getters-only design and remain the canonical source of truth
  #
  # EXAMPLES:
  #   1. RhaiToolFacadeAdapter.name() returns the name string from its underlying RhaiToolDef (e.g. 'my_read')
  #   2. RhaiToolFacadeAdapter.parameters_schema() returns a &serde_json::Value exactly matching the Rhai-supplied JSON schema
  #   3. RhaiToolFacadeAdapter.maps_to() returns the routing identifier string (e.g. 'file:read') so callers can dispatch to the correct internal tool
  #   4. After reconciliation, PROV-066 Rule 0 reads 'RhaiToolFacadeAdapter is a getters-only adapter (not a rig::Tool impl)...'
  #   5. After reconciliation, PROV-066 Example 8 reads '...adapter.name() returns the tool name from the RhaiToolDef and .parameters_schema() returns the JSON schema...' (no rig::Tool reference)
  #
  # ========================================
  Background: User Story
    As a custom provider developer reading the PROV-066 spec
    I want to have the rules and architecture notes accurately describe RhaiToolFacadeAdapter as a getters-only adapter (not a full rig::Tool impl)
    So that I can trust the specification when implementing or reviewing Rhai-scriptable tool facades

  Scenario: Adapter name getter returns the Rhai-supplied tool name
    Given a RhaiToolDef with name "my_read", description "read a file", and a parameters schema, and maps_to "file:read"
    When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    Then RhaiToolFacadeAdapter.name() returns "my_read"

  Scenario: Adapter parameters_schema getter returns the Rhai-supplied schema
    Given a RhaiToolDef with parameters schema {"type":"object","properties":{"path":{"type":"string"}}}
    When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    Then RhaiToolFacadeAdapter.parameters_schema() returns a &serde_json::Value equal to that schema

  Scenario: Adapter maps_to getter exposes the routing identifier
    Given a RhaiToolDef with maps_to "file:read"
    When I build a RhaiToolFacadeAdapter from that RhaiToolDef
    Then RhaiToolFacadeAdapter.maps_to() returns the string "file:read"

  Scenario: PROV-066 Rule 0 describes getters-only adapter design
    Given the PROV-066 work unit rules have been reconciled with the implementation
    When I read PROV-066 rule with stable id 0
    Then the rule text states that RhaiToolFacadeAdapter is a getters-only adapter and does not claim it implements rig::Tool

  Scenario: PROV-066 architecture note 0 describes getters-only adapter design
    Given the PROV-066 architecture notes have been reconciled with the implementation
    When I read PROV-066 architecture note with stable id 0
    Then the note text describes RhaiToolFacadeAdapter as a getters-only adapter and does not claim it implements rig::Tool

  Scenario: PROV-061 architecture note 1 describes getters-only adapter design
    Given the PROV-061 architecture notes have been reconciled with the implementation
    When I read PROV-061 architecture note with stable id 1
    Then the note text describes RhaiToolFacadeAdapter as a getters-only adapter and does not claim it implements rig::Tool

  Scenario: PROV-066 feature file no longer references rig::Tool semantics
    Given the custom-provider-rhai-scriptable-tool-facades feature file has been reconciled
    When I read the feature file background doc string and scenario step text
    Then no positive rig::Tool implementation claim remains in the background doc string or any scenario steps
