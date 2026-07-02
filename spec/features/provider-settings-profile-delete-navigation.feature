@done
@provider-settings
@tui
@ts-parity
@rust
@PROV-116
Feature: Profile delete restores cursor to parent provider row (PROV-036 parity)
  """
  Wire set_navigate_target(provider_id) into the delete success path so apply_pending_navigate (already called in handle_provider_credentials_loaded) moves the cursor to the parent provider row; ensure NO target is set on the Err path (no cursor jump on failure). Prefer setting the target only on Ok before the reload Action is emitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On a SUCCESSFUL profile delete the cursor returns to the parent provider row after the nav tree repaints (set_navigate_target is called before the credential reload)
  #   2. A FAILED profile delete does not move the cursor and preserves the profiles
  #   3. The save path is unchanged — saving a profile does not move the cursor to the provider row (TS only navigates on delete)
  #   4. Reuse the existing set_navigate_target / apply_pending_navigate mechanism (the same one the OAuth-disconnect path uses); do not introduce a parallel mechanism
  #
  # EXAMPLES:
  #   1. User deletes the 'fireworks' profile from an expanded openai provider that also has 'home' and confirms with y -> after refresh the cursor is on the 'openai' provider row
  #   2. User deletes the only profile and confirms -> cursor on the 'openai' provider row (now showing just the '+ Add Profile' child)
  #   3. delete_profile returns an error -> the cursor stays put and both profiles are still present
  #
  # ========================================
  Background: User Story
    As a fspec-tui user deleting an OpenAI profile
    I want to confirm the per-profile delete
    So that the cursor returns to the parent provider row exactly like the TypeScript TUI instead of being left in an arbitrary position

  @tui
  @provider-settings
  Scenario: Deleting one of several profiles returns the cursor to the provider row
    Given the "openai" provider is expanded with profiles "fireworks" and "home"
    And the cursor is on the "fireworks" profile row
    When the user presses "d" and confirms the delete with "y"
    And the backend delete succeeds and the nav tree refreshes
    Then the cursor is on the "openai" provider row

  @tui
  @provider-settings
  Scenario: Deleting the only profile returns the cursor to the provider row
    Given the "openai" provider is expanded with a single profile "fireworks"
    And the cursor is on the "fireworks" profile row
    When the user presses "d" and confirms the delete with "y"
    And the backend delete succeeds and the nav tree refreshes
    Then the cursor is on the "openai" provider row
    And the "+ Add Profile" row is the only child shown

  @tui
  @provider-settings
  @error
  Scenario: A failed delete does not move the cursor and preserves the profiles
    Given the "openai" provider is expanded with profiles "fireworks" and "home"
    And the cursor is on the "fireworks" profile row
    When the user presses "d" and confirms the delete with "y"
    And the backend delete returns an error
    Then the cursor does not jump to the "openai" provider row
    And both profiles "fireworks" and "home" are still present

  @tui
  @provider-settings
  Scenario: Saving a profile does not move the cursor to the provider row
    Given the "openai" provider is expanded with profiles "fireworks" and "home"
    And the cursor is on the "fireworks" profile row
    When the user edits and saves the "fireworks" profile
    And the backend save succeeds and the nav tree refreshes
    Then the cursor is not forced onto the "openai" provider row
