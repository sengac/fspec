@session
@session-management
@rust
@testing
@BUG-412
Feature: Stub session manager handle reads poisoned tables honestly
  """
  StubSessionManagerHandle (rust/core/src/session_manager_handle/stub*) is
  the in-memory SessionManagerHandle the transport and TUI tests run against.
  Its trait methods have infallible signatures, so BUG-412 routes every table
  read and write through stub_lock: a poisoned mutex is logged at error level
  and its data is used as-is, instead of `.ok()` / `if let Ok` /
  `unwrap_or_default()` turning a poisoned table into an empty one. Methods
  that return Result keep reporting poisoning as an error. Tests: the
  bug412_tests module in rust/core/src/session_manager_handle/stub.rs.
  """

  Background: User Story
    As a developer running tests against the stub session manager
    I want a panicked test thread not to make the stub look empty
    So that a later assertion reports the real state instead of a misleading empty table

  Scenario: A poisoned stub table is read with its data
    Given a stub table holding one entry
    And a thread panicked while holding the table's lock
    When the stub locks the table
    Then the table still holds its one entry

  Scenario: No stub table read swallows a poisoned lock
    Given the StubSessionManagerHandle source
    When its lock acquisitions are listed
    Then each one goes through stub_lock or returns the poisoning as an error
