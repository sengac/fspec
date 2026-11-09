@REFAC-023
@session
@refactor
@code-quality
Feature: SessionManager handle impl and manager split into small modules
  """
  rust/sessions/src/handle_impl.rs and rust/sessions/src/session_manager.rs are split into cohesive modules (child modules for inherent impls and helpers, compose_
  macro fragments where a single trait impl must stay one block). Behaviour is unchanged and
  source-shape tests read the composed module tree through codelet_test_helpers.
  """

  Background: User Story
    As an fspec developer
    I want these large files split into small modules
    So that no file exceeds 300 lines

  Scenario: Every split source file stays under 300 lines
    Given rust/sessions/src/handle_impl.rs and rust/sessions/src/session_manager.rs and their new module files
    When the line count of every Rust source file is measured
    Then no file has 300 or more lines
