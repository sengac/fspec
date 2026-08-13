@done
@RPC-338
@napi
@model-selection
@ts-parity
@wip
Feature: Model selector profile napi binding

  # Work unit: RPC-338. NapiProviderModels mirrors the new wire fields so the
  # JS surface stays in sync (rust/napi/src/models/napi_bindings.rs).
  Background: User Story
    As a codelet TUI user
    I want the napi provider binding to mirror profile and reachability fields
    So that the JS surface stays in sync with the Rust wire type

  @data-model
  Scenario: The napi binding mirrors the new provider fields
    Given the NapiProviderModels napi object binding
    Then it exposes a profile_name field of type Option<String>
    And it exposes an is_unreachable field of type bool
