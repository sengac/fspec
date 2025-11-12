@parser
@language-support
@research-tools
@RES-019
Feature: Add multi-language support to AST research tool (Kotlin, Dart, Python, Go, Rust, Swift, C#, C, C++)

  """
  Extends existing AST research tool with tree-sitter parsers for multiple languages. Each language requires: (1) npm package installation, (2) import statement, (3) parser initialization in ast.ts, (4) language detection in detectLanguage() function, (5) query-executor updates for language-specific node types. Python, Go, Rust already have detection but lack parser implementation. New languages (Swift, C#, C, C++) need full implementation from scratch.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST tool must support tree-sitter-kotlin for Kotlin language parsing
  #   2. AST tool must support tree-sitter-dart for Dart language parsing
  #   3. Language detection must identify .kt and .kts files as Kotlin
  #   4. Language detection must identify .dart files as Dart
  #   5. AST tool must support tree-sitter parsers for Python, Go, Rust, Swift, C#, C, and C++
  #   6. Language detection must identify file extensions: .py (Python), .go (Go), .rs (Rust), .swift (Swift), .cs (C#), .c/.h (C), .cpp/.hpp/.cc/.cxx (C++)
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --operation=list-functions --file=MyClass.kt' and gets list of Kotlin functions
  #   2. User runs 'fspec research --tool=ast --operation=list-classes --file=MyWidget.dart' and gets list of Dart classes
  #   3. User runs 'fspec research --tool=ast --operation=list-functions --file=app.dart' and gets Flutter/Dart functions
  #   4. User runs 'fspec research --tool=ast --operation=list-functions --file=main.py' and gets list of Python functions
  #   5. User runs 'fspec research --tool=ast --operation=list-functions --file=main.go' and gets list of Go functions
  #   6. User runs 'fspec research --tool=ast --operation=list-functions --file=main.rs' and gets list of Rust functions
  #   7. User runs 'fspec research --tool=ast --operation=list-functions --file=App.swift' and gets list of Swift functions
  #   8. User runs 'fspec research --tool=ast --operation=list-functions --file=Program.cs' and gets list of C# methods
  #   9. User runs 'fspec research --tool=ast --operation=list-functions --file=main.c' and gets list of C functions
  #   10. User runs 'fspec research --tool=ast --operation=list-functions --file=main.cpp' and gets list of C++ functions
  #
  # ========================================

  Background: User Story
    As a developer using AST research tool
    I want to analyze codebases in multiple programming languages
    So that I can use fspec research commands across polyglot projects

  Scenario: Parse Python file and list functions
    Given a Python file "main.py" exists with function definitions
    And tree-sitter-python parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=main.py"
    Then the output should contain a list of Python functions
    And the output should include function names and line numbers

  Scenario: Parse Go file and list functions
    Given a Go file "main.go" exists with function definitions
    And tree-sitter-go parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=main.go"
    Then the output should contain a list of Go functions
    And the output should include function names and line numbers

  Scenario: Parse Rust file and list functions
    Given a Rust file "main.rs" exists with function definitions
    And tree-sitter-rust parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=main.rs"
    Then the output should contain a list of Rust functions
    And the output should include function names and line numbers

  Scenario: Parse Swift file and list functions
    Given a Swift file "App.swift" exists with function definitions
    And tree-sitter-swift parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=App.swift"
    Then the output should contain a list of Swift functions
    And the output should include function names and line numbers

  Scenario: Parse C# file and list methods
    Given a C# file "Program.cs" exists with method definitions
    And tree-sitter-c-sharp parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=Program.cs"
    Then the output should contain a list of C# methods
    And the output should include method names and line numbers

  Scenario: Parse C file and list functions
    Given a C file "main.c" exists with function definitions
    And tree-sitter-c parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=main.c"
    Then the output should contain a list of C functions
    And the output should include function names and line numbers

  Scenario: Parse C++ file and list functions
    Given a C++ file "main.cpp" exists with function definitions
    And tree-sitter-cpp parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=main.cpp"
    Then the output should contain a list of C++ functions
    And the output should include function names and line numbers
