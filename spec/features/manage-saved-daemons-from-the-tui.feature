@BUG-582
@done
@TUI-121
@tui
@security
@rust
Feature: Add, edit and remove saved daemons from the TUI
  """
  The board daemon switcher could only switch; saved daemons were managed
  with the fspec daemons CLI only. While the switcher is open, `a` opens a
  daemon form (components/daemon_form_dialog.rs: name, URL and a masked
  token), `e` opens it for the highlighted saved daemon and `d` removes it
  (after a y/n confirmation since TUI-123).
  Saving goes through DaemonDirectory::save, which applies the
  saved-daemons validation via codelet_common::saved_daemons::update; a
  refusal reopens the form with the reason. The token is never displayed
  and is redacted from Debug output; editing keeps the saved token unless a
  new one is typed. Local (discovered) daemons are not saved and cannot be
  edited or removed. TUI-123 (daemon-management-on-par-with-workspaces)
  adds the clickable header label, Settings menu entries, name and URL
  rules, connect-after-save and the removal confirmation.
  Tests: rust/fspec-tui/tests/tui121_manage_saved_daemons_from_the_tui.rs.
  """

  Background: User Story
    As a user who connects the TUI to several daemons
    I want to add, edit and remove saved daemons from the daemon switcher
    So that I manage remote daemons without leaving the TUI and my tokens are never shown

  Scenario: Adding a saved daemon from the switcher
    Given the daemon switcher is open
    When the user adds daemon "work" at "wss://fspec.example.com" with token "secret-token"
    Then "work" is saved with that URL and token
    And the switcher lists "work"
    And "secret-token" never appears on screen

  Scenario: A remote daemon without a token is refused and the form reopens
    Given the daemon switcher is open
    When the user adds daemon "work" at "wss://fspec.example.com" without a token
    Then nothing is saved
    And the form is open again showing that the daemon needs a token

  Scenario: Editing a saved daemon keeps its token unless a new one is typed
    Given saved daemon "work" at "wss://fspec.example.com" with token "secret-token"
    When the user edits it to "wss://fspec2.example.com" leaving the token empty
    Then "work" is saved with the new URL and the token "secret-token"

  Scenario: Removing a saved daemon, but not a local one
    Given the switcher lists local daemon "alpha" and saved daemon "work"
    When the user removes "work" and then tries to remove "alpha"
    Then "work" is no longer saved
    And an error explains that the local daemon "alpha" is not saved
