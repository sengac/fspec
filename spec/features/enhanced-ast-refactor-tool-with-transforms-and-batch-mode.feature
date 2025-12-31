@code-analysis
@ast-analysis
@refactoring
@tools
@codelet
@TOOLS-004
Feature: Enhanced AST Refactor Tool with Transforms and Batch Mode

  """
  Extends astgrep_refactor tool in codelet/tools/src/astgrep_refactor.rs. Uses ast-grep-language crate for pattern matching and AST manipulation. Transforms implemented inline (substring, replace, convert) with topological sort for dependency ordering. No external transform library - pure Rust implementation. Tool description in rig::tool::Tool definition method must include comprehensive pattern and transform documentation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool description must clearly explain that patterns match PARTIAL code structure, not exact code strings
  #   2. Meta-variable syntax must be documented: $NAME (single capture), $$NAME (unnamed), $_ (dropped), $$$ (multi-node), $$$ARGS (captured multi)
  #   3. Meta-variable names must be UPPERCASE letters, underscores, or digits (after first char)
  #   4. Transform mode must support three transform types: substring (with start/end char), replace (regex), and convert (case conversion)
  #   5. Convert transform must support: lowerCase, upperCase, capitalize, camelCase, snakeCase, kebabCase, pascalCase
  #   6. Transforms can be chained - output of one transform can be input to another via dependency ordering
  #   7. Batch mode (batch: true) allows replacing ALL matches instead of requiring exactly one
  #   8. Batch mode must report the count of replacements made in the result
  #   9. Preview/dry-run mode (preview: true) shows what would be matched/changed without modifying files
  #   10. Preview mode must show all matches with their locations, original code, and proposed replacement
  #   11. Tool description must include concrete examples showing common refactoring patterns
  #   12. Replacement templates can reference captured meta-variables and transformed variables
  #   13. Transforms only apply to replace mode, not extract mode - extract keeps code as-is
  #   14. Batch mode only applies to replace mode, not extract mode - extract is deliberate one-at-a-time
  #   15. Transform errors must fail the whole operation with clear error message - no silent skipping
  #   16. All separator options must be documented: caseChange, underscore, dash, dot, slash, space
  #
  # EXAMPLES:
  #   1. Pattern 'fn $NAME($$$ARGS)' matches any Rust function - $NAME captures the function name, $$$ARGS captures all parameters
  #   2. Pattern 'console.log($MSG)' matches all console.log calls - does NOT require passing the full statement as pattern
  #   3. Transform to rename: pattern 'fn old_name()', transforms {NEW: {convert: {source: old_name, toCase: camelCase}}}, replacement 'fn $NEW()' produces 'fn oldName()'
  #   4. Substring transform: {substring: {source: $NAME, startChar: 0, endChar: -1}} removes last character from captured NAME
  #   5. Replace transform: {replace: {source: $NAME, replace: '_id$', by: ''}} removes '_id' suffix from captured NAME
  #   6. Batch mode with pattern 'oldFunc($$$ARGS)' and replacement 'newFunc($$$ARGS)' changes all 15 call sites in one operation
  #   7. Preview mode returns: [{location: 'file.rs:10:5', original: 'fn old()', replacement: 'fn new()'}] without modifying files
  #   8. Chained transforms: STRIPPED removes suffix via replace, then FINAL converts STRIPPED to camelCase - ordered by dependency
  #   9. Convert separatedBy option: 'user_accountName' with separatedBy: [underscore] splits only on underscore, preserving camelCase within segments
  #
  # ========================================

  Background: User Story
    As an AI agent using AST refactoring for code modifications
    I want to understand patterns clearly, rename symbols with case transforms, replace multiple matches at once, and preview changes before applying
    So that I can perform complex refactoring operations confidently without misunderstanding the tool or making unintended changes

  Scenario: Match function using partial pattern with meta-variables
    Given a Rust source file containing 'fn calculate_sum(a: i32, b: i32) -> i32 { a + b }'
    When I use pattern 'fn $NAME($$$ARGS)' to match the function
    Then the pattern matches and captures $NAME as 'calculate_sum' and $$$ARGS as 'a: i32, b: i32'


  Scenario: Match call expressions without full statement as pattern
    Given a TypeScript source file containing 'const x = 1; console.log(x); const y = 2;'
    When I use pattern 'console.log($MSG)' to find the call
    Then the pattern matches 'console.log(x)' and captures $MSG as 'x'


  Scenario: Rename function using convert transform with case conversion
    Given a Rust source file containing 'fn old_snake_name() { }'
    When I refactor with pattern 'fn $NAME()', transform {NEW: {convert: {source: $NAME, toCase: camelCase}}}, and replacement 'fn $NEW()'
    Then the file is updated to contain 'fn oldSnakeName() { }'


  Scenario: Extract substring from captured variable
    Given a source file with variable name 'userNameInput'
    When I apply transform {SHORT: {substring: {source: $NAME, startChar: 0, endChar: -5}}} to remove 'Input'
    Then the transform produces 'userName' in $SHORT


  Scenario: Remove suffix using replace transform with regex
    Given a source file with function name 'get_user_id'
    When I apply transform {CLEAN: {replace: {source: $NAME, replace: '_id$', by: ''}}}
    Then the transform produces 'get_user' in $CLEAN


  Scenario: Batch replace all matching call sites in one operation
    Given a TypeScript file with 5 calls to 'oldFunc(arg1)', 'oldFunc(arg2)', etc.
    When I refactor with pattern 'oldFunc($$$ARGS)', replacement 'newFunc($$$ARGS)', and batch: true
    Then all 5 call sites are replaced with 'newFunc' and the result reports matches_count: 5


  Scenario: Preview changes without modifying files
    Given a source file containing 'fn old() { }'
    When I refactor with pattern 'fn old()', replacement 'fn new()', and preview: true
    Then the result shows the match location, original code, and proposed replacement without modifying the file


  Scenario: Chain multiple transforms with dependency ordering
    Given a source file with function name 'get_user_impl'
    When I apply transforms {STRIPPED: {replace: {source: $NAME, replace: '_impl$', by: ''}}, FINAL: {convert: {source: $STRIPPED, toCase: pascalCase}}}
    Then STRIPPED is computed first as 'get_user', then FINAL is computed as 'GetUser'


  Scenario: Use separatedBy option to control word splitting in case conversion
    Given a variable name 'user_accountName' with mixed naming conventions
    When I apply transform {RESULT: {convert: {source: $NAME, toCase: snakeCase, separatedBy: [underscore]}}}
    Then the result is 'user_accountname' preserving the underscore boundary but not splitting on case change


  Scenario: Fail operation when transform has invalid regex
    Given a refactor operation with transform {BAD: {replace: {source: $NAME, replace: '[unclosed', by: ''}}}
    When I execute the refactor operation
    Then the operation fails with an error message indicating invalid regex in the transform


  Scenario: Reject transforms when using extract mode
    Given a refactor operation with target_file set and transforms specified
    When I execute the refactor operation
    Then the operation fails with an error message indicating transforms are not supported in extract mode


  Scenario: Reject batch mode when using extract mode
    Given a refactor operation with target_file set and batch: true
    When I execute the refactor operation
    Then the operation fails with an error message indicating batch mode is not supported in extract mode


  Scenario: Fail operation when transforms have cyclic dependency
    Given a refactor operation with transforms {A: {replace: {source: $B, replace: 'x', by: 'y'}}, B: {replace: {source: $A, replace: 'a', by: 'b'}}}
    When I execute the refactor operation
    Then the operation fails with an error message indicating cyclic dependency between transforms

