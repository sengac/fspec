@code-search
@tool-execution
@CORE-005
Feature: AstGrep Tool for AST-based Code Search

  """
  Architecture notes:
  - Uses ast-grep-core 0.40.0 and ast-grep-language 0.40.0 crates for native Rust integration (NOT shell out to binary)
  - SupportLang enum provides 27 languages with auto-detection via from_path() from file extension
  - Pattern API: lang.ast_grep(source).root().find_all(pattern) returns iterator of NodeMatch
  - NodeMatch provides start_pos().line() (0-based), start_pos().column(), text(), and get_env() for captured variables
  - Uses ignore crate (already a dependency) for gitignore-aware directory walking
  - Meta-variable syntax: $VAR for single node capture, $$$VAR for multiple node capture (ellipsis)
  - Pattern must be valid syntax in target language - parsed as AST before matching
  - Follows existing Tool trait pattern with ToolParameters JSON schema for LLM integration
  - Applies shared truncation utilities from truncation.rs (30000 chars, 2000 per line)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool must use native ast-grep Rust crate (ast-grep-core + ast-grep-language) as library, NOT shell out to binary
  #   2. Tool must support pattern parameter (required) and language parameter (required) with optional paths parameter
  #   3. Search results must be returned in file:line:column:text format for easy parsing by AI agents
  #   4. Support all SupportLang languages: TypeScript, JavaScript, TSX, Rust, Python, Go, Java, C, Cpp, etc. (27 languages)
  #   5. Pattern syntax must support meta-variables: $VAR for single node capture, $$$VAR for multiple node capture
  #   6. Error messages for invalid patterns must guide the agent to correct the pattern syntax with helpful examples
  #   7. Apply same output limits as other tools (30000 chars max, 2000 chars per line) using shared truncation utilities
  #   8. AstGrepTool must implement Tool trait and be registered in ToolRegistry.with_core_tools()
  #
  # EXAMPLES:
  #   1. Agent searches for 'function executeTool' in TypeScript, result shows src/agent/tools.ts with line number and function signature
  #   2. Agent searches pattern 'logger.$METHOD($$$ARGS)' in TypeScript to find all logger method calls across codebase
  #   3. Agent searches pattern 'fn $NAME($$$ARGS) -> $RET' in Rust to find all function definitions with return types
  #   4. Agent provides invalid AST pattern 'function {{{', receives helpful error with pattern syntax guide
  #   5. Agent searches with paths=['src/'] to limit search to specific directory
  #   6. Large codebase search returns truncated results at 30000 chars with warning message
  #
  # ========================================

  Background: User Story
    As a AI coding agent exploring codebases
    I want to search code using AST-based pattern matching
    So that I can find code by syntax structure with fewer false positives than text-based search

  Scenario: Find function definition by pattern
    Given a directory with TypeScript files containing function definitions
    When I execute the AstGrep tool with pattern "function executeTool" and language "typescript"
    Then the result should contain file paths with line numbers and column positions
    And the result should be in "file:line:column:text" format
    And the result should not be an error

  Scenario: Find method calls using meta-variable pattern
    Given a directory with TypeScript files containing logger calls
    When I execute the AstGrep tool with pattern "logger.$METHOD($$$ARGS)" and language "typescript"
    Then the result should contain all files with logger method calls
    And the matched text should include the method names and arguments

  Scenario: Find Rust function definitions with return types
    Given a directory with Rust source files
    When I execute the AstGrep tool with pattern "fn $NAME($$$ARGS) -> $RET { $$$BODY }" and language "rust"
    Then the result should contain function definitions with return types
    And the result should show the function names and return types

  Scenario: Handle invalid pattern syntax with helpful error
    Given an invalid AST pattern "function {{{"
    When I execute the AstGrep tool with the invalid pattern and language "typescript"
    Then the result should contain "Error"
    And the result should contain pattern syntax guidance
    And the result should suggest how to fix the pattern

  Scenario: Limit search to specific directory paths
    Given a project with files in src and tests directories
    When I execute the AstGrep tool with pattern "fn $NAME" and language "rust" and paths ["src/"]
    Then the result should only contain files from the src directory
    And the result should not contain files from other directories

  Scenario: Large search results are truncated at character limit
    Given a large codebase with many matching patterns
    When I execute the AstGrep tool with a pattern that matches many files
    Then the output should be truncated at 30000 characters
    And a truncation warning should be included

  Scenario: AstGrepTool is registered in default ToolRegistry
    Given a default ToolRegistry
    Then the registry should contain the "AstGrep" tool
    And the AstGrep tool should have the correct name

  Scenario: Runner can execute AstGrep tool
    Given a Runner with default tools
    When I execute the AstGrep tool through the runner
    Then the execution should succeed
    And the result should contain search matches
