@done
@BUG-618
@GIT-033
@git
@multi-session
@rust
@BUG-541
Feature: File Search Popup Uses Worktree Path for Isolated Sessions
  """
  codelet-rpc: appended RPC search_files_in_session(session: SessionId, prefix, limit) -> Result<Vec<String>, String>: session_manager().get_effective_cwd(session) then codelet_core::file_search::search (empty prefix: no results, as search_files_in_workspace); MCP tool in the appended group. codelet-core: StubSessionManagerHandle can be given a session's working directory (set_effective_cwd), otherwise keeps its error. codelet-fspec-tui: FspecBackend::search_files_in_session (refusing default, embedded, websocket, mock); ScopedCalls carries the composer's session and search_files routes to search_files_in_session for every session (BUG-618, product owner 2026-11-07: every session's @file search uses its own working directory); with no session open it returns 'no session is open, so there are no files to search' (product owner, 2026-11-08). The board's selected workspace never affects @file search. Rewritten for the Rust daemon and TUI by BUG-541 (product owner, 2026-11-06, H-FILE-SEARCH-WORKTREE: route + test); the TypeScript useFileSearchInput scenarios and their deleted test mappings are gone.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A new RPC search_files_in_session(session, prefix, limit) searches the session's effective working directory: its worktree when the session is isolated (sessions resolve it; the TUI does not pass a path)
  #   2. In the agent view, @file search while the composer's session is isolated (its isolation state says so) asks search_files_in_session for that session; BUG-618: so does every non-isolated session, and with no session open the search shows 'no session is open, so there are no files to search'; the board's selected workspace never affects @file search
  #   3. A session whose working directory cannot be resolved is an error that is shown ('The files could not be searched: ...'); there is no silent fallback to the project root (the old TS fallback scenario is dropped)
  #
  # EXAMPLES:
  #   1. isolated session s-1 works in worktree W holding src/newfile.ts; the project root holds src/legacy.js, which was deleted in W: searching 'src/' in s-1 lists src/newfile.ts and not src/legacy.js
  #   2. in the agent view, isolated s-1 is focused and the user types @src: the search names session s-1, not the workspace
  #   3. in the agent view, non-isolated s-1 is focused and the board shows workspace beta: the search names session s-1 (BUG-618)
  #   4. the daemon cannot resolve s-1's working directory: the search fails and the TUI shows that the files could not be searched
  #
  # QUESTIONS (ANSWERED):
  #   Q: should @file search in a NON-isolated session also follow the session's own directory (it can differ from the board's selected workspace), or keep RPC-455's selected-workspace routing?
  #   A: Conservative option taken (supervisor instruction 2026-11-06, pending product owner review): keep RPC-455's routing for non-isolated sessions and for no session; only isolated sessions search their worktree, as the product owner decided for H-FILE-SEARCH-WORKTREE (2026-11-06). Superseded by BUG-618 (product owner, 2026-11-07): every session searches its own working directory.
  #
  # ========================================
  Background: User Story
    As a user typing @ to reference a file in an isolated session
    I want to have the @file search list the files of the session's worktree
    So that I find the files the agent created there and never pick one it deleted or cannot see

  # ─────────────────────────────────────────────────────────────────────
  # The daemon searches the session's working directory (rules 1, 3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Searching files in an isolated session lists its worktree
    Given session "s-1" works in a worktree holding "src/newfile.ts"
    And the project root holds "src/legacy.js", which the worktree no longer has
    When files starting with "src/" are searched in session "s-1" over RPC
    Then "src/newfile.ts" is found
    And "src/legacy.js" is not found

  Scenario: Searching files in a session whose working directory is unknown fails
    Given the daemon cannot resolve the working directory of session "s-1"
    When files starting with "src/" are searched in session "s-1" over RPC
    Then the search fails naming session "s-1"

  # ─────────────────────────────────────────────────────────────────────
  # The TUI routes @file search (rules 2, 3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: The agent view searches an isolated session's files in that session
    Given isolated session "s-1" is focused in the agent view
    When the user types "@src"
    Then the file search names session "s-1"
    And no workspace file search is made

  Scenario: The agent view searches a non-isolated session's files in that session
    Given the board shows workspace "beta"
    And non-isolated session "s-1" is focused in the agent view
    When the user types "@src"
    Then the file search names session "s-1"
    And no workspace file search is made

  Scenario: A failed session file search is shown
    Given isolated session "s-1" is focused in the agent view
    And the daemon cannot search the files of session "s-1"
    When the user types "@src"
    Then the TUI shows that the files could not be searched

  Scenario: With no session open, @file search says there are no files to search
    Given the board shows workspace "beta"
    And no session is open in the agent view
    When the user types "@src"
    Then the TUI shows 'no session is open, so there are no files to search'
    And no file search is made
