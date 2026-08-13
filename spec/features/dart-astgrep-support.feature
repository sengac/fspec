@KGRAPH-056
Feature: Add Dart language support to AstGrep and AstGrepRefactor tools via tree-sitter-dart 0.1.0
  """
  Create a LanguageChoice enum to unify SupportLang variants with custom DartLang — both tools dispatch through this enum. The DartLang struct lives in a new rust/tools/src/dart_lang.rs module shared by astgrep.rs and astgrep_refactor.rs.
  The LLM tool description JSON schema (TypeScript side) must also add 'dart' to the language enum so models know they can request Dart searches.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. tree-sitter-dart v0.1.0 must be added as a direct dependency to rust/tools since ast-grep upstream removed builtin Dart support in v0.30.0
  #   2. A custom DartLang struct must implement ast-grep-core's Language trait — no expando_char needed since $ is valid in Dart identifiers (same as Java/JS/Bash in ast-grep)
  #   3. Both AstGrepTool and AstGrepRefactorTool must recognize 'dart' as a language string and dispatch to the custom DartLang
  #   4. The get_extensions() function must map Dart to ["dart"] so directory-walking file search finds .dart files
  #   5. Error messages and supported_languages() lists must include 'dart' so the LLM knows Dart is available
  #   6. Missing Solidity, Nix, and Hcl entries must be added to get_extensions() since they exist in SupportLang but have no file extension mapping
  #   7. Pattern matching with $VAR meta-variables works natively for Dart syntax since $ is a valid identifier character — no expando_char substitution needed
  #   8. Dart's tree-sitter grammar splits top-level function declarations into sibling nodes (function_signature + function_body), so full-function patterns like 'void $NAME() { $$$BODY }' fail — match signatures only or use class-level patterns
  #
  # EXAMPLES:
  #   1. Searching a directory of .dart files with AstGrepTool for 'class $NAME { $$$BODY }' returns matching Dart classes with file, line, column
  #   2. Passing language='dart' to AstGrepRefactorTool replaces a matched Dart pattern in a source file (e.g., rename a function)
  #   3. Passing an unsupported language like 'brainfuck' returns an error message listing all supported languages including dart
  #   4. Searching for 'void $NAME($$$PARAMS)' (signature only) in a Dart file matches function signatures with correct meta-variable captures
  #   5. AstGrepRefactorTool batch mode replaces all occurrences of a Dart pattern across a file
  #
  # ========================================
  Background: User Story
    As a developer
    I want to search and refactor Dart code using AstGrep and AstGrepRefactor tools
    So that Flutter/Dart projects get the same AST-based code intelligence as all other supported languages

  Scenario: Search Dart files for class declarations using AstGrepTool
    Given a directory containing .dart files with class declarations
    When I search with AstGrepTool using pattern 'class $NAME { $$$BODY }' and language 'dart'
    Then the results contain matching Dart classes with file path, line number, and column
    And the meta-variable $NAME captures the class name

  Scenario: Search Dart files for function declarations with meta-variable capture
    Given a .dart file containing top-level functions and class methods
    When I search with AstGrepTool using pattern 'void $NAME($$$PARAMS)' and language 'dart'
    Then the results contain matches for each void function signature
    And the meta-variable $NAME captures each function name correctly

  Scenario: Refactor a Dart source file by replacing a matched class pattern
    Given a .dart source file containing a class named 'OldService'
    When I use AstGrepRefactorTool with language 'dart' to replace 'class OldService { $$$BODY }' with 'class NewService { $$$BODY }'
    Then the source file is updated with the class renamed to 'NewService'
    And the class body is preserved unchanged

  Scenario: Batch replace all occurrences of a Dart class pattern
    Given a .dart source file containing multiple class declarations with the same field
    When I use AstGrepRefactorTool in batch mode to replace 'class $NAME { $$$BODY }' with 'class $NAME { int id; }' for language 'dart'
    Then all class bodies are replaced with the new field
    And the class names are preserved in each replacement

  Scenario: Unsupported language error message includes dart in supported list
    When I search with AstGrepTool using language 'brainfuck'
    Then the error message lists all supported languages
    And the supported languages list includes 'dart'

  Scenario: AstGrepTool finds .dart files when walking a directory
    Given a directory containing mixed files including .dart, .ts, and .rs files
    When I search with AstGrepTool using a Dart pattern and language 'dart'
    Then only .dart files are searched
    And .ts and .rs files are not included in results

  Scenario: Solidity file extensions are mapped correctly
    Given a directory containing .sol and .ts files
    When I search with AstGrepTool using language 'solidity'
    Then only .sol files are searched

  Scenario: Nix file extensions are mapped correctly
    Given a directory containing .nix files
    When I search with AstGrepTool using language 'nix'
    Then only .nix files are searched

  Scenario: Hcl file extensions are mapped correctly
    Given a directory containing .hcl and .tf files
    When I search with AstGrepTool using language 'hcl'
    Then .hcl and .tf files are searched
