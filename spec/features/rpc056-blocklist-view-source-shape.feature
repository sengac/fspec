@done
@testing
@tui
@RPC-056
@rpc
@rust
@source-shape
Feature: /blocklist RPC surface source shape
  """
  Pin the source-shape contract that downstream slices in RPC-030 depend on:
  blocklist_list MUST be declared at every layer of the dual-transport stack
  (trait + tarpc service + backend trait + both transport forwarders),
  BlocklistRuleInfo MUST exist as a public wire type in codelet-rpc-types,
  ViewMode::Blocklist MUST exist on the Navigator, and the slash-command
  dispatch wiring MUST live in its own dispatch_blocklist.rs file so the
  orchestrator dispatch_slash_commands.rs stays under the 300-LoC ceiling.

  These tests run against source files at compile time — they catch
  refactors that accidentally collapse the dual-transport layering or
  inline the /blocklist wiring back into dispatch_slash_commands.rs.
  """

  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the layering of the /blocklist RPC surface
    So that no future refactor can collapse the dual-transport boundary or strand the view

  Scenario: BlocklistRuleInfo is exported from codelet-rpc-types
    Given the file rust/rpc-types/src/lib.rs is compiled
    Then it declares a public struct named "BlocklistRuleInfo"
    And the struct has fields named id, pattern, action, reason, guidance, source

  Scenario: SessionManagerHandle declares blocklist_list
    Given the file rust/core/src/session_manager_handle.rs is compiled
    Then it declares a trait method named "blocklist_list" that returns Vec<BlocklistRuleInfo>

  Scenario: StubSessionManagerHandle exposes a blocklist_list call counter
    Given the file rust/core/src/session_manager_handle.rs is compiled
    Then StubSessionManagerHandle declares a method named "blocklist_list_calls" returning u64

  Scenario: FspecService declares blocklist_list
    Given the file rust/rpc/src/lib.rs is compiled
    Then it declares an async fn named "blocklist_list" with return type Vec<BlocklistRuleInfo>

  Scenario: FspecBackend declares blocklist_list
    Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    Then it declares an async fn named "blocklist_list" on the FspecBackend trait returning Result<Vec<BlocklistRuleInfo>>

  Scenario: Both transports implement blocklist_list
    Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    Then each file contains an impl of "blocklist_list" that calls the corresponding tarpc client method

  Scenario: BlocklistView module exists with the documented entry points
    Given the file rust/fspec-tui/src/views/blocklist/mod.rs exists
    Then it declares a public struct named "BlocklistView"
    And it declares an enum named "BlocklistEvent" or its rename equivalent
    And it declares a free function named "derive_category" returning &'static str

  Scenario: Navigator exposes a ViewMode::Blocklist variant
    Given the file rust/fspec-tui/src/views/navigator.rs is compiled
    Then ViewMode declares a variant named "Blocklist"

  Scenario: /blocklist slash command wiring lives in dispatch_blocklist.rs
    Given the file rust/fspec-tui/src/app/dispatch_blocklist.rs exists
    Then it declares a method named "handle_open_blocklist_view"
    And it declares a method named "handle_close_blocklist_view"
    And it declares a method named "handle_blocklist_rules_loaded"
    And it declares a method named "try_dispatch_blocklist"
