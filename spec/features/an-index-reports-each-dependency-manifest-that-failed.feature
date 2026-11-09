@done
@parser
@code-analysis
@rust
@BUG-681
Feature: Code graph dependency-extractor failures are only logged, not reported in the index result
  """
  rust/graph/src/ast_pipeline/mod.rs add_dependencies logged each extractor's Err with tracing::warn! and dropped it, and the Cargo, Java and Python extractors stopped at their first bad manifest. Each extractor now returns Extracted {entities, failures}: every manifest that could not be read or parsed is a ManifestError {file, error} and the extractor goes on with its other manifests; add_dependencies turns each into a DependencyFailure {file, language, error}, Indexed carries them as dependency_failures, and ast_index.rs puts them in the result as dependencies_failed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An index whose dependency extractor fails for a manifest succeeds, and its result lists the failure under dependencies_failed as {file, language, error}: file is the manifest's path relative to the root, language is the extractor's language (rust, javascript, python, go, java, php, ruby, csharp, scala, swift, dart) and error is the exact read or parse error (product owner, 2026-11-09)
  #   2. A failed extractor is never only logged: every failure is in the result; an index with no failure reports dependencies_failed as an empty list
  #   3. A failed manifest adds no Dependency nodes and no DependsOn edges of its own; every other manifest, including other manifests of the same language, and every source file are indexed as usual
  #
  # EXAMPLES:
  #   1. A project holds src/lib.rs and a package.json that is not valid JSON: ast_index succeeds, the graph holds src/lib.rs's function, and dependencies_failed is one entry with file package.json, language javascript and the JSON parse error
  #   2. A project whose manifests all parse: dependencies_failed is an empty list
  #   3. A Cargo workspace whose root Cargo.toml names serde lists member crates/a whose Cargo.toml is not valid TOML: dependencies_failed is one entry with file crates/a/Cargo.toml and language rust, and the graph still holds the Dependency serde
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch
    I want to index a project whose package manifest is malformed or unreadable
    So that the index still succeeds and its result names each dependency manifest that failed, so missing dependencies are never silent

  Scenario: A malformed package.json is listed in dependencies_failed and the index succeeds
    Given a project holding src/lib.rs with fn alpha and a package.json that is not valid JSON
    When ast_index runs for the project
    Then the index succeeds
    And the graph holds the Function alpha
    And dependencies_failed is one entry with file package.json, language javascript and the JSON parse error

  Scenario: A malformed workspace member manifest is listed and the other Cargo manifests are still indexed
    Given a Cargo workspace whose root Cargo.toml names the dependency serde and lists member crates/a
    And crates/a/Cargo.toml is not valid TOML
    When ast_index runs for the project
    Then dependencies_failed is one entry with file crates/a/Cargo.toml, language rust and the TOML parse error
    And the graph holds the Dependency serde

  Scenario: An index whose manifests all parse reports no dependency failure
    Given a project holding src/lib.rs and a valid package.json that names left-pad
    When ast_index runs for the project
    Then dependencies_failed is an empty list
    And the graph holds the Dependency left-pad
