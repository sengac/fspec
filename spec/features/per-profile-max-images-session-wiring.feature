@done
@PROV-144
@rust
@session
@providers
@high
Feature: Per-profile Max Images session wiring
  """
  The effective image budget for a session is resolved from the profile's
  maxImages and stored in the tool-layer session capability registry at
  session creation and on every mid-session model switch. The budget comes
  from the shared resolver resolve_profile_max_images (rust/sessions/src/
  model_resolution.rs) so no set-site can drift. It is registered alongside
  the existing vision entry at all five set-sites that call
  set_session_model_vision: the shared create helper
  (session_creation_helper.rs), the isolated-session create
  (session_manager.rs), the mid-session set_model (handle_impl.rs), and the
  two NAPI model-switch bindings (session_bindings.rs). It is cleared on
  session destroy alongside the vision entry. maxImages is a tool-layer
  concern only — it is NOT bridged into OPENAI_* env vars by
  apply_profile_env_vars.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want my session's image budget to follow the active model
    So that a mid-session switch to a no-vision profile disables image reads immediately

  # ========================================
  # SESSION WIRING (source-shape)
  # ========================================

  Scenario: Session creation and model-switch set-sites register the max-images budget
    Given the shared create, isolated create, mid-session set_model, and both NAPI model-switch set-sites exist
    When the max-images budget is resolved through the shared resolver
    Then each set-site registers the budget alongside the vision entry
    And the session destroy path clears the budget alongside the vision entry

  Scenario: The max-images resolver is defined once and not bridged into env vars
    Given the model resolution module defines resolve_profile_max_images
    When the shared resolver and the OPENAI_* env bridge are inspected
    Then the resolver is defined exactly once
    And the env bridge does not reference the max-images value
