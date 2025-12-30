@done
@high
@codelet
@tools
@ast
@TOOLS-003
Feature: AST Code Refactor Tool for Codelet

  """
  Implements rig::tool::Tool trait following existing AstGrepTool pattern in codelet/tools/src/astgrep.rs. Uses ast-grep-core and ast-grep-language crates for AST parsing. NAPI refactor implementation in codelet/napi/src/astgrep.rs provides reference algorithm. Supports 23 languages. Two modes: extract-to-file (default) and replace-in-place. Target file append mode for collecting multiple extractions. Async file I/O with tokio.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool must implement rig::tool::Tool trait following the same pattern as AstGrepTool
  #   2. Refactor operation requires exactly one AST match - zero matches or multiple matches must return an error
  #   3. Tool must support all 23 languages supported by ast-grep (TypeScript, TSX, JavaScript, Rust, Python, Go, Java, C, C++, C#, Ruby, Kotlin, Swift, Scala, PHP, Bash, HTML, CSS, JSON, YAML, Lua, Elixir, Haskell)
  #   4. Tool must extract matched code from source file and write it to target file
  #   5. Tool must clean up resulting blank lines in source file after code extraction
  #   6. Tool must use async file operations with tokio for non-blocking I/O
  #   7. Yes, support BOTH modes: extract-to-file (default) AND replace-in-place with a replacement pattern
  #   8. Append to target file if it exists (useful for collecting multiple extractions)
  #   9. No dry-run mode for initial implementation - keep it simple
  #
  # EXAMPLES:
  #   1. Agent calls astgrep_refactor with pattern 'const $NAME = () => { $$$BODY }', language 'typescript', source 'src/components.ts', target 'src/extracted.ts' - extracts arrow function to new file
  #   2. Agent calls astgrep_refactor with pattern 'fn $NAME($_)' on a file with 3 matching functions - returns error listing all 3 match locations
  #   3. Agent calls astgrep_refactor with pattern 'class NonExistent' - returns error 'No matches found for pattern'
  #   4. Agent calls astgrep_refactor with language 'invalid_lang' - returns error with list of supported languages
  #   5. After successful extraction, source file has consecutive blank lines collapsed to single blank line
  #   6. Tool returns AstGrepRefactorResult with success=true, moved_code containing extracted text, source_file and target_file paths
  #   7. Agent calls astgrep_refactor with replacement pattern - matched code is replaced in-place rather than extracted
  #   8. Agent extracts function A, then extracts function B to same target file - both functions appear in target file
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the refactor tool support a replace-in-place mode in addition to extract-to-file?
  #   A: Yes, support BOTH modes: extract-to-file (default) AND replace-in-place with a replacement pattern
  #
  #   Q: When writing to target file, should it overwrite or append if file exists?
  #   A: Append to target file if it exists (useful for collecting multiple extractions)
  #
  #   Q: Should the tool support a dry-run mode?
  #   A: No dry-run mode for initial implementation - keep it simple
  #
  # ========================================

  Background: User Story
    As a AI agent using codelet tools
    I want to refactor code by extracting AST-matched patterns to new files
    So that I can perform structural code refactoring with precision and safety

  @happy-path
  Scenario: Extract code matching AST pattern to new file
    Given a TypeScript file "src/components.ts" containing an arrow function component
    And the pattern "const $NAME = () => { $$$BODY }" matches exactly one function
    When the agent calls astgrep_refactor with extract mode
    And specifies target file "src/extracted.ts"
    Then the matched function should be written to "src/extracted.ts"
    And the function should be removed from "src/components.ts"
    And the tool should return success with moved_code containing the extracted text

  @error-handling
  Scenario: Error when pattern matches multiple nodes
    Given a Rust file containing 3 functions matching pattern "fn $NAME($_)"
    When the agent calls astgrep_refactor with this pattern
    Then the tool should return an error
    And the error message should list all 3 match locations
    And no files should be modified

  @error-handling
  Scenario: Error when pattern matches zero nodes
    Given a source file with no class definitions
    When the agent calls astgrep_refactor with pattern "class NonExistent"
    Then the tool should return an error "No matches found for pattern"
    And no files should be modified

  @error-handling
  Scenario: Error when invalid language specified
    Given a valid source file
    When the agent calls astgrep_refactor with language "invalid_lang"
    Then the tool should return an error
    And the error should list all 23 supported languages

  @happy-path
  Scenario: Blank lines cleaned up after extraction
    Given a source file with a function surrounded by blank lines
    When the agent extracts the function using astgrep_refactor
    Then consecutive blank lines in the source file should be collapsed to single blank line
    And the source file should remain syntactically valid

  @happy-path
  Scenario: Successful refactor returns complete result structure
    Given a valid refactor operation
    When the agent calls astgrep_refactor and it succeeds
    Then the result should contain success=true
    And the result should contain moved_code with the extracted text
    And the result should contain source_file path
    And the result should contain target_file path

  @replace-mode
  Scenario: Replace matched code in-place with replacement pattern
    Given a source file with a function to refactor
    And a replacement pattern to transform the code
    When the agent calls astgrep_refactor in replace mode
    Then the matched code should be replaced in-place
    And no extraction to target file should occur
    And the source file should contain the replacement code

  @append-mode
  Scenario: Append to existing target file
    Given a target file "src/extracted.ts" already containing function A
    When the agent extracts function B to the same target file
    Then function B should be appended to "src/extracted.ts"
    And function A should still be present in the target file
    And both functions should appear in order