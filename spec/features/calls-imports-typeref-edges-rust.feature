@KGRAPH-045
Feature: Calls/Imports/TypeRef edges — Rust

  """
  Follows PHP extractor pattern: extract_functions returns HashSet, extract_imports returns
  import_map, then extract_calls and extract_type_refs use shared edge_helpers.
  Rust uses `use crate::`, `use super::`, and `mod` for local imports.
  """

  # EXAMPLE MAPPING CONTEXT
  # BUSINESS RULES:
  #   1. Rust extractor must emit Imports edges for `use crate::` / `use super::` / `mod` resolved to known_files
  #   2. Rust extractor must emit Calls edges from function bodies resolved against same-file functions
  #   3. Rust extractor must emit TypeRef edges from `: Type` and `-> Type` in signatures against local types
  #   4. External crate imports must NOT generate edges

  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from Rust source files
    So that dead code detection works for Rust projects via ast_dead_code

  Scenario: Extract Imports edges from Rust use statements
    Given a Rust file with `use crate::graph::helpers;`
    And the target file `graph/helpers.rs` exists in the project
    When the Rust extractor processes the source file
    Then an Imports edge should be emitted from the source file to `graph-helpers-rs`
    And external `use serde_json::Value` imports should NOT produce edges

  Scenario: Extract Calls edges from Rust function calls
    Given a Rust file with function `extract()` that calls `slugify_path()`
    And `slugify_path` is defined in the same file
    When the Rust extractor processes the source file
    Then a Calls edge should be emitted from `extract` to `slugify_path`

  Scenario: Extract TypeRef edges from Rust type annotations
    Given a Rust file with `fn extract(source: &str) -> Vec<GraphEntity>`
    And type `GraphEntity` is defined in the same file
    When the Rust extractor processes the source file
    Then a TypeRef edge should be emitted from `extract` to `GraphEntity`
