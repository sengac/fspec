@done
@model-selection
@rpc
@RPC-347
Feature: Backend custom-model RPC + NAPI surface (add/update/delete_custom_model)
  """
  Layer wiring (mirror set_session_model): (1) rpc-types/src/lib.rs add CustomModelDefinition; (2) rpc/src/lib.rs FspecService trait + FspecServiceImpl delegation; (3) core/src/session_manager_handle.rs SessionManagerHandle trait default no-op methods (~line 137/146); (4) sessions/src/handle_impl.rs concrete override calling profile_sections::save_custom_model/delete_custom_model; (5) fspec-tui/src/transport/{mod,embedded,websocket}.rs FspecBackend; (6) napi/src bindings; (7) fspec-tui/src/components/mod.rs Action enum variants
  Builds directly on RPC-346 public fns: save_custom_model(provider_id, profile_name, &CustomModelDef, original_model_id: Option<&str>) and delete_custom_model(provider_id, profile_name, model_id). add=save(None), update=save(Some(old_id)). Tests stay offline using the path-injectable *_at helpers + temp config files (no env mutation, no network)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The FspecService RPC trait exposes three methods: add_custom_model, update_custom_model, and delete_custom_model (matching the TS save/delete service split: add = save with no original id, update = save replacing an original id)
  #   2. A transport-portable CustomModelDefinition wire type carries id (required) plus optional displayName, facade, contextWindow, maxOutputTokens, compactionThreshold, reasoning, hasVision; it maps 1:1 to codelet/sessions CustomModelDef
  #   3. add_custom_model delegates to SessionManagerHandle and ultimately profile_sections::save_custom_model with original_model_id = None (append); update_custom_model passes original_model_id = Some(old_id) (replace in place); delete_custom_model calls profile_sections::delete_custom_model
  #   4. SessionManagerHandle gains the three methods with default no-op implementations returning Ok(()); FspecServiceImpl delegates through the optional handle and returns Ok (silent no-op) when no handle is attached, mirroring set_session_model's idempotent contract
  #   5. All three transports (FspecBackend default trait, embedded, and websocket) expose the three methods with identical behavior — cross-transport parity, the same pattern as the existing RPC-0xx surface
  #   6. NAPI bindings expose add/update/delete_custom_model to the JS/TS layer, accepting a CustomModelDefinition-shaped object and the profile/provider identifiers
  #   7. The fspec-tui Action enum gains AddCustomModel / EditCustomModel / DeleteCustomModel variants carrying the profile name and definition/model-id payload; they are inert wire surface here (RPC-344 wires the a/e/d keybinds and dispatch)
  #   8. Provider/profile guards are inherited from RPC-346: only the openai provider is supported and a missing profile is a no-op success (idempotent); the RPC layer adds no new validation beyond delegation
  #
  # EXAMPLES:
  #   1. add_custom_model with a full CustomModelDefinition on an existing openai profile appends the entry; it is then visible in list_providers and on disk in fspec-config.json
  #   2. update_custom_model with original_model_id replaces the matching entry in place (preserving array position), changing its displayName/facade/limits
  #   3. delete_custom_model removes the entry by id; deleting the last custom model drops the customModels key entirely (delegated RPC-346 behavior)
  #   4. Calling add_custom_model over the embedded transport and over the websocket transport produces identical config results (cross-transport parity)
  #   5. Calling any of the three methods on a FspecServiceImpl with no SessionManagerHandle attached returns Ok(()) and makes no change (silent no-op)
  #   6. A CustomModelDefinition round-trips through the NAPI boundary (JS object in → same fields persisted) for add_custom_model
  #   7. delete_custom_model on a non-existent profile or a non-openai provider returns Ok(()) and leaves config untouched (idempotent no-op)
  #
  # ========================================
  Background: User Story
    As a model-selector UI (and any RPC client)
    I want to add, update, and delete custom models on a local-server profile through the RPC/NAPI surface
    So that the custom-model write path built in RPC-346 is reachable over every transport, unblocking the RPC-344 keybinds/views

  Scenario: add_custom_model appends a new definition to an existing profile
    Given an openai profile "work-vllm" exists with no custom models
    When a client calls add_custom_model for "work-vllm" with a full CustomModelDefinition for "my-model"
    Then the call returns Ok
    And the profile's customModels contains an entry with id "my-model" and the supplied fields

  Scenario: update_custom_model replaces an existing definition in place
    Given an openai profile "work-vllm" exists with custom models "alpha" then "beta"
    When a client calls update_custom_model for "work-vllm" with original id "alpha" and a new definition id "alpha2"
    Then the call returns Ok
    And the customModels entry formerly "alpha" is now "alpha2" at the same array position
    And the entry "beta" is unchanged

  Scenario: delete_custom_model removes an entry and drops the empty key
    Given an openai profile "work-vllm" exists with a single custom model "only-model"
    When a client calls delete_custom_model for "work-vllm" with id "only-model"
    Then the call returns Ok
    And the profile no longer has a customModels key

  Scenario: delete_custom_model on a missing profile or non-openai provider is an idempotent no-op
    Given a config without a profile named "does-not-exist"
    When a client calls delete_custom_model for provider "openai" profile "does-not-exist"
    And a client calls delete_custom_model for a non-openai provider
    Then each call returns Ok
    And the configuration is left untouched

  Scenario: add_custom_model and update_custom_model on a non-openai provider return an error
    Given a config with an openai profile "work-vllm"
    When a client calls add_custom_model for a non-openai provider
    And a client calls update_custom_model for a non-openai provider
    Then each call returns Err mentioning the OpenAI-only constraint
    And the configuration is left untouched
