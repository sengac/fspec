@KGRAPH-055
Feature: Python and Java ast_index crashes on real repos — imported classes create dangling Function slug references
  """
  edge_helpers.rs:285 — strip leading underscores from original_name before first_char.is_uppercase() check
  helpers.rs extract_name_after_keyword — skip // line comments and /* */ block comments before searching for keyword
  mod.rs deduplicate_entities — build typed slug set HashMap<String, HashSet<String>> (slug → set of node_types) and validate edge endpoints match schema expected types
  Tests MUST use fixtures with: (1) class names starting with underscore, (2) comments containing 'class ' keyword before the actual declaration, (3) mixed function+class imports in one file
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. resolve_calls uses first_char.is_uppercase() to decide Calls vs TypeRef for imported names, but names starting with underscore fail the check
  #   2. extract_name_after_keyword finds the keyword in COMMENTS before the actual declaration
  #   3. deduplicate_entities edge pruning checks slug existence but NOT node type
  #   4. resolve_calls must strip leading underscores before the uppercase check
  #   5. extract_name_after_keyword must not match keywords inside comments
  #   6. deduplicate_entities should validate that to_slug node type matches schema-expected target type
  #   7. After fixes, ast_index on tmp/python-click must complete without errors
  #   8. After fixes, ast_index on tmp/java-gson must complete without errors
  #   9. Go (cobra) and PHP (slim) must not regress
  #
  # EXAMPLES:
  #   1. Python _OptionParser: underscore-prefixed class imported and called as constructor → should produce TypeRef, not crash
  #   2. Java JsonIOException: class with comment containing 'class ' keyword before declaration → wrong name extracted
  #   3. Python mixed: imports function parse_args and class Config → parse_args gets Calls, Config gets TypeRef
  #   4. Java class with block comment: `public /*helper class*/ class MyHelper {}` → must find declaration, not comment
  #   5. Dedup edge pruning: Calls edge to slug that only exists as Type → pruned, not crash
  #
  # ========================================
  Background: User Story
    As a developer running ast_index on real-world repos
    I want to index Python and Java projects without crashes
    So that I can use ast_dead_code on these languages like I can with Go and PHP

  Scenario: Python underscore-prefixed class imported and called as constructor produces TypeRef
    Given a Python file "src/app/parser.py" defining class "_OptionParser" and function "split_args"
    And a Python file "src/app/core.py" with "from app.parser import _OptionParser, split_args"
    And "src/app/core.py" has function "make_parser" that calls "_OptionParser(ctx)" and "split_args()"
    When I extract entities from both files with known_files containing both paths
    Then a TypeRef edge should exist from "make_parser" to "_OptionParser"
    And a Calls edge should exist from "make_parser" to "split_args"
    And no Calls edge should target "_OptionParser"

  Scenario: Java class with comment containing keyword before declaration extracts correct name
    Given a Java file "com/myapp/MyException.java" with content:
      """
      package com.myapp;
      // This is a class for custom exceptions
      @SuppressWarnings("MemberName") // class name is part of the public API
      public final class MyException extends RuntimeException {
          public MyException(String msg) { super(msg); }
      }
      """
    When I extract entities from the Java file
    Then a Type node should exist with name "MyException"
    And no Type node should exist with name "name"
    And no Type node should exist with name "MemberName"

  Scenario: Java imported class used in constructor produces TypeRef not crash
    Given a Java file "com/myapp/MyException.java" defining class "MyException"
    And a Java file "com/myapp/Service.java" with "import com.myapp.MyException;"
    And "com/myapp/Service.java" has method "doWork" that calls "new MyException(msg)"
    When I extract entities from both files with known_files containing both paths
    Then a TypeRef edge should exist from "doWork" to "MyException"
    And no Calls edge should target "MyException" as a Function
    And indexing should complete without errors

  Scenario: Python mixed imports — function gets Calls edge, class gets TypeRef edge
    Given a Python file "src/app/utils.py" defining function "parse_args" and class "Config"
    And a Python file "src/app/main.py" with "from app.utils import parse_args, Config"
    And "src/app/main.py" has function "run" that calls "parse_args()" and "Config()"
    When I extract entities from both files with known_files containing both paths
    Then a Calls edge should exist from "run" to "parse_args"
    And a TypeRef edge should exist from "run" to "Config"

  Scenario: Deduplicate prunes Calls edge targeting a Type-only slug
    Given extracted entities containing a Type node with slug "file-a::MyClass"
    And no Function node exists with slug "file-a::MyClass"
    And a Calls edge from "file-b::caller" to "file-a::MyClass"
    When I run deduplicate_entities on the entity list
    Then the Calls edge from "file-b::caller" to "file-a::MyClass" should be pruned
    And no Calls edge should remain targeting "file-a::MyClass"

  Scenario: Python stdlib import does not create edges or crash
    Given a Python file "src/app/core.py" with "from os import path"
    And "src/app/core.py" has function "resolve" that calls "path.join(a, b)"
    When I extract entities with known_files NOT containing any "os" path
    Then no Imports edge should exist for the "os" import
    And indexing should complete without errors
