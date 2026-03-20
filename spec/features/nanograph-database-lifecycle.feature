@KGRAPH-002
Feature: Nanograph Database Lifecycle & Integration

  """
  Nanograph integrated as Rust crate via git submodule + Cargo path dependency. Database stored at ~/.fspec/graph/agent-memory.nano/. Singleton pattern: lazy_static Mutex<Option<Database>>. Schema bundled via include_str!(). Feature-gated behind 'graph' Cargo feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Database must be lazily initialized on first use (not at startup)
  #   2. Database must be resettable when set_data_directory() changes the base path (Pattern A: lazy_static Mutex<Option<T>>)
  #   3. The agent-memory.pg schema file must be bundled with codelet (compiled into the binary or read from a known location)
  #   4. Database init creates the .nano directory + schema; Database open reuses existing; both must be ACID-safe
  #   5. Nanograph is integrated as a Rust crate dependency (not the npm package) since codelet-napi is already a Rust crate
  #   6. If the .nano directory does not exist, first GraphSearch/index call auto-initializes it
  #   7. Close/cleanup must happen on process exit to avoid Lance corruption
  #   8. Embeddings are only needed at load time when data with @embed properties is ingested, or when running the embed command. The database can init/open/query without any embedding API key. Safe to defer.
  #   9. The nanograph database must be stored in the global ~/.fspec/graph/agent-memory.nano/ directory, alongside sessions and messages
  #   10. Git submodule with Cargo path dependency — gives us control over version pinning and ability to patch if needed, same pattern as rig-core patches
  #   11. Significant impact — Lance + DataFusion + Arrow add ~30-50MB to binary and 2-5 min to compile. Acceptable tradeoff for a typed graph DB. Feature-gate behind a 'graph' Cargo feature so it can be disabled in minimal builds.
  #
  # EXAMPLES:
  #   1. Subsequent process starts: Database.open() detects existing .nano directory, loads schema, graph is available immediately
  #   2. set_data_directory() resets the nanograph singleton so next access re-initializes from the new path
  #   3. GraphSearch stats action on empty graph returns all zeros — no crash, no error
  #   4. Schema includes all node types (Concept, Decision, CodeEntity, WorkUnit, Session, Turn) and all edge types from the design doc
  #   5. First call to GraphSearch: ~/.fspec/graph/agent-memory.nano/ is created, schema is written, Database.init() succeeds, empty graph is ready for queries
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we vendor nanograph as a git submodule, a Cargo path dependency, or a crates.io dependency?
  #   A: Git submodule with Cargo path dependency — gives us control over version pinning and ability to patch if needed, same pattern as rig-core patches
  #
  #   Q: Does nanograph require the OpenAI embedding API key at init time, or only when @embed properties are first populated?
  #   A: Embeddings are only needed at load time when data with @embed properties is ingested, or when running the embed command. The database can init/open/query without any embedding API key. Safe to defer.
  #
  #   Q: Nanograph pulls in Lance + DataFusion + Arrow — how does this affect binary size and compile time?
  #   A: Significant impact — Lance + DataFusion + Arrow add ~30-50MB to binary and 2-5 min to compile. Acceptable tradeoff for a typed graph DB. Feature-gate behind a 'graph' Cargo feature so it can be disabled in minimal builds.
  #
  # ========================================

  Background: User Story
    As an agent developer
    I want to have an embedded nanograph database that initializes, opens, and closes cleanly
    So that the knowledge graph has a reliable persistence layer for all subsequent features

  Scenario: First GraphSearch call auto-initializes the database
    Given the ~/.fspec/graph/ directory does not exist
    When a GraphSearch action is invoked for the first time
    Then the ~/.fspec/graph/agent-memory.nano/ directory is created
    And the schema.pg file contains the agent-memory schema
    And the database is open and ready for queries


  Scenario: Subsequent process opens existing database
    Given the ~/.fspec/graph/agent-memory.nano/ directory already exists with a valid schema
    When a GraphSearch action is invoked
    Then the existing database is opened without re-initialization
    And all previously stored graph data is accessible


  Scenario: Data directory change resets graph singleton
    Given the graph database is open and initialized
    When set_data_directory() is called with a new path
    Then the graph singleton is reset to None
    And the next GraphSearch call initializes from the new data directory


  Scenario: Empty graph returns zero stats without error
    Given the graph database has been initialized with no data loaded
    When a stats query is executed against the graph
    Then all node and edge counts are zero
    And no error is returned


  Scenario: Schema contains all required node and edge types
    Given the graph database has been initialized
    When the database schema is described
    Then node types Concept, Decision, CodeEntity, WorkUnit, Session, and Turn exist
    And edge types Mentions, Discusses, Decides, Implements, Modifies, RelatesTo, Supersedes, WorksOn, References, and ContainsTurn exist

