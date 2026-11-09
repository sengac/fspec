@done
@rust
@code-analysis
@parser
@BUG-643
Feature: nanograph 1.3 Overwrite keeps rows whose keys aren't in the new load
  """
  Root cause (nanograph 1.3.0 src/store/database/persist.rs persist_dataset_mutation_plan_at_path): an Overwrite load numbers node ids from 0, build_node_delta (src/store/loader.rs) pairs previous and next rows by id, so a row whose id now holds another key is an 'upsert'; when a type's delta has only upserts, persist applies them with Lance merge_insert by the @key, which inserts the new key and keeps the old row, while the manifest records the batch's row count. Fix: take the merge-by-key path only when every upserted row keeps its key (the before_for_updates and upserts key columns are equal); otherwise the type's whole dataset is written. Merge loads keep ids stable per key, so they are unaffected.
  ast_index drops KGRAPH-074's delete-and-rebuild before every write (registry GraphWrite::reset now runs only for reset: true); a graph with an older schema fails with the actionable 'Schema has changed ... reset: true' error, as graph-database-reset.feature specifies.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An Overwrite load leaves a graph holding exactly the rows of the load: a node whose key is absent from the load is gone from queries, export and stats
  #   2. The vendored nanograph applies a node type's changed rows as a merge by key only when every changed row keeps its key; otherwise it writes the type's whole dataset (patches/nanograph.patch, a minimal documented change)
  #   3. ast_index writes the new graph over the existing one; it deletes the on-disk graph only when reset is true
  #   4. A bundle import in overwrite mode replaces the whole graph, the code graph and the learnings graph alike
  #
  # EXAMPLES:
  #   1. A project's src/lib.rs holds fn a and is indexed; a is renamed to b and the project is indexed again: ast_search, the export and ast_stats each show b only
  #   2. A code graph holding fn a imports, in overwrite mode, a bundle exported from a graph holding fn b: queries, the export and stats show b only
  #   3. A learnings graph holding learning A imports, in overwrite mode, a bundle holding learning B: learnings_search, the export and stats show B only
  #   4. Indexing a project twice without reset leaves the graph's on-disk dataset history in place (the graph is not deleted and created again)
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch
    I want to replace a code or learnings graph with an Overwrite load
    So that the graph holds exactly the rows of the new load, so a renamed or removed definition never lingers

  Scenario: A renamed definition is the only one left after the project is indexed again
    Given a project whose src/lib.rs holds fn a has been indexed
    When a is renamed to b and ast_index runs for the project again without reset
    Then ast_search lists b and not a
    And the graph's export holds one Function node, b
    And ast_stats counts 1 Function

  Scenario: An Overwrite load in place keeps no node whose key is absent from the load
    Given a code graph loaded with the File src/lib.rs and fn a
    When the same database is Overwrite-loaded with the File src/lib.rs and fn b
    Then a query of every function returns b only
    And the export holds the File and b only
    And the stats count 1 Function

  Scenario: A bundle import in overwrite mode replaces a code graph's definitions
    Given a code graph holding fn a
    And a bundle exported from a code graph holding fn b
    When the bundle is imported into the first graph in overwrite mode
    Then a query of every function returns b only
    And the export holds the File and b only
    And the stats count 1 Function

  Scenario: A bundle import in overwrite mode replaces a learnings graph's learnings
    Given a learnings graph holding learning A
    And a bundle exported from a learnings graph holding learning B
    When the bundle is imported into the first graph in overwrite mode
    Then learnings_search returns B and not A
    And the export holds learning B only
    And the stats count 1 Learning

  Scenario: Indexing again without reset writes over the existing graph
    Given a project that has been indexed
    And a marker file placed in the project's code graph directory
    When ast_index runs for the project again without reset
    Then the marker file is still in the code graph directory
    When ast_index runs for the project with reset
    Then the marker file is gone

  Scenario: Indexing a project whose source files were all removed leaves an empty graph
    Given a project whose src/lib.rs holds fn a has been indexed
    When src/lib.rs is deleted and ast_index runs for the project again without reset
    Then the graph's export holds no node
    And ast_stats counts 0 File and 0 Function

  Scenario: An in-place index that renumbers thousands of rows succeeds
    Given a project whose Python file holds 5,000 functions that each call the next has been indexed
    When a function that calls the first is added at the top of the file and ast_index runs for the project again without reset
    Then the index succeeds
    And the graph holds 5,001 functions and 5,000 Calls edges
