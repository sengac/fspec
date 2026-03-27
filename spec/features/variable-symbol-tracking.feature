@done
@KGRAPH-066
Feature: Variable and Symbol Tracking

  """
  Extend ast-code.pg schema with Variable node type and ContainsVariable edge type.
  Add build_variable_node() and build_contains_variable_edge() to helpers.rs.
  Variable extraction per-language via ast-grep patterns in variables.rs for module/class-level declarations.
  Add Variable to EntityType enum in types.rs and dispatch_ast_search in ast_dispatch.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Variable is a new first-class node type in the AST graph schema
  #   2. Extraction scoped to module-level and class-level declarations only
  #   3. ast_search with entity_type=Variable returns variable results (DRY)
  #   4. Variable nodes have: slug, name, path, lineStart, value, scope, scopeName, isConstant, language
  #   5. ContainsVariable edge connects File → Variable
  #   6. isConstant for: const/final, static readonly, ALL_CAPS (Python/Go/Ruby convention), Rust const
  #   7. Variable extraction reuses existing ast-grep infrastructure via variables.rs
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to search for variables and constants by name across all indexed files
    So that I can locate configuration values, feature flags, module exports, and understand where key symbols are defined

  Scenario: TypeScript module-level const declarations extracted as Variables
    Given a project with a TypeScript file containing module-level const declarations
    When the project is indexed with ast_index
    Then ast_search with entity_type Variable returns the const declarations
    And each Variable has isConstant true and scope module

  Scenario: Python module-level variables extracted while function-local excluded
    Given a project with a Python file containing module-level assignments and function-local variables
    When the project is indexed with ast_index
    Then ast_search with entity_type Variable returns only the module-level variables
    And function-local variables are not included in the results

  Scenario: Rust const and static declarations extracted as Variables
    Given a project with a Rust file containing const and static declarations
    When the project is indexed with ast_index
    Then ast_search with entity_type Variable returns both const and static items
    And the const declaration has isConstant true
    And the static declaration has isConstant false

  Scenario: Java class-level static fields extracted as Variables
    Given a project with a Java file containing a class with static final fields
    When the project is indexed with ast_index
    Then ast_search with entity_type Variable returns the static fields
    And each Variable has scope class and scopeName matching the class name

  Scenario: Search variables by name pattern across languages
    Given a project with multiple files containing variables with API in their names
    When ast_search is called with query API and entity_type Variable
    Then all variables matching the name pattern are returned
    And results include variables from different languages

  Scenario: ast_stats includes variable count after indexing
    Given a project with files containing module-level variables
    When the project is indexed with ast_index
    Then ast_stats shows the total variable count alongside function and type counts
