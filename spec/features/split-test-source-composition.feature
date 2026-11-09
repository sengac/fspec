@done
@REFAC-027
@BUG-372
@BUG-392
@rpc
@code-quality
Feature: Source-shape tests read split test files as one source
  """
  Splitting oversized test files leaves a root (e.g. tests/foo.rs, an integration-test crate root) declaring
  `mod helpers;` beside itself and `#[path = "foo/part.rs"] mod part;` parts. Source-shape tests that inspect such files
  read them through codelet_test_helpers::source_composition::read_module_tree / read_tree_or_file, which now honour
  `#[path]` attributes and resolve a crate root's children beside it, as rustc does.
  BUG-372: read_tree_or_file returns the module-tree error for a .rs path instead of silently reading the single file;
  only paths that are not .rs files are read as plain text.
  """

  Background: User Story
    As an fspec maintainer
    I want source-shape tests to read a split test file as one source
    So that splitting files under the 300-line limit does not hide what they assert

  Scenario: Source-shape tests read a split test file as one source
    Given a test crate root that declares a sibling module and a path-attributed part module
    When the module tree is read through the shared source composition helper
    Then the text contains the root, the sibling module and the part module

  Scenario: A Rust file whose module tree is broken reports the error
    Given a test crate root that declares a sibling module whose file is missing
    When the root is read through read_tree_or_file
    Then the read fails and names the missing module
    And the root file is not returned on its own

  Scenario: A Rust file that does not parse reports the error
    Given a Rust file that does not parse
    When the file is read through read_tree_or_file
    Then the read fails instead of returning the raw text
