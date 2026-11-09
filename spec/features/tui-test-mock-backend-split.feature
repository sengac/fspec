@done
@REFAC-025
@BUG-388
@tui
@refactor
@code-quality
Feature: fspec-tui test MockBackend split into small modules
  """
  rust/fspec-tui/tests/common/mod.rs keeps the shared fixtures; MockBackend's field list and its
  FspecBackend impl are composed from include!d compose_ macro fragments under tests/common/mock/,
  its Default impl and scripting methods live in child modules (child modules can reach the
  struct's private fields), and the long reconnect test files move their helpers into #[path]
  support modules.
  """

  Background: User Story
    As an fspec developer
    I want the TUI test mock split into small files
    So that no test helper file exceeds 300 lines

  Scenario: Every fspec-tui test helper file stays under 300 lines
    Given rust/fspec-tui/tests/common and the reconnect test files
    When the line count of every Rust file is measured
    Then no file has 300 or more lines

  Scenario: The mock backend behaves as before
    Given a MockBackend seeded with one work unit
    When it is used as an FspecBackend
    Then list_work_units returns the seeded unit and records the call
