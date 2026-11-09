@done
@tui
@providers
@provider-settings
@critical
@BUG-575
Feature: Provider Settings loads and writes never corrupt the view
  """
  Implementation:
  - one App-level refresh (app/provider_settings_refresh.rs, spawn_provider_settings_refresh) reads list_provider_credentials then list_provider_settings in one task and sends Action::ProviderSettingsRefreshed with its number (an App counter bumped per request), both results and an optional navigate request (a provider or profile target plus the view's navigation epoch when the write began). Writes send Action::RefreshProviderSettings { navigate } when they finish instead of listing themselves; ProviderSettingsConfigsLoaded and ProfileDeleteNavigate are removed. The view (views/provider_settings/refresh.rs) shows a refresh only when its number is newer than the one shown, and a new view ignores every refresh requested before it opened. The Settings form opens only once the settings read has answered. Focus restore compares provider, kind and, for a profile, its bare name (bare_profile_name), so a changed label keeps the row; every rebuild clamps the cursor and scroll into the tree. OAuth numbers come from a process-wide counter, taken by each view and each login start (list_actions.rs, oauth_copilot.rs); the Copilot device start task is tracked. MockBackend gets per-call gates (tests/common/call_gates.rs) so two reads can be released in reverse order. The sign-in success path (app/dispatch_provider_settings_oauth.rs, BUG-634's area, not edited here) still lists credentials itself after set_navigate_target and sends ProviderCredentialsLoaded: that list is folded as is, every refresh requested before it is dropped, and the recorded target applies unless the cursor moved.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Product owner decision (2026-11-07): no settings form opens before that provider's settings have loaded, and saving never deletes stored settings the user did not clear.
  #   2. Credentials and settings load together in one refresh and are shown together: the providers, their profile rows and Settings rows appear only once both reads have answered.
  #   3. Every refresh is numbered when it is requested; a refresh older than the newest one shown is dropped, so refreshes that finish in reverse order leave the newest state on screen.
  #   4. The return to a provider row after logout, sign-in or profile delete belongs to the refresh that write requested: an earlier refresh never takes it, and it is dropped if the user moved the cursor since the write began.
  #   5. After a reload the cursor stays on the same row (provider, kind of row and profile name), even when a profile's label changes; when that row is gone the cursor stays inside the tree.
  #   6. A status message is coloured by its outcome (a ✓ message is never red), and it is cleared when the view changes mode (opening, cancelling or saving a form).
  #   7. Before the first refresh answers the list says 'Loading providers…'; a refresh that fails says the providers could not be loaded and why, never '(no providers configured)'.
  #   8. An OAuth result from a login that was cancelled, or from an earlier visit to /provider, never acts: every view and every login start takes a new number from one process-wide counter, Esc moves the view past the cancelled login's number, and Esc cancels every login task, the GitHub Copilot device start included.
  #
  # EXAMPLES:
  #   1. Enter on anthropic Settings while the settings read is still running: no form opens and the status says the settings are still loading; once they load the form shows the stored values, and saving one change keeps every other stored setting
  #   2. The settings read fails: Enter on Settings opens no form and the status says the settings could not be loaded
  #   3. Save profile p1, then delete profile p2; the two refreshes finish in reverse order: p2 stays gone and p1 is shown
  #   4. Log out of Codex while a refresh from saving a key is still running: the earlier refresh does not move the cursor; after the logout refresh the cursor is on the Codex provider row
  #   5. Log out of Codex and move the cursor before the logout refresh lands: the cursor stays where the user put it
  #   6. Edit profile local and rename it to zlocal: after the save the cursor is on zlocal's row, not on another profile
  #   7. A reload shrinks the tree below the cursor: the cursor is on the last row and Enter still acts on it
  #   8. Start a GitHub Copilot login, press Esc twice, reopen /provider; the old device start answers: the list stays in list mode and no device poll starts
  #   9. Save profile a, then open profile b's form; 'saved' arrives while b's form is open: it is not shown in red
  #   10. Type a bad Max Images value, see the hint, press Esc: the hint is gone from the list
  #   11. Open /provider on a slow connection: 'Loading providers…' until the refresh answers; a failed refresh says the providers could not be loaded and why
  #
  # ========================================
  Background: User Story
    As a user changing provider credentials, profiles and settings in /provider
    I want to work in the view while its loads and writes are still finishing
    So that my stored settings are never erased, the tree shows the newest state, and the cursor and messages stay where they belong

  Scenario: The Settings form does not open before the settings have loaded
    Given anthropic has stored settings and the settings read has not answered yet
    When I open /provider, expand anthropic and press Enter on Settings
    Then no settings form opens and the status says the settings are still loading
    And once the settings have loaded, Enter opens the form with the stored values
    And saving one changed field keeps every other stored setting

  Scenario: Settings that could not be read cannot be edited
    Given the settings read fails
    When I open /provider, expand anthropic and press Enter on Settings
    Then no settings form opens and the status says the settings could not be loaded

  Scenario: Refreshes that finish in reverse order leave the newest state
    Given profiles p1 and p2 are listed under openai
    When profile p1 is saved and profile p2 is deleted, and the delete's refresh answers before the save's
    Then p2 is not listed and p1 is listed

  Scenario: An earlier refresh does not take the return to the provider row
    Given I am on the Codex logout row while a refresh from saving a key is still running
    When I log out of Codex and the earlier refresh answers first
    Then the earlier refresh leaves the cursor where it was
    And after the logout refresh the cursor is on the Codex provider row

  Scenario: Moving the cursor during a logout keeps it where I put it
    Given I log out of Codex
    When I move the cursor before the logout refresh answers
    Then the cursor stays on the row I moved to

  Scenario: Renaming a profile keeps the cursor on it
    Given profiles a, local and m are listed under openai and the cursor is on local
    When local is renamed to zlocal and the refresh answers
    Then the cursor is on zlocal

  Scenario: A reload that shrinks the tree keeps the cursor inside it
    Given the cursor is on the last of many rows
    When a refresh answers with a much shorter tree
    Then the cursor is on the last row and the view draws without error

  Scenario: An OAuth answer from an earlier visit is ignored
    Given I started a GitHub Copilot login and pressed Esc twice
    When I open /provider again and the old device start answers
    Then the view stays in the provider list and no device poll starts

  Scenario: A saved message is not shown in red
    Given profile b's form is open
    When the status "✓ openai: a profile saved" arrives
    Then the status is drawn in green, not red

  Scenario: Leaving a form clears its hint
    Given the profile form shows a hint about a bad Max Images value
    When I press Esc
    Then the provider list shows no status

  Scenario: Loading and failed loads are not shown as no providers
    Given the first refresh has not answered
    When the view is drawn
    Then it says "Loading providers…"
    And when the refresh fails it says the providers could not be loaded and why
