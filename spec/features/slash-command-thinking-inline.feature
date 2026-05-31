@done
@tui-component
@rpc
@rust
@agent-view
@slash-command
@tui
@RPC-048
Feature: /thinking off|low|med|high inline-arg parsing
  """
  New SlashCommandParse variants: `SetThinkingLevel(ThinkingLevel)` and `InvalidThinkingLevel(String)`. parse_slash_command extends its `/thinking ...` branch to call `trimmed.strip_prefix("/thinking ")` and match the trimmed lowercased arg against off|low|med|medium|high; anything else yields InvalidThinkingLevel.
  Dispatch wiring goes into dispatch_rpc020.rs::handle_input_submitted: new arms for SetThinkingLevel(level) → self.handle_thinking_level_selected(session, level) (existing RPC-022 helper handles backend.set_thinking_level + get_thinking_level refresh) and InvalidThinkingLevel(other) → self.navigator.agent.push_line(&mut self.agent_view_store, format!("[error] unknown thinking level: {other}")). NO changes to dispatch_rpc022.rs are required.
  Tests: extend slash_parser inline `mod tests` to cover all 6 paths (off|low|med|medium|high + invalid + bare). Integration tests in tests/slash_command_wiring_rpc022.rs (or a new tests/slash_thinking_rpc048.rs) drive the App with `submit_input("/thinking high")` against MockBackend and assert (a) backend.set_thinking_level called with (s-1, High), (b) no dialog pushed, (c) send_input NOT called, (d) thinking_level_for(s-1) folds via the refresh, (e) error-arm pushes scrollback line for invalid arg. RPC-024 source-shape (300-LoC ceiling) constraint stays clean — only slash_parser.rs and dispatch_rpc020.rs are touched, both well under the ceiling.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. parse_slash_command MUST recognise `/thinking <arg>` where <arg> is off|low|med|medium|high (case-insensitive) and return SlashCommandParse::SetThinkingLevel(ThinkingLevel) without opening the ThinkingLevelDialog
  #   2. parse_slash_command MUST recognise `/thinking <unknown>` (anything other than off|low|med|medium|high) and return SlashCommandParse::InvalidThinkingLevel(other) so the dispatcher can emit `[error] unknown thinking level: {other}` into the focused session's scrollback
  #   3. Bare `/thinking` (no arg, with or without trailing whitespace-only suffix) MUST continue to return SlashCommandParse::OpenThinkingDialog (RPC-022 behaviour is preserved)
  #   4. On SetThinkingLevel(level), handle_input_submitted MUST route through handle_thinking_level_selected so backend.set_thinking_level + backend.get_thinking_level fire AND AgentViewStore.thinking_level_for(session_id) is refreshed via Action::ThinkingLevelLoaded — the text is NOT forwarded to backend.send_input and NO dialog is pushed
  #   5. On InvalidThinkingLevel(other), handle_input_submitted MUST push `[error] unknown thinking level: {other}` into the focused session's scrollback via navigator.agent.push_line — the text is NOT forwarded to backend.send_input and NO backend.set_thinking_level call fires
  #
  # EXAMPLES:
  #   1. parse_slash_command("/thinking off") returns SlashCommandParse::SetThinkingLevel(ThinkingLevel::Off)
  #   2. parse_slash_command("/thinking low") returns SlashCommandParse::SetThinkingLevel(ThinkingLevel::Low)
  #   3. parse_slash_command("/thinking med") returns SlashCommandParse::SetThinkingLevel(ThinkingLevel::Medium)
  #   4. parse_slash_command("/thinking medium") returns SlashCommandParse::SetThinkingLevel(ThinkingLevel::Medium)
  #   5. parse_slash_command("/thinking HIGH") returns SlashCommandParse::SetThinkingLevel(ThinkingLevel::High) (case-insensitive)
  #   6. parse_slash_command("/thinking gibberish") returns SlashCommandParse::InvalidThinkingLevel("gibberish")
  #   7. parse_slash_command("/thinking") still returns SlashCommandParse::OpenThinkingDialog (bare command unchanged)
  #   8. Given an App with one open session s-1 and AgentViewStore.thinking_level_for(s-1) = Some(Off), when input "/thinking high" is submitted, then within 1 second backend.set_thinking_level is called with (s-1, ThinkingLevel::High) AND no ThinkingLevelDialog is pushed AND no text is forwarded to backend.send_input
  #   9. Given the same session s-1 with backend.get_thinking_level returning ThinkingLevel::High, after the spawned task completes, AgentViewStore.thinking_level_for(s-1) folds to Some(ThinkingLevel::High) via Action::ThinkingLevelLoaded
  #   10. Given an App with one open session s-1, when input "/thinking gibberish" is submitted, then s-1's scrollback contains the line "[error] unknown thinking level: gibberish" AND backend.set_thinking_level is NOT called AND no dialog is pushed
  #   11. Given an App with one open session s-1, when input "/thinking" (bare) is submitted, then the ThinkingLevelDialog is pushed onto the Compositor (RPC-022 parity, no regression)
  #
  # ========================================
  Background: User Story
    As a AgentView user
    I want to type /thinking off|low|med|medium|high to switch reasoning level inline
    So that I can change thinking levels without opening the picker dialog every time

  Scenario Outline: parse_slash_command recognises /thinking <level> inline arg
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text "<input>"
    Then it returns SlashCommandParse::SetThinkingLevel(<level>)

    Examples:
      | input            | level                 |
      | /thinking off    | ThinkingLevel::Off    |
      | /thinking low    | ThinkingLevel::Low    |
      | /thinking med    | ThinkingLevel::Medium |
      | /thinking medium | ThinkingLevel::Medium |
      | /thinking high   | ThinkingLevel::High   |
      | /thinking HIGH   | ThinkingLevel::High   |

  Scenario: parse_slash_command returns InvalidThinkingLevel for an unknown arg
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text "/thinking gibberish"
    Then it returns SlashCommandParse::InvalidThinkingLevel("gibberish")

  Scenario: Bare /thinking continues to open the ThinkingLevelDialog
    Given the function parse_slash_command from app/slash_parser.rs
    When it is called with text "/thinking"
    Then it returns SlashCommandParse::OpenThinkingDialog

  Scenario: Submitting "/thinking high" sets the level via the backend and does NOT open the dialog
    Given an App with one open session SessionId("s-1") wired to a MockBackend
    And AgentViewStore.thinking_level_for(SessionId("s-1")) = Some(ThinkingLevel::Off)
    And the MockBackend's get_thinking_level returns ThinkingLevel::High
    When the input is submitted with text "/thinking high"
    Then within 1 second backend.set_thinking_level is called exactly once with (SessionId("s-1"), ThinkingLevel::High)
    And no ThinkingLevelDialog is pushed onto the Compositor
    And the text is NOT forwarded to backend.send_input
    And after the spawned task completes AgentViewStore.thinking_level_for(SessionId("s-1")) is Some(ThinkingLevel::High)

  Scenario: Submitting "/thinking gibberish" emits an error notice and does NOT call the backend
    Given an App with one open session SessionId("s-1") wired to a MockBackend
    When the input is submitted with text "/thinking gibberish"
    Then SessionId("s-1") scrollback contains a chunk whose text equals "[error] unknown thinking level: gibberish"
    And backend.set_thinking_level is NOT called
    And no ThinkingLevelDialog is pushed onto the Compositor
    And the text is NOT forwarded to backend.send_input

  Scenario: Submitting bare "/thinking" still opens the ThinkingLevelDialog (RPC-022 parity)
    Given an App with one open session SessionId("s-1") wired to a MockBackend
    And no dialogs are pushed onto the Compositor
    When the input is submitted with text "/thinking"
    Then a ThinkingLevelDialog with id "thinking-level-dialog" is pushed onto the Compositor at Priority::Foreground
    And the text is NOT forwarded to backend.send_input
    And backend.set_thinking_level is NOT called
