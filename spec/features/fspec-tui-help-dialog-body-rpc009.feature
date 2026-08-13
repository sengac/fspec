@done
@tui
@rust
@infrastructure
@rpc
@RPC-009
@critical
Feature: Help dialog body update (RPC-009)
  """
  The HelpDialog static body string in rust/fspec-tui/src/components/help_dialog.rs:25 (`HELP_BODY`) changes from the RPC-008 placeholder text (`?  ESC  q`) to one-line-per-key listing exactly: `j/k  Navigate`, `Tab  Switch pane`, `?    Toggle this help`, `q    Quit fspec-tui`, `Enter  Send`, `Ctrl+C  Interrupt`, `ESC  Dismiss this dialog`. The `HelpDialog::render` body is otherwise unchanged — same `Priority::Critical`, same `tui_popup::Popup` wrapped in the SizedWidgetRef adapter (`HelpBody`), same width/height calculation. The RPC-008 keybinding-content scenario (`help_dialog_static_body_lists_question_esc_and_q_keybindings`) is preserved (the new body still contains `?`, `ESC`, `q`); a NEW RPC-009 scenario asserts the additional substrings `Tab`, `Enter`, `Ctrl+C`, `j`, `k` are present. The existing insta snapshot file `help_dialog__centered_popup_80x24.snap` at rust/fspec-tui/src/components/snapshots/ is regenerated via `cargo insta review`.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want the existing RPC-008 HelpDialog body text replaced with one-line-per-key listings for `j`, `k`, `Tab`, `?`, `q`, `Enter`, `Ctrl+C` — same Priority::Critical Component, same tui-popup adapter, no new dialog widget code
    So that pressing `?` shows accurate keybindings for the new two-pane UI introduced in this card without inventing a new dialog framework

  Scenario: HelpDialog body lists every keybinding from the RPC-009 scope on one line each
    Given an isolated HelpDialog component
    When the dialog is rendered onto an 80x24 TestBackend
    Then the rendered buffer contains the substring "j"
    And the rendered buffer contains the substring "k"
    And the rendered buffer contains the substring "Tab"
    And the rendered buffer contains the substring "?"
    And the rendered buffer contains the substring "q"
    And the rendered buffer contains the substring "Enter"
    And the rendered buffer contains the substring "Ctrl+C"
    And the rendered buffer contains the substring "ESC"

  Scenario: HelpDialog still uses the tui-popup adapter at Priority::Critical (RPC-008 invariant)
    Given an isolated HelpDialog component
    Then its priority() returns Priority::Critical
    And its render(...) implementation constructs a `tui_popup::Popup` wrapping a `SizedWidgetRef` adapter
    And it does NOT define a hand-rolled `centered_rect` helper as the production code path

  Scenario: HelpDialog rendering is byte-equal across runs (insta snapshot regenerated)
    Given an isolated HelpDialog component rendered onto an 80x24 TestBackend
    When the buffer cell grid is serialised via `insta::assert_yaml_snapshot!`
    Then the serialised output matches the regenerated snapshot file "help_dialog__centered_popup_80x24.snap"
