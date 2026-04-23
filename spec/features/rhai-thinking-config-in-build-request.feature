@done
@adaptive-thinking
@configuration
@provider-abstraction
@rust
@validator
@providers
@PROV-090
Feature: Pass thinking_config into Rhai build_request as an input field
  """
  request_bridge is a pure conversion layer — no HTTP, no I/O — so tests operate on Dynamic return values directly
  json_value_to_dynamic already handles null → Rhai unit, so bridging Option<&serde_json::Value> by matching None to unit is consistent
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. request_to_rhai accepts an Option<&serde_json::Value> thinking_config parameter after messages and tools
  #   2. When thinking_config is Some(value), the resulting request map has a thinking_config key bridged from the JSON value
  #   3. When thinking_config is None, the request map's thinking_config key is bridged as Rhai unit (())
  #   4. invoke_build_request and invoke_build_stream_request on RhaiCustomProvider accept and forward thinking_config
  #   5. CustomProvider::create_rig_agent accepts a thinking_config parameter matching ClaudeProvider::create_rig_agent's signature shape
  #
  # EXAMPLES:
  #   1. request_to_rhai called with Some({"type":"enabled","budget_tokens":10000}) produces a map whose thinking_config.type is "enabled" and thinking_config.budget_tokens is 10000
  #   2. request_to_rhai called with None produces a map whose thinking_config value is Rhai unit
  #   3. A Rhai build_request script reads request.thinking_config and when present uses its fields to populate the outgoing request body's thinking key
  #   4. CustomProvider::create_rig_agent(project_root, name, model_alias, session_id, preamble, Some(cfg)) compiles and wires the backend without the thinking_config being passed further (wiring parity only for this work unit)
  #
  # ========================================
  Background: User Story
    As a Rhai custom provider author
    I want to read thinking_config from the build_request input map
    So that I can emit adaptive thinking config in the request body for reasoning-capable models

  Scenario: request_to_rhai bridges Some thinking_config into the request map
    Given a messages slice and a tools slice and a JSON value {"type":"enabled","budget_tokens":10000}
    When I call request_to_rhai with Some(thinking_config)
    Then the returned Dynamic is a map containing messages tools and thinking_config
    And the thinking_config entry is a map whose type field is "enabled" and whose budget_tokens field is 10000

  Scenario: request_to_rhai bridges None thinking_config as Rhai unit
    Given a messages slice and a tools slice
    When I call request_to_rhai with None for thinking_config
    Then the returned Dynamic is a map containing a thinking_config key whose value is Rhai unit

  Scenario: A Rhai build_request script uses thinking_config to populate the outgoing request body
    Given a RhaiCustomProvider whose build_request script copies request.thinking_config.budget_tokens into the body as thinking.budget_tokens when thinking_config is present
    When I invoke invoke_build_request with Some thinking_config {"type":"enabled","budget_tokens":8192}
    Then the resulting JSON body contains thinking.budget_tokens equal to 8192

  Scenario: CustomProvider::create_rig_agent accepts a thinking_config parameter
    Given a valid custom provider config discoverable on disk
    When I call CustomProvider::create_rig_agent passing Some(thinking_config) as the new parameter
    Then the call returns Ok(CustomRigAgent) with the same wiring invariants as before
