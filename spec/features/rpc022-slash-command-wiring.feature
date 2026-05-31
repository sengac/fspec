@done
@RPC-022
@rust
@tui
@slash-command
@agent-view
Feature: Slash-command parsing for /model, /thinking, and /role
  """
  /model and /thinking are simple verbs with no arguments — the slash
  command popup intercepts them on Enter and emits
  `SlashCommandSelected(Model | Thinking)` which dispatch_rpc022.rs
  turns into a dialog push.

  /role is different: the user types `/role <text>` so once they type
  the SPACE, popups.rs `classify_buffer` returns Close and the popup
  goes away. The argument is parsed at SUBMIT time by a new
  `parse_slash_command` helper called from
  `handle_input_submitted`:

  - "/model"                  → Action::OpenModelDialog
  - "/thinking"               → Action::OpenThinkingDialog
  - "/role"                   → Action::SetSessionRole(sid, None)         (treat bare /role as clear)
  - "/role clear"             → Action::SetSessionRole(sid, None)
  - "/role <text>"            → Action::SetSessionRole(sid, Some(text))
  - any other "/cmd ..."      → falls through to backend.send_input (existing behaviour)
  - any non-slash text        → falls through to backend.send_input (existing behaviour)

  Submitted slash commands DO NOT publish to persistence_add_history
  (mirrors TS Ink TUI behaviour — only user-bound LLM input is
  history-worthy).
  """

  Background: User Story
    As a Rust fspec TUI user
    I want to invoke /model, /thinking, and /role from the input line and have them open dialogs or set role text without being sent to the LLM
    So that slash commands behave like commands, not like questions

  @parse
  @smoke
  Scenario Outline: parse_slash_command recognises the four wired commands
    Given the function parse_slash_command from app/dispatch_rpc022.rs
    When it is called with text=<input>
    Then it returns <expected_variant>

    Examples:
      | input                             | expected_variant                       |
      | /model                            | OpenModelDialog                        |
      | /thinking                         | OpenThinkingDialog                     |
      | /role                             | ClearRole                              |
      | /role clear                       | ClearRole                              |
      | /role You are a security reviewer | SetRole("You are a security reviewer") |
      | /role  leading space ok           | SetRole("leading space ok")            |
      | hello world                       | NotASlashCommand                       |
      | /unknown anything                 | NotASlashCommand                       |

  @model
  @dispatch
  Scenario: Submitting "/model" opens the ModelSelectorDialog and spawns list_providers
    Given an App with one open session SessionId("s-1") and no dialogs pushed
    And the backend's list_providers returns [ProviderInfo{ key: "openai", ... }]
    When the input is submitted with text "/model"
    Then the text is NOT forwarded to backend.send_input
    And a ModelSelectorDialog with id "model-selector-dialog" is pushed onto the Compositor at Priority::Foreground
    And a tokio task is spawned that calls backend.list_providers()
    When the spawned task completes
    Then Action::ListProvidersLoaded([ProviderInfo{ key: "openai", ... }]) is dispatched
    And the open ModelSelectorDialog now contains 1 provider

  @thinking
  @dispatch
  Scenario: Submitting "/thinking" opens the ThinkingLevelDialog seeded with the cached level
    Given an App with one open session SessionId("s-1") and no dialogs pushed
    And AgentViewStore.thinking_level_for(SessionId("s-1")) = Some(ThinkingLevel::Medium)
    When the input is submitted with text "/thinking"
    Then the text is NOT forwarded to backend.send_input
    And a ThinkingLevelDialog is pushed onto the Compositor at Priority::Foreground
    And the dialog's initial selected_level is ThinkingLevel::Medium

  @role
  @dispatch
  Scenario: Submitting "/role You are a reviewer" sets the role and shows the RoleBanner
    Given an App with one open session SessionId("s-1") and no dialogs pushed
    And AgentViewStore.role_for(SessionId("s-1")) is None
    When the input is submitted with text "/role You are a reviewer"
    Then the text is NOT forwarded to backend.send_input
    And Action::SetSessionRole(SessionId("s-1"), Some("You are a reviewer")) is dispatched
    And AgentViewStore.role_for(SessionId("s-1")) becomes Some("You are a reviewer")
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a reviewer".to_string()))

  @role
  @dispatch
  @clear
  Scenario: Submitting "/role clear" clears the role and hides the RoleBanner
    Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A")
    When the input is submitted with text "/role clear"
    Then Action::SetSessionRole(SessionId("s-1"), None) is dispatched
    And AgentViewStore.role_for(SessionId("s-1")) becomes None
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)

  @role
  @dispatch
  @clear
  Scenario: Submitting bare "/role" is treated as a clear
    Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A")
    When the input is submitted with text "/role"
    Then Action::SetSessionRole(SessionId("s-1"), None) is dispatched
    And AgentViewStore.role_for(SessionId("s-1")) becomes None

  @passthrough
  Scenario: Submitting plain text falls through to backend.send_input unchanged
    Given an App with one open session SessionId("s-1") and no dialogs pushed
    When the input is submitted with text "hello world"
    Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "hello world")
    And no dialog is pushed onto the Compositor
    And no Action::SetSessionRole is dispatched

  @persistence
  Scenario: Slash commands are NOT appended to the per-session history
    Given an App with one open session SessionId("s-1")
    When the input is submitted with text "/model"
    Then no tokio task is spawned that calls backend.persistence_add_history
    When the input is submitted with text "hello"
    Then exactly one tokio task is spawned that calls backend.persistence_add_history(SessionId("s-1"), "hello")

  @popup-integration
  Scenario: Slash popup selection of /model also opens the ModelSelectorDialog
    Given an App with one open session SessionId("s-1") and the slash popup open with selected command Model
    When the user presses Enter inside the popup
    Then Action::SlashCommandSelected(SlashCommandAction::Model) is dispatched
    And a ModelSelectorDialog with id "model-selector-dialog" is pushed onto the Compositor at Priority::Foreground
    And a tokio task is spawned that calls backend.list_providers()

  @popup-integration
  @role
  @clear
  Scenario: Slash popup selection of /role is treated as a clear and does not surface a [notice]
    Given an App with one open session SessionId("s-1") whose role is Some("Reviewer A") and the slash popup open with selected command Role
    When the user presses Enter inside the popup
    Then Action::SlashCommandSelected(SlashCommandAction::Role) is dispatched
    And AgentViewStore.role_for(SessionId("s-1")) becomes None
    And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)
    And no scrollback line containing the substring "[notice] /role" is appended
