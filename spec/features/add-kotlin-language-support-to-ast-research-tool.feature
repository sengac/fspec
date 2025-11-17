@parser
@research-tools
@high
@research
@ast
@language-support
@kotlin
Feature: Add Kotlin language support to AST research tool
  """
  Extends existing AST research tool (src/research-tools/ast.ts) with tree-sitter-kotlin parser. Requires npm package: tree-sitter-kotlin. Language detection in detectLanguage() function must map .kt/.kts to kotlin. Parser initialization follows existing pattern used for JavaScript, TypeScript, Python, Go, Rust.

  Note: Dart support was removed due to tree-sitter-dart package compatibility issues with tree-sitter@0.25.0 (package built for tree-sitter@0.21.0, incompatible binding format).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST tool must support tree-sitter-kotlin for Kotlin language parsing
  #   2. Language detection must identify .kt and .kts files as Kotlin
  #   3. Parser must correctly parse Kotlin function declarations
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --operation=list-functions --file=MyClass.kt' and gets list of Kotlin functions
  #
  # ========================================
  Background: User Story
    As a developer using AST research tool
    I want to analyze Kotlin codebases
    So that I can use fspec research commands with projects written in Kotlin

  Scenario: Parse Kotlin file and list functions
    Given a Kotlin file "MyClass.kt" exists with function definitions
    And tree-sitter-kotlin parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=MyClass.kt"
    Then the output should contain a list of Kotlin functions
    And the output should include function names and line numbers
