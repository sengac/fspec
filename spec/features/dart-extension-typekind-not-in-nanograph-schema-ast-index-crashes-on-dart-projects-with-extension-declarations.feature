@KGRAPH-070
Feature: Dart extension typeKind not in nanograph schema — ast_index crashes on Dart projects with extension declarations
  """
  Single-line fix: add 'extension' to typeKind enum in rust/napi/schemas/ast-code.pg. No extractor or dispatch changes needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The nanograph schema typeKind enum must include 'extension' as a valid value
  #   2. Only the schema needs updating — the extractor (ast_dart_extractor.rs) and dispatch filter (ast_dispatch.rs) already handle extension correctly
  #
  # EXAMPLES:
  #   1. ast_index on a Dart project with extension declarations succeeds after adding extension to typeKind enum in ast-code.pg
  #   2. ast_index on the fspec codebase (which contains Rust files including the Dart extractor) still succeeds — no regression
  #
  # ========================================
  Background: User Story
    As a developer
    I want to index a Dart project with extension declarations via ast_index
    So that the graph loads successfully without schema violations

  Scenario: ast_index succeeds on Dart project with extension declarations
    Given a Dart project that contains extension declarations
    When I run ast_index on the project directory
    Then the index completes without schema violation errors
    Then the extension types are stored with typeKind extension in the graph

  Scenario: Non-Dart indexing is not affected
    Given a project with no Dart files
    When I run ast_index on the project directory
    Then the index completes successfully with no errors
