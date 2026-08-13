@code-search
@tool-execution
@CORE-004
Feature: Grep and Glob Tools Implementation

  """
  Architecture notes:
  - Uses grep crate (ripgrep core) with regex crate for pattern matching - no external binary needed
  - Uses ignore crate for gitignore-aware directory walking and glob pattern matching
  - Dependencies: grep, ignore, regex crates in Cargo.toml
  - Reuses shared truncation utilities from src/tools/truncation.rs and limits from src/tools/limits.rs
  - Implements Tool trait following same pattern as BashTool, ReadTool, WriteTool, EditTool
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GrepTool must use the grep crate (ripgrep's core library) for regex-based content search
  #   2. GrepTool must support three output modes: files_with_matches (default), content (with line numbers), count (file:count format)
  #   3. GrepTool must support context lines (-A after, -B before, -C both) in content mode only
  #   4. GrepTool must support case-insensitive search (-i flag) and multiline matching
  #   5. GrepTool must support glob pattern filter and type filter for restricting search to specific files
  #   6. GlobTool must use the ignore crate for gitignore-aware file pattern matching
  #   7. GlobTool must respect .gitignore by default (ignore crate default behavior)
  #   8. Both tools must truncate output at 30000 characters with truncation warning
  #   9. Both tools must replace lines longer than 2000 characters with '[Omitted long line]'
  #   10. Both tools must implement the Tool trait and register in ToolRegistry.with_core_tools()
  #
  # EXAMPLES:
  #   1. Grep for 'TODO' in directory returns file paths: file1.ts, file2.js
  #   2. Grep with output_mode='content' returns '2:export function hello()' format
  #   3. Grep with glob='*.ts' only returns matches from TypeScript files
  #   4. Grep with -A=2 includes 2 lines of context after each match
  #   5. Grep with -i flag matches 'error', 'Error', and 'ERROR'
  #   6. Grep with multiline=true matches 'function foo(\n  param1' spanning lines
  #   7. Grep with output_mode='count' returns 'file1.ts:3' showing 3 matches
  #   8. Glob for '**/*.ts' returns all TypeScript files recursively
  #   9. Glob with path='src' limits search to src directory only
  #   10. Glob for 'nonexistent*.xyz' returns 'No matches found'
  #   11. Glob respects .gitignore and excludes node_modules by default
  #   12. Runner.new() includes GrepTool and GlobTool in available_tools()
  #
  # ========================================

  Background: User Story
    As a developer using AI agents for coding
    I want to search codebases for content patterns and find files by glob patterns
    So that quickly navigate and understand large codebases through AI tool calls

  # ==========================================
  # GREP TOOL SCENARIOS
  # ==========================================

  Scenario: Grep search returns file paths containing pattern
    Given a directory with files containing "TODO" comments
    When I execute the Grep tool with pattern "TODO"
    Then the result should contain file paths of matching files
    And the result should not be an error

  Scenario: Grep with content mode shows matching lines with line numbers
    Given a directory with source files
    When I execute the Grep tool with pattern "export function" and output_mode "content"
    Then the result should contain lines with line numbers in "N:" format
    And the result should contain "function"

  Scenario: Grep with glob filter only searches matching files
    Given a directory with .ts and .js files containing "TODO"
    When I execute the Grep tool with pattern "TODO" and glob "*.ts"
    Then the result should only contain .ts files
    And the result should not contain .js files

  Scenario: Grep with context lines includes surrounding lines
    Given a directory with source files
    When I execute the Grep tool with pattern "TODO" and -A set to 2 in content mode
    Then the result should include context lines after each match

  Scenario: Grep with case-insensitive flag matches all cases
    Given a file containing "ERROR", "error", and "Error"
    When I execute the Grep tool with pattern "error" and -i flag
    Then the result should contain all three variations

  Scenario: Grep with multiline mode matches patterns spanning lines
    Given a file with a multi-line function definition
    When I execute the Grep tool with a multiline pattern and multiline enabled
    Then the result should match content spanning multiple lines

  Scenario: Grep with count mode returns match counts per file
    Given multiple files containing the search pattern
    When I execute the Grep tool with output_mode "count"
    Then the result should show file paths with their match counts in "file:N" format

  Scenario: Grep for non-existent pattern returns no matches message
    Given a directory with source files
    When I execute the Grep tool with a non-existent pattern
    Then the result should be "No matches found"

  # ==========================================
  # GLOB TOOL SCENARIOS
  # ==========================================

  Scenario: Glob search returns all files matching pattern
    Given a project directory with TypeScript files in various directories
    When I execute the Glob tool with pattern "**/*.ts"
    Then the result should contain all TypeScript files recursively

  Scenario: Glob with path parameter limits search to directory
    Given a project with files in src and lib directories
    When I execute the Glob tool with pattern "*.ts" and path "src"
    Then the result should only contain files from the src directory

  Scenario: Glob for non-existent pattern returns no matches
    Given a project directory
    When I execute the Glob tool with pattern "nonexistent*.xyz"
    Then the result should be "No matches found"

  Scenario: Glob respects gitignore by default
    Given a project with node_modules directory and .gitignore
    When I execute the Glob tool with pattern "**/*.js"
    Then the result should not include files from node_modules

  # ==========================================
  # TOOL REGISTRY INTEGRATION SCENARIOS
  # ==========================================

  Scenario: GrepTool and GlobTool are registered in default ToolRegistry
    Given a default ToolRegistry
    Then the registry should contain the "Grep" tool
    And the registry should contain the "Glob" tool

  Scenario: Runner can execute Grep and Glob tools
    Given a Runner with default tools
    When I execute the Grep tool through the runner
    Then the execution should succeed
    When I execute the Glob tool through the runner
    Then the execution should succeed
