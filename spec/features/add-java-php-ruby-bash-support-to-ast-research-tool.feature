@high
@language-support
@research-tools
@parser
@RES-020
Feature: Add Java, PHP, Ruby, Bash support to AST research tool

  """
  Architecture notes:
  - Extends existing AST research tool (src/research-tools/ast.ts) with 4 additional tree-sitter parsers
  - Uses tree-sitter-java (v0.23.5), tree-sitter-php (v0.24.2), tree-sitter-ruby (v0.23.1), tree-sitter-bash (v0.25.0)
  - Parsers already installed in package.json, only need integration into research tool
  - Language detection in detectLanguage() function must map file extensions: .java, .php, .rb, .sh, .bash
  - Parser initialization follows existing pattern used for other languages (Python, Go, Rust, etc.)
  - Query executor (src/utils/query-executor.ts) must handle language-specific AST node types:
    * Java: method_declaration, class_declaration
    * PHP: function_definition, method_declaration
    * Ruby: method, def, class
    * Bash: function_definition
  - Help documentation (src/help.ts) must be updated to list all research tools
  - AST tool help must reflect total of 15 supported languages
  - All parsers externalized in vite.config.ts (already done for java, php, ruby, bash)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AST tool must support tree-sitter-java for Java language parsing (.java files)
  #   2. AST tool must support tree-sitter-php for PHP language parsing (.php files)
  #   3. AST tool must support tree-sitter-ruby for Ruby language parsing (.rb files)
  #   4. AST tool must support tree-sitter-bash for Bash script parsing (.sh, .bash files)
  #   5. All parsers must be properly imported and initialized in src/research-tools/ast.ts
  #   6. Language detection must identify file extensions for all 4 new languages
  #   7. Query executor must handle language-specific AST node types for each language
  #   8. Help documentation in src/help.ts must list all research tools and their capabilities
  #   9. AST tool help must document all 15 supported languages
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --operation=list-functions --file=Example.java' and gets list of Java methods
  #   2. User runs 'fspec research --tool=ast --operation=list-functions --file=example.php' and gets list of PHP functions
  #   3. User runs 'fspec research --tool=ast --operation=list-functions --file=example.rb' and gets list of Ruby methods
  #   4. User runs 'fspec research --tool=ast --operation=list-functions --file=script.sh' and gets list of Bash functions
  #   5. User runs 'fspec research --tool=ast --help' and sees all 15 supported languages documented
  #   6. User runs 'fspec research' and sees AST tool listed with correct language support count
  #   7. Developer checks src/help.ts and finds complete research tools documentation
  #
  # ========================================

  Background: User Story
    As a developer using AST research tool
    I want to analyze codebases in Java, PHP, Ruby, and Bash
    So that I can use fspec research tool with all major programming languages in my projects

  Scenario: Parse Java file and list methods
    Given a Java file "Example.java" exists with method definitions
    And tree-sitter-java parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=Example.java"
    Then the output should contain a list of Java methods
    And the output should include method names and line numbers

  Scenario: Parse PHP file and list functions
    Given a PHP file "example.php" exists with function definitions
    And tree-sitter-php parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=example.php"
    Then the output should contain a list of PHP functions
    And the output should include function names and line numbers

  Scenario: Parse Ruby file and list methods
    Given a Ruby file "example.rb" exists with method definitions
    And tree-sitter-ruby parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=example.rb"
    Then the output should contain a list of Ruby methods
    And the output should include method names and line numbers

  Scenario: Parse Bash script and list functions
    Given a Bash script "script.sh" exists with function definitions
    And tree-sitter-bash parser is installed
    When I run "fspec research --tool=ast --operation=list-functions --file=script.sh"
    Then the output should contain a list of Bash functions
    And the output should include function names and line numbers

  Scenario: AST tool help documents all 15 languages
    When I run "fspec research --tool=ast --help"
    Then the output should list all 15 supported languages
    And the language list should include JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash

  Scenario: Research command lists AST tool with correct capabilities
    When I run "fspec research"
    Then the output should list the AST research tool
    And the AST tool description should mention multi-language support
    And the tool should indicate 15 languages are supported

  Scenario: Help file documents all research tools
    Given I check the file "src/help.ts"
    Then it should contain documentation for all research tools
    And it should list the AST tool with complete language support information
    And it should include usage examples for research tools
