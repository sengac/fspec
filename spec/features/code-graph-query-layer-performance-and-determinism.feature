@done
@parser
@code-analysis
@rust
@KGRAPH-077
Feature: Code graph query layer performance and determinism
  """
  Query index: GraphDatabase holds Arc<QueryIndex> behind a tokio Mutex (single flight), keyed by the graph version. It is built by 6 whole-graph queries (all_calls, function_index and type_index without source, all_files, all_contains, all_contains_type) and dropped by every Overwrite load. Forward and reverse adjacency lists are sorted by slug when built; BFS sorts each level before emitting and truncating. Function and type files come from the Contains/ContainsType edges, not from slug prefixes
  Graph version: nanograph's manifest db_version and committed_at. A read handle is opened from one DatabaseMetadata read (Database::open_from_metadata), so the version is that of the snapshot the handle reads (Lance datasets are pinned to their versions); a write handle is opened under the writer lock and reads the version again after each Overwrite load. indexed_at is committed_at (unix seconds) as RFC 3339
  Reachability: nanograph's native calls{1,N} returns no hop count (measured 0.4 ms for 59 callers here), and callers/callees report each row's depth, so they run BFS over the cached adjacency instead (microseconds once the index is loaded). Correcting KGRAPH-059's traversal note is KGRAPH-078
  Measured before, on a snapshot of this repo (release, 5,333 files, 211,555 entities, 38,872 functions, 58,550 Calls edges, load about 11): ast_callers 20.2-28.7 s with output 133,362 vs 134,896 bytes on the same graph; ast_search 148-234 ms; ast_dead_code 144-177 ms. Probe: all_calls 35-132 ms (4.6 MB), function metadata without source 100 ms (19.5 MB) vs all_functions 133 ms (52.6 MB), one function by slug 0.3-0.8 ms
  Calls edge properties (callCount, isConditional) cannot be projected by a nanograph traversal (no edge variables), so call_details keep null for them as before; tests that index real fixtures for call-chain metadata are KGRAPH-078
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The query index of a code graph (the forward and reverse Calls adjacency, the metadata of every function and type without source text, and the file of every function and type) is loaded by a fixed number of whole-graph queries, once per graph version, and loaded again only after a write. No query runs once per function of the graph
  #   2. Every Calls adjacency list and every BFS level is in slug order: ast_callers and ast_callees return their results in (depth, slug) order and a limit keeps the first results of that order, so the same graph gives byte-identical output in every process
  #   3. ast_call_chain returns the paths whose every function before the target is at its shortest call distance from the source (a shortest path to a direct caller of the target, then the call to the target), within max_depth, ordered by length and then by their slug sequence, at most 20. They are enumerated from BFS distances and parents, never by queueing partial paths
  #   4. Source text is fetched only for the rows a response returns: matching, filtering, sorting and limiting in ast_search (name mode), ast_callers, ast_callees, ast_call_chain, ast_dead_code and ast_complexity read no source
  #   5. Every list result carries count (the items returned) and total (every item that matched before the limit): ast_search, ast_callers, ast_callees, each entity type of ast_dead_code and ast_complexity's top list; ast_call_chain reports truncated when more than 20 chains exist
  #   6. A failed query makes the action's result an error naming the query, never empty results; an invalid path glob is an error; a slug that is not in the graph is a not-found error for ast_neighbors as for ast_callers, ast_callees, ast_call_chain and ast_hierarchy
  #   7. ast_neighbors finds the node's type first and runs only that type's neighbour queries
  #   8. Every response of an AST action carries graph_version (the nanograph db_version of the graph state it read or wrote) and indexed_at (that state's commit time in RFC 3339)
  #   9. ast_search returns each entity type's matches in slug order
  #
  # EXAMPLES:
  #   1. A Python module defines target and 12 direct callers c11 down to c00 in reverse order; ast_callers target with limit 5 returns c00..c04 at depth 1 with count 5 and total 12, and after the graph is opened again the output is byte-identical
  #   2. ast_callers of a function with one caller runs the same number of queries on a graph of 3 functions and on one of 60; a second ast_callers on the same graph version loads no index, and after ast_index writes a new caller the next ast_callers returns it
  #   3. A calls B and C, B calls D, C calls E, E calls D: ast_call_chain A to D returns [A,B,D] then [A,C,E,D]
  #   4. A call graph of 12 layers of 2 functions, each calling both functions of the next layer (4096 shortest paths): ast_call_chain from the top to the bottom returns 20 chains of length 13 in slug-sequence order with truncated true
  #   5. ast_search helper in name mode over 5 matching functions with limit 2 returns the 2 lowest slugs, each with its source, count 2 and total 5
  #   6. ast_search with the path glob '[' returns an error naming the invalid glob
  #   7. After the graph's datasets are removed from disk under an open graph, ast_callers, ast_search, ast_neighbors, ast_hierarchy, ast_dead_code and ast_complexity each return an error, not empty results
  #   8. ast_neighbors of a function runs only the 4 function queries and returns its calls, callers, type refs and container; ast_neighbors of a slug not in the graph returns a not-found error
  #   9. After ast_index, ast_stats, ast_search and ast_callers report the graph_version and indexed_at of the index result; after a second index writes the graph, graph_version is higher
  #   10. ast_dead_code with limit 2 over 5 uncalled functions returns the 2 lowest slugs with their source, count 2 and total 5
  #   11. ast_complexity top 2 over functions of complexity 3, 3 and 1 returns the two of complexity 3 in slug order with total 3, running no query per function
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch
    I want to query callers, callees, call chains, search results, neighbours, hierarchy, dead code and complexity of an indexed project
    So that answers come back in milliseconds, are identical every time for the same graph, name the graph version they read, and a failed query is reported as an error instead of as no results

  Scenario: Callers come back in depth and slug order and a limit keeps the first of that order
    Given a Python module that defines target and 12 direct callers c11 down to c00 in reverse order has been indexed
    When ast_callers runs for target with limit 5
    Then the results are c00, c01, c02, c03 and c04 at depth 1
    And the result has count 5 and total 12
    And after the graph is opened again the same request returns byte-identical output

  Scenario: The query index is loaded once per graph version with a fixed number of queries
    Given a project of 3 functions and a project of 60 functions, each with one caller of target, have been indexed
    When ast_callers runs for target in each project
    Then both runs execute the same number of queries
    And a second ast_callers on the same graph version loads no index
    And after ast_index writes a new caller of target the next ast_callers returns it

  Scenario: Call chains are shortest paths to a caller of the target, shortest first
    Given an indexed project where A calls B and C, B calls D, C calls E and E calls D
    When ast_call_chain runs from A to D
    Then the chains are A, B, D and then A, C, E, D

  Scenario: Call chains in a graph with thousands of shortest paths are capped at 20 in slug order
    Given an indexed call graph of 12 layers of 2 functions below a top function and above a bottom function, each function calling both functions of the next layer
    When ast_call_chain runs from the top function to the bottom function with max_depth 13
    Then it returns 20 chains of length 13 in the order of their slug sequences
    And the result reports truncated true

  Scenario: A name search reads no source and returns the source of its result rows
    Given an indexed project with 5 functions whose names contain helper
    When ast_search runs for helper in name mode with limit 2
    Then it returns the 2 functions with the lowest slugs, each with its source
    And the result has count 2 and total 5

  Scenario: An invalid path glob is an error
    Given an indexed project
    When ast_search runs with the path glob "["
    Then the result is an error naming the invalid glob

  Scenario: A failed query is an error, never empty results
    Given an indexed project whose graph is open
    When the graph's datasets are removed from disk
    Then ast_callers, ast_search, ast_neighbors, ast_hierarchy, ast_dead_code and ast_complexity each return an error

  Scenario: Neighbours run only the queries of the node's type
    Given an indexed project where a function calls another function
    When ast_neighbors runs for the calling function
    Then it runs only the 4 function neighbour queries after finding the node's type
    And it returns the called function and the containing file
    When ast_neighbors runs for a slug that is not in the graph
    Then the result is a not-found error

  Scenario: Every response names the graph version it read
    Given a project has been indexed
    When ast_stats, ast_search and ast_callers run
    Then each reports the graph_version and indexed_at of the index result
    And after a second index writes the graph each reports a higher graph_version

  Scenario: Dead code reports its uncapped total and the source of the returned rows only
    Given an indexed project with 5 uncalled functions
    When ast_dead_code runs for functions with limit 2
    Then it returns the 2 uncalled functions with the lowest slugs, each with its source
    And the function result has count 2 and total 5

  Scenario: The most complex functions are ranked without a query per function
    Given an indexed project with functions of cyclomatic complexity 3, 3 and 1
    When ast_complexity runs for the top 2
    Then it returns the two functions of complexity 3 in slug order with total 3
    And it runs no query per function
