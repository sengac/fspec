@done
@KGRAPH-063
Feature: Source Code and Metadata Storage in Graph Nodes
  """
  DRY approach: Create a single metadata.rs module under ast_pipeline/ with data-driven
  per-language configs for docstring patterns, decorator patterns, and parameter extraction.
  Each language extractor passes raw AST source text to shared helpers rather than implementing
  extraction logic. Same pattern as complexity.rs. Extend build_function_node() and
  build_type_node() signatures to force all 14 extractors through compile error.
  """

  Background: User Story
    As an AI agent
    I want to retrieve function/type source code, docstrings, parameters, decorators, and language directly from graph nodes
    So that I can understand code without making separate Read tool calls for every function I find

  Scenario: Function nodes include metadata after indexing
    Given a TypeScript project has been indexed with ast_index
    When I search for a function using ast_search
    Then the result includes parameters as comma-separated names
    And the result includes the function source code
    And the result includes the extracted JSDoc docstring
    And the result includes decorators as comma-separated list
    And the result includes the language identifier "typescript"

  Scenario: Type nodes include line numbers and metadata after indexing
    Given a Python project has been indexed with ast_index
    When I search for a type using ast_search with entity_type "Type"
    Then the result includes lineStart and lineEnd properties
    And the result includes the extracted docstring
    And the result includes decorators as comma-separated list
    And the result includes the language identifier "python"

  Scenario: Source code is capped at 100 lines or 4096 bytes
    Given a project contains a function with more than 100 lines
    When the project is indexed with ast_index
    Then the function node source is truncated to at most 100 lines or 4096 bytes
    And the function node has truncated set to true

  Scenario: Short function source is stored in full
    Given a project contains a function with fewer than 100 lines and under 4096 bytes
    When the project is indexed with ast_index
    Then the function node source contains the complete function body
    And the function node has truncated set to false

  Scenario: Docstring extraction uses language-specific patterns
    Given a project has functions with language-specific doc comments
    When the project is indexed with ast_index
    Then JSDoc comments are extracted for TypeScript functions
    And rustdoc comments are extracted for Rust functions
    And triple-quoted docstrings are extracted for Python functions

  Scenario: Parameter names extracted without types
    Given a project has functions with typed parameters
    When the project is indexed with ast_index
    Then parameter names are stored as comma-separated string without types
    And language-specific self parameters are filtered appropriately

  Scenario: Decorator extraction uses language-specific patterns
    Given a project has functions and types with decorators or annotations
    When the project is indexed with ast_index
    Then Python @decorator syntax is captured
    And Rust #[attribute] syntax is captured
    And Java @Annotation syntax is captured

  Scenario: Function with no metadata has empty strings
    Given a project has a plain function with no decorators or docstring
    When the project is indexed with ast_index
    Then the function node has empty strings for decorators and docstring
    And the function node still has parameters and source populated
