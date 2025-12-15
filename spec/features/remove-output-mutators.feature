@code-quality
@cleanup
@cli
@refactoring
@CLEAN-005
Feature: Remove output mutators (highlighting and diff)
  """
  Removes codelet/cli/src/highlight.rs (tree-sitter-bash syntax highlighting) and codelet/cli/src/diff.rs (ANSI color-coded diff rendering). Updates codelet/cli/src/interactive.rs to remove imports and usage. Simplifies Edit/Write tool outputs in codelet/tools/src/edit.rs and write.rs to plain text. Removes tree-sitter-highlight and tree-sitter-bash dependencies from Cargo.toml files. Deletes all related tests, examples, feature files, coverage files, and attachment directories.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All source files for highlight.rs and diff.rs modules must be deleted
  #   2. All test files related to diff rendering and text styling must be deleted
  #   3. All example files demonstrating diff rendering must be deleted
  #   4. All feature files and coverage files for CLI-006 and CLI-007 must be deleted
  #   5. All attachment directories for CLI-006 and CLI-007 must be deleted
  #   6. Interactive mode must continue to function, displaying tool results without colored diff/highlight formatting
  #   7. Edit and Write tools must return plain text output instead of diff-formatted output
  #   8. Tree-sitter dependencies (tree-sitter-highlight, tree-sitter-bash) must be removed from Cargo.toml files
  #
  # EXAMPLES:
  #   1. Delete codelet/cli/src/highlight.rs and codelet/cli/src/diff.rs source modules
  #   2. Delete codelet/tests/diff_rendering_integration_test.rs, diff_rendering_e2e_test.rs, and text_styling_test.rs
  #   3. Delete codelet/examples/demo_diff_rendering.rs and test_text_styling.rs
  #   4. Remove 'pub mod diff' and 'pub mod highlight' declarations from codelet/cli/src/lib.rs
  #   5. Remove 'use crate::diff::render_diff_line' and 'use crate::highlight::highlight_bash_command' imports from interactive.rs
  #   6. Simplify Edit tool output from 'File: path\n- old\n+ new' to just 'Edited file: path (replaced old_string with new_string)'
  #   7. Simplify Write tool output from 'File: path\n+ line1\n+ line2' to just 'Wrote file: path (N lines)'
  #   8. Remove diff rendering logic from interactive.rs handle_tool_result function (lines 544-572)
  #   9. Remove bash highlighting logic from interactive.rs handle_tool_call function (lines 440-443)
  #   10. Remove tree-sitter-highlight and tree-sitter-bash from codelet/cli/Cargo.toml and codelet/Cargo.toml workspace dependencies
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the interactive mode still function after removal, just without colored diff/highlight output?
  #   A: Yes, interactive mode should still function after removal, just without colored diff/highlight output
  #
  #   Q: Should I also remove any feature files and coverage files related to diff rendering (CLI-006, CLI-007)?
  #   A: Yes, remove all feature files and coverage files related to diff rendering (CLI-006, CLI-007)
  #
  #   Q: Are there any other consumers of highlight.rs or diff.rs besides interactive.rs that I should be aware of?
  #   A: Complete inventory found: SOURCE FILES (delete): codelet/cli/src/highlight.rs, codelet/cli/src/diff.rs. TEST FILES (delete): codelet/tests/diff_rendering_integration_test.rs, codelet/tests/diff_rendering_e2e_test.rs, codelet/tests/text_styling_test.rs. EXAMPLES (delete): codelet/examples/demo_diff_rendering.rs, codelet/examples/test_text_styling.rs. FEATURE FILES (delete): codelet/spec/features/enhanced-text-styling-with-bash-highlighting-and-diff-rendering.feature (+.coverage), codelet/spec/features/integrate-diff-rendering-for-file-change-visualization.feature (+.coverage). ATTACHMENTS (delete): codelet/spec/attachments/CLI-006/, codelet/spec/attachments/CLI-007/. FILES TO MODIFY: codelet/cli/src/lib.rs (remove module declarations), codelet/cli/src/interactive.rs (remove imports/usage), codelet/tools/src/edit.rs (simplify output), codelet/tools/src/write.rs (simplify output), codelet/cli/Cargo.toml (remove tree-sitter deps), codelet/Cargo.toml (remove workspace tree-sitter deps).
  #
  # ========================================
  Background: User Story
    As a codelet maintainer
    I want to remove output mutators (highlighting and diff modules)
    So that the codebase is simpler with less unused functionality

  Scenario: Output mutator source files are removed
    Given the codelet project contains highlight.rs and diff.rs modules
    When the output mutators are removed
    Then codelet/cli/src/highlight.rs should not exist
    Then codelet/cli/src/diff.rs should not exist
    Then codelet/cli/src/lib.rs should not contain 'pub mod diff' or 'pub mod highlight'

  Scenario: Interactive mode displays tool results without colored formatting
    Given interactive mode is running
    When the agent executes an Edit or Write tool
    Then the tool result should display without ANSI color codes
    Then the tool result should display without diff formatting prefixes

  Scenario: Edit tool returns simplified plain text output
    Given a file exists with content to edit
    When the Edit tool replaces 'old_string' with 'new_string'
    Then the output should be 'Edited file: path (replaced old_string with new_string)'
    Then the output should not contain diff prefixes like '+' or '-'

  Scenario: Write tool returns simplified plain text output
    Given a file path is specified for writing
    When the Write tool creates a file with 10 lines of content
    Then the output should be 'Wrote file: path (10 lines)'
    Then the output should not list each line with '+' prefix

  Scenario: Tree-sitter dependencies are removed from Cargo.toml
    Given the project has tree-sitter-highlight and tree-sitter-bash dependencies
    When the output mutators are removed
    Then codelet/Cargo.toml should not contain 'tree-sitter-highlight'
    Then codelet/Cargo.toml should not contain 'tree-sitter-bash'
    Then codelet/cli/Cargo.toml should not contain 'tree-sitter-highlight'
    Then codelet/cli/Cargo.toml should not contain 'tree-sitter-bash'

  Scenario: Project builds successfully without output mutator modules
    Given all output mutator files and dependencies have been removed
    When I run 'cargo build' in the codelet directory
    Then the build should complete successfully with exit code 0
    Then there should be no compilation errors related to missing modules
