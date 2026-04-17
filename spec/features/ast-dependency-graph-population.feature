@KGRAPH-018
Feature: AST Dependency Graph Population
  """
  Dependency parsers in codelet/napi/src/graph/ast_pipeline/ — npm_dep_extractor.rs and cargo_dep_extractor.rs. Uses serde_json for package.json and toml crate for Cargo.toml parsing. Produces GraphEntity values reusing helpers.rs from KGRAPH-017.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Parse package.json dependencies and devDependencies into Dependency nodes with name, version, and isDev properties
  #   2. Parse Cargo.toml [dependencies] and [dev-dependencies] into Dependency nodes with name, version, and isDev properties
  #   3. Create DependsOn edges from the project File node (package.json or Cargo.toml) to each Dependency node
  #   4. Each dependency parser (npm, cargo) must be in its own file for separation of concerns
  #   5. Must produce GraphEntity values compatible with the AST graph database load_entities/load_jsonl
  #   6. Dependency nodes must have a @key slug in format 'dep::<package-name>' for upsert semantics
  #   7. Must handle workspace Cargo.toml files (with [workspace.members]) by scanning member crate Cargo.toml files
  #
  # EXAMPLES:
  #   1. Parse a package.json with both dependencies and devDependencies, creating Dependency nodes with name, version, isDev=false for deps and isDev=true for devDeps, plus DependsOn edges from the project root
  #   2. Parse a Cargo.toml with [dependencies] and [dev-dependencies] sections, creating Dependency nodes for each crate with correct version constraints and isDev flags
  #   3. Parse a Cargo workspace with multiple member crates, combining all dependency information into the graph with crate-scoped DependsOn edges
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to have package.json and Cargo.toml dependencies parsed into the AST graph as Dependency nodes with DependsOn edges
    So that I can query which external packages are used, find unused dependencies, and understand which modules depend on which external libraries

  Scenario: Parse package.json dependencies into Dependency nodes
    Given a project directory with a package.json containing dependencies and devDependencies
    When the npm dependency extractor parses the package.json
    Then Dependency nodes should be created for each dependency with name, version, and isDev=false
    And Dependency nodes should be created for each devDependency with name, version, and isDev=true
    And each Dependency node should have a slug in the format "dep::<package-name>"
    And each Dependency node should have source "npm"
    And DependsOn edges should link the package.json File node to each Dependency

  Scenario: Parse Cargo.toml dependencies into Dependency nodes
    Given a project directory with a Cargo.toml containing dependencies and dev-dependencies
    When the cargo dependency extractor parses the Cargo.toml
    Then Dependency nodes should be created for each dependency with correct version constraints
    And Dependency nodes should be created for each dev-dependency with isDev=true
    And each Dependency node should have source "crate"
    And DependsOn edges should link the Cargo.toml File node to each Dependency

  Scenario: Parse Cargo workspace with multiple member crates
    Given a Cargo workspace with a root Cargo.toml listing member crates
    And each member crate has its own Cargo.toml with dependencies
    When the cargo dependency extractor parses the workspace
    Then Dependency nodes should be created from all member crate Cargo.toml files
    And DependsOn edges should link each member crate's Cargo.toml to its dependencies
    And workspace-level dependencies should be included
