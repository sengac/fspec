@done
@RPC-061
@rust
@tui
@rpc
@agent-view
@supervisor
@session-management
@source-shape
Feature: RPC-061 source-shape — supervisor surface file layout
  """
  Source-shape regression test for RPC-061. Pins the file locations
  and member-symbol invariants for the new supervisor surface across
  rpc-types, core (SessionManagerHandle trait), rpc (FspecService
  trait), sessions (handle_impl), fspec-tui (FspecBackend trait,
  Action enum, SessionHeader, SessionFooter, dispatch_supervisor_links,
  app/dispatch.rs catch-all).

  Companion features:
  - spec/features/rpc061-cross-transport-parity.feature
  - spec/features/rpc061-supervisor-links.feature
  """

  Background: User Story
    As a fspec TUI maintainer extending the supervisor surface to the Rust frontend
    I want every cross-crate touchpoint pinned by a source-shape test
    So that NAPI-free dependency layering and AgentView parity invariants stay enforced

  Scenario: codelet-rpc-types exposes IncomingMessageInput
    Given the crate codelet-rpc-types is compiled
    Then it declares a public struct named "IncomingMessageInput"
    And the struct has field "source_session_id" of type String
    And the struct has field "role_name" of type String
    And the struct has field "message" of type String
    And the struct has field "images" of type Option<Vec<IncomingMessageImage>>
    And the struct derives Debug, Clone, PartialEq, Eq, Serialize, Deserialize

  Scenario: SessionManagerHandle trait declares all five supervisor methods
    Given the trait file codelet/core/src/session_manager_handle.rs is compiled
    Then it declares fn add_supervisor with the documented signature
    And it declares fn remove_supervisor
    And it declares fn get_subordinate
    And it declares fn get_subordinates
    And it declares fn receive_incoming_message

  Scenario: FspecService trait declares all five supervisor methods
    Given the crate codelet-rpc is compiled
    Then the FspecService trait declares async fn add_supervisor
    And it declares async fn remove_supervisor
    And it declares async fn get_subordinate
    And it declares async fn get_subordinates
    And it declares async fn receive_incoming_message

  Scenario: FspecBackend trait gains the five supervisor forwarders
    Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    Then it declares async fn add_supervisor
    And it declares async fn remove_supervisor
    And it declares async fn get_subordinate
    And it declares async fn get_subordinates
    And it declares async fn receive_incoming_message

  Scenario: codelet-sessions handle_impl wires the five supervisor methods
    Given the file codelet/sessions/src/handle_impl.rs is compiled
    Then it impls fn add_supervisor
    And it impls fn remove_supervisor
    And it impls fn get_subordinate
    And it impls fn get_subordinates
    And it impls fn receive_incoming_message

  Scenario: components/mod.rs Action enum gains the two RPC-061 variants
    Given the file codelet/fspec-tui/src/components/mod.rs is compiled
    Then it declares Action::SupervisorsLoaded
    And it declares Action::SendToSubordinate

  Scenario: SessionHeader gains a subordinate_label field
    Given the file codelet/fspec-tui/src/views/agent/header.rs is compiled
    Then SessionHeader declares subordinate_label

  Scenario: SessionFooter gains a supervisor_pending_count field
    Given the file codelet/fspec-tui/src/views/agent/footer.rs is compiled
    Then SessionFooter declares supervisor_pending_count

  Scenario: dispatch_supervisor_links.rs has the documented helper surface
    Given the file codelet/fspec-tui/src/app/dispatch_supervisor_links.rs is compiled
    Then it declares method "handle_supervisors_loaded"
    And it declares method "handle_send_to_subordinate"
    And it declares method "try_dispatch_supervisor_links"
    And the file stays under 300 lines

  Scenario: app/dispatch.rs catch-all routes through try_dispatch_supervisor_links
    Given the file codelet/fspec-tui/src/app/dispatch.rs is compiled
    Then it calls self.try_dispatch_supervisor_links
    And the file stays under 300 lines
