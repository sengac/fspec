@done
@BUG-468
@tui
@testing
@rust
Feature: Mock-backed TUI sessions open without error notices
  """
  After BUG-466 made ten FspecBackend defaults refuse, the shared
  MockBackend (rust/fspec-tui/tests/common/mock) lacked seven of them
  (get/set_continue_state, get/set_goal_state, set_thinking_level_default,
  rename_profile, send_input_with_thinking), so every mock-backed agent
  session showed "[error] /continue state unavailable" in its scrollback.
  The mock now implements them, storing continue and goal state in
  common/backend_state_mock.rs (with the scripted list_sessions failure);
  list_sessions moved to mock_backend_7.rs to keep the fragments under 300
  lines. The RPC-018/022/058/059 source-shape contracts assert the Result
  return types, and the RPC-009 App smoke tests attach the stub session
  manager (temp_service_with_stub) because RPC-451 shows a failed provider
  listing at startup.
  Tests: rust/fspec-tui/tests/bug468_mock_backed_sessions_open_cleanly.rs.
  """

  Background: User Story
    As a developer running the TUI test suite
    I want to have the mock backend and contract tests match the refusing session APIs
    So that the suite tests behaviour instead of tripping over error notices from unimplemented mock methods

  Scenario: An agent session on the mock backend opens without an error notice
    Given an App on the mock backend with a focused agent session
    When the pending startup work is drained
    Then the session's scrollback holds no error notice
