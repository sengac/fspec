@done
@KGRAPH-054
Feature: Fix AST extractor edge quality gaps — Python/Java/Go missing Imports edges, Go missing method-body Calls, Go missing TypeRef edges

  """
  Uses tree-sitter AST queries in Rust extractors — each extractor is in codelet/napi/src/ast_{lang}_extractor.rs
  File→File Imports edges require resolving module/package names to actual file paths using the known_files list passed to each extractor
  PHP extractor (ast_php_extractor.rs) is the gold standard — all 3 edge types working. Use as reference pattern for fixes.
  Tests in codelet/napi/tests/ast_dead_code_test.rs and language-specific extractor tests
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Python import/from-import statements must produce File→File Imports edges
  #   2. Java import declarations must produce File→File Imports edges
  #   3. Go cross-package import statements must produce File→File Imports edges
  #   4. Go files in the same package must have implicit Imports edges to represent shared visibility
  #   5. Go function calls inside method receiver bodies must produce Calls edges
  #   6. Go type references in function parameters, struct fields, and variable declarations must produce TypeRef edges
  #   7. Python type annotations in function signatures must produce TypeRef edges
  #   8. Existing PHP edge extraction must not regress (PHP is the gold standard with all 3 edge types working)
  #
  # EXAMPLES:
  #   1. Python click repo: `from click.core import Command` in __init__.py creates Imports edge from __init__.py → core.py
  #   2. Python click repo: `import os` does NOT create an Imports edge (stdlib, not in known_files)
  #   3. Java gson repo: `import com.google.gson.Gson` creates Imports edge from importing file → Gson.java
  #   4. Go cobra repo: command.go and completions.go share `package cobra` → implicit Imports edges between them
  #   5. Go cobra repo: `stripFlags()` called from `Find()` method body → Calls edge from Find→stripFlags exists
  #   6. Go cobra repo: `func (c *Command) Find(args []string)` parameter → TypeRef edge to Command struct
  #   7. Python click repo: `def echo(message: str) -> None` → TypeRef edges for str and None (if they resolve to known types)
  #   8. PHP slim repo: after fixes, still shows 9 orphan files (HttpTooManyRequestsException etc) — no regression
  #
  # ========================================

  Background: User Story
    As a developer using ast_dead_code
    I want to get accurate dead code detection across Python, Java, and Go projects
    So that I can trust the results without manual verification of false positives

  Scenario: Python from-import statements produce Imports edges
    Given a Python project with files "src/click/__init__.py" and "src/click/core.py"
    And "src/click/__init__.py" contains "from click.core import Command"
    When I run the Python AST extractor with the known_files set
    Then an Imports edge should exist from "__init__.py" to "core.py"
    And the import_map should contain "Command" mapped to the core.py file slug

  Scenario: Python stdlib imports do not produce Imports edges
    Given a Python project with file "src/click/core.py"
    And "src/click/core.py" contains "import os"
    When I run the Python AST extractor with the known_files set
    Then no Imports edge should exist for the "os" import
    And the Imports edge count should be 0

  Scenario: Java import declarations produce Imports edges
    Given a Java project with files "com/myapp/service/UserService.java" and "com/myapp/App.java"
    And "com/myapp/App.java" contains "import com.myapp.service.UserService;"
    When I run the Java AST extractor with the known_files set
    Then an Imports edge should exist from "App.java" to "UserService.java"
    And the import_map should contain "UserService" mapped to the UserService.java file slug

  Scenario: Go same-package files have implicit Imports edges
    Given a Go project with files "command.go" and "completions.go" both declaring "package cobra"
    When I run the Go AST extractor with the known_files set
    Then implicit Imports edges should connect "command.go" and "completions.go"
    And neither file should appear as an orphan in dead code detection

  Scenario: Go method receiver bodies produce Calls edges
    Given a Go file "command.go" with a method "func (c *Command) Find(args []string)" that calls "stripFlags(args, c)"
    And "stripFlags" is a package-level function in "command.go"
    When I run the Go AST extractor
    Then a Calls edge should exist from "Find" to "stripFlags"
    And "stripFlags" should not appear in dead code uncalled functions

  Scenario: Go type references in function parameters produce TypeRef edges
    Given a Go file "command.go" with a function "func Find(cmd *Command) error"
    And "Command" is a struct declared in the same file
    When I run the Go AST extractor
    Then a TypeRef edge should exist from "Find" to "Command"
    And "Command" should not appear in dead code unreferenced types

  Scenario: Python type annotations produce TypeRef edges
    Given a Python file "core.py" with a function "def process(ctx: Context) -> None"
    And "Context" is a class declared in the same file
    When I run the Python AST extractor
    Then a TypeRef edge should exist from "process" to "Context"

  Scenario: Go cross-package import statements produce Imports edges
    Given a Go project with files "main.go" and "utils/helpers.go" in separate packages
    And "main.go" imports the local "./utils" package
    When I run the Go AST extractor with the known_files set
    Then an Imports edge should exist from "main.go" to the utils package file
    And the import_map should contain the package name for cross-file call resolution

  Scenario: Java imports with external-looking prefix resolve when file exists in project
    Given a Java project structured like gson with files "com/google/gson/Gson.java" and "com/google/gson/GsonBuilder.java"
    And "com/google/gson/GsonBuilder.java" contains "import com.google.gson.Gson;"
    When I run the Java AST extractor with the known_files set
    Then an Imports edge should exist from "GsonBuilder.java" to "Gson.java"

