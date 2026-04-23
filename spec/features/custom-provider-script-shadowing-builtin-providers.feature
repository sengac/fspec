@done
@PROV-085
Feature: Remove BUILTIN_PROVIDER_NAMES guard so Rhai scripts may shadow built-in providers
  """
  Custom configs shadowing built-ins must leave the NameConflict variant in the enum (public API stability) but the default path never produces it
  The FSPEC_DISABLE_SCRIPT_SHADOWING check lives in the ProviderType::from_str path (and its custom_provider_registered helper) so a single env var toggle flips behaviour across both FromStr and map_provider_id_to_type
  Updated test custom_config_and_loader_tests.rs::reject_provider_name_that_collides_with_a_builtin_provider must be rewritten (not deleted) to assert the NEW behaviour: loading succeeds and produces a ProviderConfig
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderConfig::validate no longer returns NameConflict for names matching claude, openai, codex, gemini, zai, copilot, or github-copilot
  #   2. The BUILTIN_PROVIDER_NAMES constant is removed from custom/config.rs
  #   3. When discovery returns a custom config whose name collides with a built-in provider, ProviderType::from_str resolves the slug to ProviderType::Custom(slug) instead of the hardcoded built-in variant
  #   4. Setting FSPEC_DISABLE_SCRIPT_SHADOWING=1 restores the original behaviour: built-in slug names resolve to the hardcoded built-in variant and the custom config is ignored for that slug
  #   5. The NameConflict error variant stays in the public API so existing callers compile; it is simply never produced by the default code path
  #
  # EXAMPLES:
  #   1. A valid provider JSON named 'claude' with a .rhai script loads via ProviderConfig::from_file and validate() returns Ok
  #   2. Discovery places a claude.json config with name='claude' in ~/.fspec/providers/, and ProviderType::from_str("claude") resolves to ProviderType::Custom("claude")
  #   3. With FSPEC_DISABLE_SCRIPT_SHADOWING=1 and the same claude.json present, ProviderType::from_str("claude") resolves to ProviderType::Claude (the hardcoded built-in)
  #   4. Loading a provider config named 'codex' with a valid script and auth block returns Ok instead of NameConflict
  #   5. Loading a provider config with an invalid name like 'My Provider' (whitespace/caps) still fails with InvalidName — shadowing only affects built-in collisions, not pattern validation
  #
  # ========================================
  Background: User Story
    As a fspec maintainer shipping first-class scripted subscription providers
    I want to have custom Rhai provider configs named after built-in providers (claude, codex, openai, gemini, zai, copilot, github-copilot) load successfully and take precedence over the hardcoded built-in
    So that I can ship claude-code.rhai and codex.rhai as shadowing first-class scripts while still keeping the hardcoded path available behind an escape hatch for CI regression testing

  Scenario: Load a custom provider config named 'claude' without NameConflict
    Given a valid JSON provider config with name "claude" and a valid .rhai script on disk
    When I call ProviderConfig::from_file on the JSON path
    Then the result is Ok and the loaded ProviderConfig has name "claude"

  Scenario: Load a custom provider config named 'codex' without NameConflict
    Given a valid JSON provider config with name "codex" and a valid .rhai script on disk
    When I call ProviderConfig::from_file on the JSON path
    Then the result is Ok and the loaded ProviderConfig has name "codex"

  Scenario: Shadowing custom config resolves provider slug to Custom variant
    Given a discovered custom provider config with name "claude" is registered in the global providers directory
    And the FSPEC_DISABLE_SCRIPT_SHADOWING environment variable is unset
    When I call ProviderType::from_str("claude")
    Then the result is ProviderType::Custom("claude")

  Scenario: Escape hatch env var disables shadowing and restores hardcoded built-in
    Given a discovered custom provider config with name "claude" is registered in the global providers directory
    And the FSPEC_DISABLE_SCRIPT_SHADOWING environment variable is set to "1"
    When I call ProviderType::from_str("claude")
    Then the result is ProviderType::Claude

  Scenario: Invalid name pattern still fails with InvalidName
    Given a provider config JSON with name "My Provider" containing whitespace and uppercase
    When I call ProviderConfig::from_file on the JSON path
    Then the result is an InvalidName error mentioning the allowed pattern ^[a-z][a-z0-9-]*$
