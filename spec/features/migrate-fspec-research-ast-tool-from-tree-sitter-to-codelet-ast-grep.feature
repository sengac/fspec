@codelet
@research-tools
@refactoring
@done
@REFAC-006
Feature: Migrate fspec research AST tool from tree-sitter to codelet ast-grep

  """
  
  - Rust implementation in codelet/napi/src/astgrep.rs exposes astGrepSearch() and astGrepRefactor() via NAPI
  - TypeScript CLI wrapper in src/research-tools/ast.ts handles argument parsing and calls NAPI functions
  - Existing codelet/tools/src/astgrep.rs AstGrepTool contains core pattern matching logic to reuse
  - Removes 17 tree-sitter npm dependencies and associated TypeScript code (query-executor.ts, language-loader.ts, ast-queries/)
  
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The tool must include a refactor capability that can move matched code to a new file, not just extract/display it
  #   2. Refactor only does what ast-grep can deterministically do: remove matched code from source file, write matched code to new file. No guessing imports or adding code.
  #   3. Business logic (AST parsing, pattern matching, refactoring) in Rust exposed via NAPI. CLI handling (fspec research --tool=ast) remains in TypeScript.
  #   4. NAPI exports: astGrepSearch(pattern, language, paths) and astGrepRefactor(pattern, language, sourceFile, targetFile)
  #   5. Must update tool definitions in both codelet facades and tools modules
  #   6. Remove all 17 tree-sitter dependencies from package.json and delete all deprecated TypeScript AST code
  #   7. Use exact ast-grep pattern syntax ($NAME, $$$ARGS, etc.) with no wrappers or simplification
  #   8. Refactor uses fixed target filename. User searches first to see matches, then refactors one specific match to a specific file. No variable interpolation in target path.
  #   9. Refactor requires exactly 1 match. If pattern matches multiple nodes, error out with list of matches so user can refine the pattern.
  #
  # EXAMPLES:
  #   1. Search: fspec research --tool=ast --pattern='function $NAME($$$ARGS)' --lang=typescript --path=src/
  #   2. Refactor workflow: 1) Search to find matches, 2) Pick specific match, 3) Refactor with fixed target: fspec research --tool=ast --refactor --pattern='const SafeTextInput' --lang=tsx --source=src/tui/components/AgentModal.tsx --target=src/tui/components/SafeTextInput.tsx
  #   3. NAPI usage from TypeScript: const results = await astGrepSearch('function $NAME', 'typescript', ['src/'])
  #   4. Search output format: file:line:column:matched_text (e.g., src/auth.ts:42:1:function login())
  #   5. Refactor with ambiguous pattern shows error: 'Pattern matched 5 nodes. Refactor requires exactly 1 match. Matches found at: line 42, line 156, ...' then user refines pattern to be more specific.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we keep backwards compatibility with the old --operation flag syntax, or is a clean break acceptable?
  #   A: Yes, --extract capability is 100% required in this story. Must be included.
  #
  #   Q: Do we need the --extract capability for code extraction in the initial implementation, or can that be a follow-up story?
  #   A: Yes, --extract capability is 100% required in this story. Must be included.
  #
  #   Q: When refactoring/moving code, should the tool automatically add the import statement to the original file, or just remove the code and let the user handle imports?
  #   A: Only do what ast-grep can deterministically do. Remove matched code from source, create new file with matched code. Do not guess or add imports - user handles that manually.
  #
  #   Q: Should the tool be exposed via NAPI for use by other TypeScript code (like fspec review), or only through the fspec research --tool=ast CLI?
  #   A: Yes, expose via NAPI. Fully written in Rust, just exposed to TypeScript. No TypeScript implementation.
  #
  #   Q: What should the NAPI function names be? e.g. astGrepSearch(), astGrepRefactor()?
  #   A: Yes, use astGrepSearch() and astGrepRefactor(). Also update tool definitions in both facades and tools.
  #
  #   Q: Should we remove the tree-sitter dependencies from package.json as part of this story, or leave that for a separate cleanup task?
  #   A: Yes, remove all 17 tree-sitter dependencies from package.json and remove all deprecated code.
  #
  #   Q: For the pattern syntax, should we use the exact ast-grep syntax (, 93100) or provide any wrapper/simplification?
  #   A: Use exact ast-grep pattern syntax. No wrappers or simplification.
  #
  #   Q: For refactor target file, can we use $NAME from the pattern as a variable in the target path (e.g., --target=src/components/$NAME.tsx), or should it be a fixed filename?
  #   A: Fixed filename. Refactoring should be deliberate - search first to see matches, then refactor one specific match to a specific target file. No variable interpolation.
  #
  #   Q: What should happen if the refactor pattern matches multiple nodes in the source file? Error out, or require an additional selector like line number?
  #   A: Error out with helpful message showing all matches and suggesting more specific pattern. Refactor requires exactly 1 match - no ambiguity.
  #
  #   Q: Any other capabilities or requirements I'm missing for this AST tool?
  #   A: Looks good. Can improve in later iterations or revisit if something is missing.
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to use the ast research tool with pattern-based search via codelet's native ast-grep implementation
    So that I eliminate 17 tree-sitter npm dependencies, get better performance, and gain code extraction capabilities for refactoring large files

  Scenario: Search for functions using pattern matching
    Given a TypeScript file with function declarations
    When I run ast search with pattern 'function $NAME($$$ARGS)' and language 'typescript'
    Then the output contains matches in file:line:column:text format


  Scenario: Refactor moves matched code to new file
    Given a source file containing 'const SafeTextInput' component
    When I run ast refactor with pattern 'const SafeTextInput' from source to target file
    Then the matched code is removed from the source file
    And the matched code is written to the target file


  Scenario: Refactor errors when pattern matches multiple nodes
    Given a source file with multiple const declarations
    When I run ast refactor with pattern 'const $NAME' that matches 5 nodes
    Then an error is returned stating 'Pattern matched 5 nodes. Refactor requires exactly 1 match.'
    And the error lists all match locations


  Scenario: NAPI exports astGrepSearch function
    Given the codelet-napi module is loaded
    When I call astGrepSearch with pattern, language, and paths
    Then the function returns an array of match results


  Scenario: NAPI exports astGrepRefactor function
    Given the codelet-napi module is loaded
    When I call astGrepRefactor with pattern, language, source file, and target file
    Then the function moves the matched code from source to target


  Scenario: Tree-sitter dependencies are removed
    Given the package.json file
    When the migration is complete
    Then no @sengac/tree-sitter packages are listed as dependencies

