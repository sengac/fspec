@REFAC-022
@session
@refactor
@code-quality
Feature: SessionManagerHandle trait split into composed fragments
  """
  rust/core/src/session_manager_handle.rs keeps its imports, status_str and the stub struct; the
  trait body and the stub's trait impl are composed from include!d compose_ macro fragments under
  rust/core/src/session_manager_handle/ (the FspecBackend pattern), and the stub's inherent impl
  lives in a sibling module. codelet_test_helpers::source_composition::read_source reproduces the
  composed trait for source-shape tests.
  """

  Background: User Story
    As an fspec developer
    I want the session-manager handle trait split into small files
    So that no file exceeds 300 lines while the trait contract stays identical

  Scenario: Every session-manager-handle source file stays under 300 lines
    Given rust/core/src/session_manager_handle.rs and its fragment directory
    When the line count of every Rust source file is measured
    Then no file has 300 or more lines

  Scenario: The composed trait keeps its methods
    Given the composed source of rust/core/src/session_manager_handle.rs
    When it is read through source_composition::read_source
    Then it declares pub trait SessionManagerHandle with fn reconcile_work_units and fn loop_list
    And StubSessionManagerHandle still implements the trait for a dyn SessionManagerHandle
