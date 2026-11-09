@tui
@provider-settings
@oauth
@providers
@BUG-634
Feature: OAuth code entry keeps URL actions out of the code
  """
  Architecture: App routes code-entry keys through ProviderSettingsView.
  URL copy and open actions retain URL focus regardless of launcher outcome.
  Existing clipboard and launcher seams verify App effects without a browser.
  """

  Background: User Story
    As a user signing in with a code
    I want URL actions to stay separate from code entry
    So that copying or opening the URL never contaminates my authorization code

  Scenario: Copy then open keeps URL focus and leaves the code empty
    Given sign in with code has URL focus
    When I press c then o
    Then the URL is copied and opened
    And URL focus remains with an empty code and URL action hints
    And Enter does not submit a code

  Scenario: Failed open then copy follows the recovery hint without typing c
    Given sign in with code has URL focus and the browser launcher fails
    When I press o then c
    Then the failure explains how to copy and the URL is copied
    And URL focus remains with an empty code
    And Enter does not submit a code

  Scenario: Explicit field focus preserves codes starting with c or o after URL actions
    Given sign in with code has URL focus
    When I copy the URL and focus the Code field with Tab or Down
    And I type a code starting with c or o and press Enter
    Then exactly the typed code is submitted without opening a URL

  Scenario: Pasting after URL actions preserves codes starting with c or o
    Given sign in with code has URL focus
    When I copy and open the URL then paste a code starting with c or o
    Then the paste focuses the Code field and preserves the code
    And Enter submits exactly the pasted code

  Scenario: Other typing enters code and returning to the URL preserves it
    Given sign in with code has URL focus
    When I copy the URL and type a code starting with a different character
    And I return to the URL with Up and press c then o
    Then the URL is copied and opened without changing the code
    And Down returns to the Code field where c and o are typed literally
