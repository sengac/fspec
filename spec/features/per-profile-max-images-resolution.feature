@done
@PROV-144
@rust
@providers
@session
@high
Feature: Per-profile Max Images resolution
  """
  resolve_profile_max_images (rust/sessions/src/model_resolution.rs) is the
  single source of truth for a session's image budget. For a profile model
  selection (composite openai:<profile>/<model>) it reads the profile's
  stored maxImages from fspec-config.json via load_local_server_profiles():
  explicit n => Some(n) (including the Some(0) no-vision sentinel), absent
  key => None. Non-profile selections (cloud / custom / codex) resolve to
  None so the tool layer applies the uniform default of 4. The resolution
  follows the model on every re-resolution (mid-session switches).
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want the session layer to resolve my profile's Max Images value
    So that profile, absent, and non-profile selections each resolve correctly

  # ========================================
  # RESOLUTION
  # ========================================

  Scenario: A profile model resolves its stored maxImages value
    Given a session against a profile with maxImages 7
    When the session creation path resolves the profile max-images value
    Then it resolves to the stored value 7

  Scenario: A profile model with maxImages 0 resolves the no-vision sentinel
    Given a session against a profile with maxImages 0
    When the session creation path resolves the profile max-images value
    Then it resolves to the explicit 0 (no vision) rather than the default

  Scenario: A profile model without a maxImages key resolves absent
    Given a session against a profile with no maxImages key
    When the session creation path resolves the profile max-images value
    Then it resolves to absent (the tool layer applies the default 4)

  Scenario: A non-profile model resolves absent
    Given a session against a cloud model that has no profile behind it
    When the session creation path resolves the profile max-images value
    Then it resolves to absent (the default 4 applies uniformly)

  Scenario: A mid-session switch re-resolves to the new profile's value
    Given a session against a profile with maxImages 8
    When the session switches mid-session to a profile with maxImages 0
    Then the resolution follows the new profile (0, not the previous 8)
