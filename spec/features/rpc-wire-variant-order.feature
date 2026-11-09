@BUG-596
@RPC-449
@rpc
@backward-compatibility
@multi-workspace
Feature: FspecService keeps every tarpc variant index stable
  """
  tarpc derives the request enum from the FspecService trait in declaration order, and bincode encodes a request as
  that variant index. Clients that encode by position (fspec-paseo pins set_default_model=30,
  list_provider_credentials=88, get_work_unit_details=123) break whenever a method is inserted, reordered or removed.
  The trait is composed from rust/rpc/src/service_declaration/{workspace,providers_sessions,supervision_credentials}.rs
  in that order. rust/rpc/tests/fixtures/wire_order_frozen.txt lists the methods by index; new methods are appended at
  the end of supervision_credentials.rs and then to the fixture.
  """

  Background: User Story
    As a client that encodes fspec RPC requests by variant index
    I want every existing method to keep its index
    So that new daemon releases never misroute my requests

  Scenario: Every frozen method keeps its variant index
    Given the frozen wire-order fixture
    When the FspecService methods are read in declaration order
    Then the first methods match the fixture exactly, index for index

  Scenario: The workspace methods are appended after the original methods
    Given the FspecService methods in declaration order
    When the workspace methods' indices are read
    Then list_workspaces, list_workspace_work_units and create_session_in_workspace come after get_work_unit_details
    And set_default_model is index 30 and get_work_unit_details is index 123

  Scenario: The fixture header states the formula that gives each method's index
    Given the frozen wire-order fixture whose first line states "index = line number - N"
    When N is read from the header and each method's line number is read
    Then every method's line number minus N is its declared variant index
    And list_work_units on line 4 is index 0 and get_thinking_options on line 155 is index 151
