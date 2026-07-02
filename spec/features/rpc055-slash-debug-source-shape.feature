@done
@testing
@RPC-055
@rpc
@rust
@source-shape
Feature: /debug RPC surface source shape
  """
  Pin the source-shape contract that downstream slices in RPC-030 depend
  on: set_debug_directory MUST be declared at each layer of the dual
  transport stack (trait + tarpc service + backend trait + both transport
  forwarders) and the App's slash-command routing MUST live in its own
  dispatch_slash_debug.rs file so the orchestrator dispatch_slash_commands.rs stays
  under the 300-LoC ceiling.

  These tests run against source files at compile time — they catch
  refactors that accidentally collapse the dual-transport layering or
  inline the /debug wiring back into dispatch_slash_commands.rs.
  """

  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the layering of the /debug RPC surface
    So that no future refactor can collapse the dual-transport boundary

  Scenario: SessionManagerHandle declares set_debug_directory
    Given the file codelet/core/src/session_manager_handle.rs is compiled
    Then it declares a trait method named "set_debug_directory" that takes a PathBuf and returns Result<(), String>

  Scenario: FspecService declares set_debug_directory
    Given the file codelet/rpc/src/lib.rs is compiled
    Then it declares an async fn named "set_debug_directory" with parameter type String and return type Result<(), String>

  Scenario: FspecBackend declares set_debug_directory
    Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    Then it declares an async fn named "set_debug_directory" on the FspecBackend trait

  Scenario: Both transports implement set_debug_directory
    Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    Then each file contains an impl of "set_debug_directory" that calls the corresponding tarpc client method

  Scenario: /debug slash command wiring lives in dispatch_slash_debug.rs
    Given the file codelet/fspec-tui/src/app/dispatch_slash_debug.rs exists
    Then it declares a method named "handle_slash_debug"
    And it declares a method named "try_dispatch_slash_debug"
