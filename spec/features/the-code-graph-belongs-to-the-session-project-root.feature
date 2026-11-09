@done
@KGRAPH-073
@parser
@code-analysis
@tool-execution
@bug-fix
@rust
Feature: The code graph belongs to the session's project root
  """
  Session root source: BackgroundSession::effective_cwd() (worktree_path else project), the same source WT-016 uses for the Fspec tool; the per-call root is read when the tool runs, so entering or leaving isolation mid-session moves the graph with it. agent-loop graph_search_handler::create_handler takes a root source; DeepSearch passes its project path.
  codelet-graph registry: map (GraphKind, canonical root) -> Arc<tokio RwLock<Option<GraphDatabase>>>; read leases hold the read side for the whole query, write leases hold the write side plus an OS file lock (std File::try_lock, Rust 1.89+) on <root>/.fspec/graph/ast-code.nano.lock. get_graph(name), is_graph_initialized, insert_graph_for_test, resolve_graph_config and the AST_CODE_GRAPH/LEARNINGS_GRAPH name lookup are removed (no legacy). Availability = schema.pg on disk hashes equal to the compiled schema.
  Walk: ignore::WalkBuilder with parents(false), require_git(false), git_global(false), git_exclude(false) so the indexed files depend only on the root's own tree; the old 'custom path disables .gitignore' switch is gone because a root inside a gitignored directory no longer inherits the parent's rules.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The code graph a GraphSearch AST action uses belongs to one canonical root: the action's root parameter when given (a relative root joins the session's working directory), otherwise the session's working directory, which is the worktree of an isolated session and the session's project otherwise; the daemon's working directory never takes part and graph code never reads current_dir()
  #   2. Each canonical root has its own code graph at <root>/.fspec/graph/ast-code.nano, held in a registry keyed by (graph kind, canonical root), and every AST action result names the root it used
  #   3. ast_index with root=<dir> indexes that directory into its own graph and never changes another root's graph; the files indexed are a function of the root alone: .gitignore files at and below the root apply, ignore files above the root and the user's global excludes do not; ast_index rejects the path parameter (a query glob filter) with an explicit error
  #   4. A query on a root that has no code graph fails with an explicit error naming the root and ast_index, and creates nothing on disk
  #   5. Writes to one graph (ast_index, reset, ast_import) are serialized in the process: a second writer waits, a write waits for running readers and readers wait while a write runs; the first access of a root opens or creates its graph once even when callers race
  #   6. A write holds an OS file lock at <root>/.fspec/graph/ast-code.nano.lock for its whole duration; when another process holds that lock the write fails with an explicit error naming the lock and writes nothing
  #   7. DeepSearch offers GraphSearch and the learnings context exactly when the session project's code graph or the learnings graph exists on disk with the compiled schema; which graphs this process opened before does not matter
  #   8. Provisional, most conservative option (helper 2026-11-07, pending the product owner): in an isolated session a root inside the main checkout is refused with the file tools' 'blocked from original project' error, as WT-016 does for the Fspec tool
  #
  # EXAMPLES:
  #   1. Session A's project holds fn alpha_only and session B's project holds fn beta_only; both index; A's ast_search finds alpha_only and not beta_only, B's finds beta_only and not alpha_only, and each graph sits under its own project's .fspec/graph
  #   2. An isolated session's main checkout holds fn main_only and its worktree holds fn tree_only; ast_index then ast_search lists tree_only and not main_only, the graph is under the worktree's .fspec/graph and the main checkout gets no .fspec/graph
  #   3. Project P (fn project_fn) is indexed; ast_index with root vendor/lib (fn vendor_fn) reports root P/vendor/lib; ast_search in P finds project_fn and not vendor_fn, and ast_search with root vendor/lib finds vendor_fn
  #   4. ast_index with path 'src/**' fails with an error that names the root parameter and indexes nothing
  #   5. ast_search in a project that was never indexed fails with 'no code graph' naming the project root and ast_index, and no .fspec/graph directory appears
  #   6. Another process holds P/.fspec/graph/ast-code.nano.lock; ast_index in P fails with an error naming the lock and P gets no graph
  #   7. A reader holds P's graph; ast_index with reset stays pending until the reader is done and then completes
  #   8. Two ast_index calls on the same root run at once in one process; both succeed
  #   9. Project P's code graph is on disk and nothing in the process has opened it; DeepSearch for P offers GraphSearch. Project Q has no graph on disk and no learnings graph exists, while P's graph is open in the process; DeepSearch for Q does not offer GraphSearch
  #   10. An isolated session runs ast_search with the main checkout's absolute path as root; the tool fails with 'blocked from original project'
  #
  # QUESTIONS (ANSWERED):
  #   Q: may an isolated session read or index the main checkout's code graph by passing its path as root? Provisional (most conservative, matching WT-016's Fspec tool): no, the file tools' isolation check applies to root and refuses it with 'blocked from original project'.
  #   A: Provisional, most conservative option (helper 2026-11-07, pending the product owner): in an isolated session a root inside the main checkout is refused with the file tools' 'blocked from original project' error, as WT-016 does for the Fspec tool
  #
  # ========================================
  Background: User Story
    As a agent calling GraphSearch in a session
    I want to have the code graph I index and query be the graph of my session's project root (or a root I name), never the daemon's working directory
    So that answers are about the code I am working on, an index of one tree never replaces another tree's graph, and two processes cannot corrupt one graph

  Scenario: Sessions in two projects each index and query their own graph
    Given session A's project holds a function named alpha_only
    And session B's project holds a function named beta_only
    When each session runs ast_index and then ast_search for "only"
    Then session A's results list alpha_only and not beta_only
    And session B's results list beta_only and not alpha_only
    And each project's graph is under that project's .fspec/graph directory

  Scenario: An isolated session indexes its worktree, not the main checkout
    Given an isolated session whose main checkout holds a function named main_only
    And whose worktree holds a function named tree_only
    When the session runs ast_index and then ast_search for "only"
    Then the results list tree_only and not main_only
    And the graph is under the worktree's .fspec/graph directory
    And the main checkout has no .fspec/graph directory

  Scenario: Indexing another root builds that root's own graph
    Given a session whose project holds a function named project_fn and has been indexed
    And the project's vendor/lib directory holds a function named vendor_fn
    When the session runs ast_index with root "vendor/lib"
    Then the result names the canonical vendor/lib directory as its root
    And ast_search in the project lists project_fn and not vendor_fn
    And ast_search with root "vendor/lib" lists vendor_fn

  Scenario: ast_index refuses the path parameter
    Given a session whose project holds source files
    When the session runs ast_index with path "src/**"
    Then the tool fails with an error that names the root parameter
    And the project has no .fspec/graph directory

  Scenario: A query on a root without a graph fails and creates nothing
    Given a session whose project has never been indexed
    When the session runs ast_search
    Then the tool fails with "no code graph" naming the project root and ast_index
    And the project has no .fspec/graph directory

  Scenario: A write fails while another process holds the writer lock
    Given another process holds the writer lock of project P's code graph
    When ast_index runs for project P
    Then it fails with an error naming the lock file
    And project P has no code graph

  Scenario: A reset waits for a running reader
    Given project P has been indexed
    And a reader holds project P's graph
    When ast_index runs for project P with reset
    Then the index stays pending while the reader holds the graph
    And the index completes once the reader is done

  Scenario: Two index runs on one root in one process both succeed
    Given project P holds source files
    When two ast_index runs for project P start at the same time
    Then both runs succeed
    And project P's graph holds P's functions

  Scenario: DeepSearch offers GraphSearch from the project's graph on disk
    Given project P's code graph is on disk and no graph is open in the process
    And project Q has no code graph and no learnings graph exists
    When a DeepSearch sub-agent is built for project P
    Then its tools include GraphSearch
    When P's graph is opened in the process and a DeepSearch sub-agent is built for project Q
    Then its tools do not include GraphSearch

  Scenario: An isolated session cannot use the main checkout as a graph root
    Given an isolated session whose project is the main checkout
    When the session runs ast_search with the main checkout's absolute path as root
    Then the tool fails with the 'blocked from original project' error

  Scenario: Graph code never reads the process working directory
    Given the codelet-graph sources and the GraphSearch handler sources
    When they are scanned for current_dir
    Then no call to current_dir is found
