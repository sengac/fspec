@done
@BUG-169
@bug-169
@bug
@input
@agent-view
@tui
@smoke
Feature: Slash-command submit interception is registry-driven (Tab/Esc then Enter no longer sends commands to the LLM)
  """
  Two dispatch paths exist for slash commands: (A) popup pick via Action::SlashCommandSelected → App::handle_slash_command (exhaustive over all 21 registry entries); (B) typed submit via Action::InputSubmitted → App::handle_input_submitted → parse_slash_command (partial, only 11 families). Tab-fill (PopupOutcome::Filled) and Esc (PopupOutcome::Dismiss) close the popup per RPC-020, so path A is unreachable afterwards and path B — the parser — is the only interceptor. Fix: a registry-driven catch in parse_slash_command (first token case-insensitively looked up against SLASH_COMMANDS via SlashCommandAction::name, exact bare-name match only) returning a new BareCommand(SlashCommandAction) variant, routed in handle_input_submitted to the existing handle_slash_command handler (single source of truth per AGENTS.md; no send_input, no history append, no pending-input draft clear). No popup/UI changes: Tab/Esc semantics stay exactly as spec'd in rpc020-slash-and-file-popups.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: parse_slash_command performs a registry-driven catch BEFORE the NotASlashCommand fallback: the first whitespace-delimited token of the trimmed text must start with '/' and its name (token minus '/'), case-insensitive, must match a SlashCommandAction.name() in the SLASH_COMMANDS registry. Match requires the trimmed text to be EXACTLY that token (no arguments).
  #   2. R2: The registry catch returns a new variant SlashCommandParse::BareCommand(SlashCommandAction) carrying the matched action. Existing family branches (/model, /thinking …, /role …, /schedule, /loop, /continue, /goal, /update, /mux) are evaluated first and take precedence; the registry catch only runs for text no existing branch recognized.
  #   3. R3: handle_input_submitted routes SlashCommandParse::BareCommand(action) by calling the existing handle_slash_command(action) (the same handler the popup path uses — single source of truth) and RETURNS immediately: no backend.send_input, no persistence_add_history (RPC-022 rule), no pending-input draft clear.
  #   4. R4: Exact-match-only semantics. A registered name with trailing arguments (e.g. '/provider openai'), an unknown name ('/unknown'), a bare '/' with empty name, or non-slash prose all still parse to NotASlashCommand and fall through to backend.send_input — the legacy behaviour pinned by slash_command_wiring_rpc022.rs and provider_settings_dispatch_rpc054.rs ('/providers' is NOT a command) is unchanged.
  #
  # EXAMPLES:
  #   1. Tab-fill then Enter: type '/provide' (popup open, provider highlighted), press Tab → buffer becomes '/provider' and popup closes; press Enter → ProviderSettingsView opens; backend.send_input is never called
  #   2. Esc-dismiss then Enter: type '/provider' (popup open), press Esc → popup closes with buffer unchanged; press Enter → ProviderSettingsView opens; backend.send_input is never called
  #   3. Every bare-only registered command intercepted on typed submit: submitting /help, /clear, /quit, /resume, /search, /debug, /compact, /isolation, /blocklist, /detach, /merge-worktree (popup closed at submit) routes to the same handler as a popup pick — no command is sent to the LLM
  #   4. No regression — arguments flow to the LLM: submitting '/provider openai' (registered name plus a trailing argument) is NOT a bare command and is sent to the LLM via backend.send_input, exactly like before the fix
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to type a full slash command name (e.g. /provider) and press Enter after Tab-fill or Esc-dismiss
    So that the command is intercepted by its handler instead of being sent to the LLM as plain text

  @unit
  @smoke
  Scenario Outline: parse_slash_command resolves exact bare registered names case-insensitively and trims surrounding whitespace
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text=<input>
    Then it returns the new variant BareCommand(<action>)

    Examples:
      | input           | action        |
      | /provider       | Provider      |
      | /HELP           | Help          |
      | "  /clear  "    | Clear         |
      | /merge-worktree | MergeWorktree |

  @unit
  @regression
  Scenario Outline: parse_slash_command keeps NotASlashCommand for unregistered or argument-carrying lines
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text=<input>
    Then it returns NotASlashCommand

    Examples:
      | input            |
      | /provider openai |
      | /unknown         |
      | /                |
      | hello world      |
      | /providers       |

  @dispatch
  @integration
  Scenario Outline: Submitting a bare registered command routes to the popup-pick handler and never sends text to the LLM
    Given an App with one open session SessionId("s-1") in AgentView
    When the input is submitted with text "<cmd>"
    Then the text is NOT forwarded to backend.send_input
    And the observable side effect <effect> lands (the same handler a popup pick would invoke)

    Examples:
      | cmd             | effect                                                                                             |
      | /help           | a HelpDialog with id "help-dialog" is pushed onto the Compositor                                   |
      | /clear          | the focused session's scrollback chunk_count becomes 0 and backend.clear_history is called         |
      | /quit           | App.should_quit becomes true                                                                       |
      | /resume         | the AgentView's resume mode view is open (resume_view is Some) and backend.list_sessions is called |
      | /search         | the AgentView's search mode view is open (search_view is Some)                                     |
      | /provider       | the Navigator flips to ViewMode::ProviderSettings                                                  |
      | /debug          | backend.toggle_debug is called and a "[debug] capture toggled" notice lands in scrollback          |
      | /compact        | backend.compact_session is called for the focused session                                          |
      | /isolation      | a CreateSessionDialog with id "create-session-dialog" is pushed preselecting Isolated              |
      | /blocklist      | the Navigator flips to ViewMode::Blocklist                                                         |
      | /detach         | backend.set_work_unit_context(s-1, None) is called and the session's work-unit binding is cleared  |
      | /merge-worktree | backend.inspect_session_changes is called for the focused session                                  |

  @integration
  @regression
  Scenario: Tab-fill then Enter on a typed command submits the command, not the text (reported bug)
    Given an App with one open session SessionId("s-1") in AgentView
    When the user types "/provide" and the slash popup is open with "provider" highlighted
    And the user presses Tab so the input fills with "/provider" and the popup closes
    And the user presses Enter
    Then the text "/provider" is NOT forwarded to backend.send_input
    And the Navigator flips to ViewMode::ProviderSettings
    And the same effect is observed as a popup pick of the Provider command

  @integration
  @regression
  Scenario: Esc-dismiss then Enter on a typed command also intercepts (second trigger of the bug)
    Given an App with one open session SessionId("s-1") in AgentView
    When the user types "/provider" and the slash popup is open
    And the user presses Esc so the popup closes and the input buffer is unchanged ("/provider")
    And the user presses Enter
    Then the text "/provider" is NOT forwarded to backend.send_input
    And the Navigator flips to ViewMode::ProviderSettings
    And the same effect is observed as a popup pick of the Provider command

  @integration
  @regression
  Scenario: A registered name with a trailing argument is NOT a bare command and goes to the LLM
    Given an App with one open session SessionId("s-1") in AgentView
    When the input is submitted with text "/provider openai"
    Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "/provider openai")
    And the Navigator's active_view stays ViewMode::Agent (no ProviderSettings flip)

  @integration
  @regression
  Scenario: Unknown slash lines are unchanged
    Given an App with one open session SessionId("s-1") in AgentView
    When the input is submitted with text "/unknown anything"
    Then a tokio task is spawned that calls backend.send_input(SessionId("s-1"), "/unknown anything")

  @regression
  @unit
  Scenario: Existing path-B families still parse to their existing variants
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with each of the legacy family inputs
    Then it returns the same variant as before the fix:
      | input          | expected variant            |
      | /thinking high | SetThinkingLevel(High)      |
      | /role clear    | ClearRole                   |
      | /goal          | GoalSubcommand(Show)        |
      | /update check  | UpdateSubcommand(CheckOnly) |
      | /schedule      | ScheduleSubcommand(Help)    |
      | /loop list     | LoopSubcommand(List)        |
      | /continue      | ContinueSubcommand(Toggle)  |
      | /model         | OpenModelDialog             |
      | /mux           | MuxCommand("/mux")          |

  @persistence
  @regression
  Scenario: Intercepted bare commands do NOT append to the per-session history
    Given an App with one open session SessionId("s-1")
    When the input is submitted with text "/provider"
    Then no tokio task is spawned that calls backend.persistence_add_history
    When the input is submitted with text "hello"
    Then exactly one tokio task is spawned that calls backend.persistence_add_history(SessionId("s-1"), "hello")
