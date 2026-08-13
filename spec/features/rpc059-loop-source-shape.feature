@done
@tui
@RPC-059
@rpc
@rust
@source-shape
@loop-management
Feature: /loop RPC surface source shape
  """
  Pin the source-shape contract that the rest of RPC-030 depends on:

  - The three loop_add/loop_cancel/loop_list RPC methods MUST be
  declared at every layer of the dual-transport stack
  (SessionManagerHandle trait + tarpc FspecService + FspecBackend
  trait + both transport forwarders).
  - The new wire type RegisteredLoop MUST exist as a public
  declaration in codelet-rpc-types with all seven documented fields.
  - The LoopSubcommand enum + parse_loop_command MUST live in
  rust/fspec-tui/src/app/loop_parser.rs.
  - All slash-command wiring for /loop MUST live in
  rust/fspec-tui/src/app/dispatch_slash_loop.rs (mirrors
  dispatch_slash_schedule) so the orchestrator dispatch.rs stays under the
  300-LoC ceiling.

  These tests run against source files at compile/parse time — they
  catch refactors that accidentally collapse the dual-transport
  layering, drop any of the three RPC methods, or inline the /loop
  wiring back into dispatch.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. One new wire type lives in codelet-rpc-types: RegisteredLoop struct (flat, napi(object)-compatible).
  #   2. SessionManagerHandle MUST expose default-impl methods for all three RPC operations.
  #   3. StubSessionManagerHandle MUST expose per-call counters for cross-transport parity tests.
  #   4. FspecService (tarpc) MUST declare async fns for all three operations.
  #   5. FspecBackend trait MUST expose async variants of all three methods.
  #   6. EmbeddedFspecBackend AND WebSocketFspecBackend MUST forward each method to the tarpc client.
  #   7. LoopSubcommand enum + parse_loop_command MUST live in rust/fspec-tui/src/app/loop_parser.rs.
  #   8. All /loop wiring MUST live in rust/fspec-tui/src/app/dispatch_slash_loop.rs.
  #
  # ========================================
  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the layering of the /loop RPC surface
    So that no future refactor can collapse the dual-transport boundary or strand the parser

  Scenario: RegisteredLoop wire type is exported from codelet-rpc-types
    Given the file rust/rpc-types/src/lib.rs is compiled
    Then it declares a public struct named "RegisteredLoop"
    And RegisteredLoop has fields named id, session_id, prompt, interval_seconds
    And RegisteredLoop has fields named created_at, expires_at, last_run_at

  Scenario: SessionManagerHandle declares the three new loop methods
    Given the file rust/core/src/session_manager_handle.rs is compiled
    Then it declares a trait method named "loop_add" returning Result<RegisteredLoop, String>
    And it declares a trait method named "loop_cancel" returning Result<bool, String>
    And it declares a trait method named "loop_list" returning Vec<RegisteredLoop>

  Scenario: StubSessionManagerHandle exposes per-call counters for all three loop methods
    Given the file rust/core/src/session_manager_handle.rs is compiled
    Then StubSessionManagerHandle declares a method named "loop_add_calls" returning u64
    And StubSessionManagerHandle declares a method named "loop_cancel_calls" returning u64
    And StubSessionManagerHandle declares a method named "loop_list_calls" returning u64

  Scenario: FspecService declares the three new RPC methods
    Given the file rust/rpc/src/lib.rs is compiled
    Then it declares an async fn named "loop_add" with return type Result<RegisteredLoop, String>
    And it declares an async fn named "loop_cancel" with return type Result<bool, String>
    And it declares an async fn named "loop_list" with return type Vec<RegisteredLoop>

  Scenario: FspecBackend declares the three new methods
    Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    Then it declares an async fn named "loop_add" on the FspecBackend trait returning Result<RegisteredLoop>
    And it declares an async fn named "loop_cancel" on the FspecBackend trait returning Result<bool>
    And it declares an async fn named "loop_list" on the FspecBackend trait returning Result<Vec<RegisteredLoop>>

  Scenario: Both transports implement the three new methods
    Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    Then each file contains an impl of "loop_add" that calls the corresponding tarpc client method
    And each file contains an impl of "loop_cancel" that calls the corresponding tarpc client method
    And each file contains an impl of "loop_list" that calls the corresponding tarpc client method

  Scenario: loop_parser module exists with the documented entry points
    Given the file rust/fspec-tui/src/app/loop_parser.rs exists
    Then it declares a public enum named "LoopSubcommand"
    And LoopSubcommand has variants named Add, Cancel, List, Help
    And it declares a public fn named "parse_loop_command" taking &str and returning LoopSubcommand

  Scenario: /loop slash command wiring lives in dispatch_slash_loop.rs
    Given the file rust/fspec-tui/src/app/dispatch_slash_loop.rs exists
    Then it declares a method named "handle_slash_loop_help"
    And it declares a method named "handle_loop_subcommand"
    And it declares a method named "handle_loop_add"
    And it declares a method named "handle_loop_list"
    And it declares a method named "handle_loop_cancel"
    And it declares a method named "try_dispatch_slash_loop"
