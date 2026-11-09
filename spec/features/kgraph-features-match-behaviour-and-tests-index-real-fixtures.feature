@done
@parser
@code-analysis
@rust
@KGRAPH-078
Feature: KGRAPH features drift from behaviour and some tests are vacuous
  """
  Calls edge properties: nanograph traversals cannot project edge properties, so the vendored nanograph export gets build_edge_export_rows(metadata, edge_type) (rust/patches/nanograph + nanograph.patch), reading one edge dataset and its endpoint keys from the handle's own DatabaseMetadata snapshot. The QueryIndex reads the Calls rows once per graph version and keeps callCount/isConditional per (caller, callee)
  Unresolved counts: ast_index writes <graph>/fspec-index-unresolved.json beside fspec-index.sha256 after a successful load; GraphDatabase::overwrite deletes both first, so a bundle import or any other write leaves no counts and dead code reports unresolved null with unresolved_unknown naming why
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. KGRAPH-065 is titled Incremental Re-indexing (no file watcher exists); its modification-time rules and its 50% full-index fallback rule are removed (superseded by KGRAPH-076, product owner 2026-11-07), and incremental-reindexing.feature describes the content-hash index
  #   2. KGRAPH-059's architecture note says nanograph supports bounded variable-length traversal ($a calls{1,N} $b) without a hop count, so depth-annotated traversals run BFS over the cached Calls adjacency (KGRAPH-077)
  #   3. Every scenario of the call-chain and transitive callers/callees features builds its graph with ast_index from a fixture project; no test of these features loads hand-written JSONL
  #   4. Each call_details hop of ast_call_chain carries the callCount and isConditional of its Calls edge as ast_index stored them
  #   5. ast_dead_code carries unresolved: the unresolved reference counts per kind that the last ast_index of the graph recorded; when the graph was last written by something other than ast_index the result says the counts are not known instead of guessing
  #   6. The ast-queries.gq comment of unreferenced_types says it checks incoming TypeRef edges only, as it does
  #
  # EXAMPLES:
  #   1. A Python fixture where a calls b once unconditionally: the hop has callCount 1 and isConditional false
  #   2. A Python fixture with an uncalled function and a call on an unknown receiver: ast_dead_code reports unresolved with the same calls counts the ast_index result reported
  #   3. After a bundle is imported into the graph, ast_dead_code reports that the unresolved counts are not known because the graph was not written by ast_index
  #   4. The linear chain a calls b calls c calls d and the isolated function of the call-chain and transitive tests come from an indexed Python fixture
  #   5. A Python fixture where a calls b twice, both inside an if: ast_call_chain a to b has one hop with callCount 2 and isConditional true
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch
    I want to rely on the KGRAPH cards, features and tests to describe what the code graph actually does
    So that a passing test proves the behaviour on a graph built by ast_index, and a dead-code answer says how many references could not be resolved

  Scenario: A hop of a call chain carries the call count and conditionality of its Calls edge
    Given an indexed Python fixture where a calls b twice, both inside an if
    When ast_call_chain runs from a to b
    Then the chain's one hop has callCount 2 and isConditional true

  Scenario: An unconditional single call has call count 1 and is not conditional
    Given an indexed Python fixture where a calls b once outside any branch
    When ast_call_chain runs from a to b
    Then the chain's one hop has callCount 1 and isConditional false

  Scenario: Dead code reports the unresolved reference counts of the last index
    Given a Python fixture with an uncalled function and a call on an unknown receiver has been indexed
    When ast_dead_code runs
    Then its unresolved counts equal the unresolved counts of the ast_index result

  Scenario: After a bundle import dead code says the unresolved counts are not known
    Given an indexed Python fixture whose graph was exported to a bundle
    When the bundle is imported into the graph and ast_dead_code runs
    Then its unresolved is null
    And unresolved_unknown says the graph was not last written by ast_index
