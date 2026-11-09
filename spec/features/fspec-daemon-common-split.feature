@REFAC-024
@cli
@refactor
@code-quality
Feature: fspec daemon common module split into small modules
  """
  rust/fspec/src/common.rs are split into cohesive modules (child modules for inherent impls and helpers, compose_
  macro fragments where a single trait impl must stay one block). Behaviour is unchanged and
  source-shape tests read the composed module tree through codelet_test_helpers.
  """

  Background: User Story
    As an fspec developer
    I want these large files split into small modules
    So that no file exceeds 300 lines

  Scenario: Every split source file stays under 300 lines
    Given rust/fspec/src/common.rs and their new module files
    When the line count of every Rust source file is measured
    Then no file has 300 or more lines
