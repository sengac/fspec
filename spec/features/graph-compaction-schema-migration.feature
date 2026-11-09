@KGRAPH-010
@parser
@migration
Feature: Graph Compaction & Schema Migration
  """
  Pure Rust module at rust/napi/src/graph/compaction.rs. Turn pruning builds cutoff date and filters. Schema migration compares SHA256 hashes. Retention config in index-state.json. No DB writes for migration — delegates to nanograph migrate.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Turn nodes older than configurable maxAgeDays (default: 90) are pruned, except those with Decides edges
  #   2. Pruning a Turn node cascades to delete its Mentions and Modifies edges
  #   3. Schema migration compares bundled schema hash with on-disk schema.ir.json hash on open
  #   4. Safe schema changes (new optional properties, new types) auto-apply via nanograph migrate
  #   5. Breaking schema changes (removed types, non-nullable additions) are blocked with a clear error
  #   6. Compaction configuration stored in index-state.json with retention settings
  #
  # EXAMPLES:
  #   1. Turn nodes older than retention period are pruned except decision-linked ones
  #   2. Pruned Turn cascades to delete its edges
  #   3. Safe schema change auto-migrates on open
  #   4. Schema hash match skips migration
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to have the knowledge graph automatically pruned, compacted, and migrated
    So that the graph stays performant as data grows and schema evolves between versions

  Scenario: Schema hash match skips migration
    Given the bundled schema hash matches the on-disk schema.pg hash
    When the database is opened
    Then no migration is performed
    And the database opens normally
