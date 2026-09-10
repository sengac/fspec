@done
@session-management
@wt-009
@tui
@session
@git
@bug-fix
@rust
@WT-009
Feature: /isolation only opens the create-session dialog and /merge-worktree never closes the session — UX contract broken in the Rust TUI
  """
  /isolation toggle in dispatch_slash_commands.rs: the Isolation arm now reads AgentViewStore::isolation_state_for(current) plus a backend.list_session_worktrees() probe; a tracked isolated session routes to a new app/dispatch_isolation_toggle.rs that spawns backend.detach_session_worktree(session_id) and on Ok emits a [isolation] detached notice; a live worktree the store does not track emits the still-isolated notice; everything else keeps the RPC-060 OpenCreateSessionDialog{preselect: Some(Isolated)} path. Merge success in dispatch_merge_worktree.rs::route_merge_outcome emits the [merge] success notice, then App::handle_merge_success_teardown mirrors dispatch_agent_exit.rs ExitChoice::CloseSession (board detach, open_sessions removal, mux re-sync, current-work-unit clear, spawned backend.destroy_session, BackToBoard).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /isolation is a state toggle on the current session: a non-isolated session with no session worktrees opens the CreateSessionDialog preselecting Isolated (existing RPC-060 behavior); an isolated session detaches from its worktree (new backend.detach_session_worktree RPC) and keeps the session; a session with a live worktree the TUI does not track emits a 'session still isolated — use /merge-worktree' notice without a backend call
  #   2. A new backend RPC detach_session_worktree(session_id) is added across the SessionManagerHandle trait (safe default), codelet-sessions handle_impl (clears the session's worktree_path/base_commit, clears the footer-cwd registry, re-spawns the footer poller with the project cwd, emits IsolationStateChange(false, None), deletes the git-session manifest so the worktree becomes prunable, and re-injects the environment context without isolation context), FspecService, FspecBackend, and both TUI transports
  #   3. On MergeStatus::Success in the Rust TUI merge flow: emit the [merge] success notice to the merged session, then run the CloseSession-equivalent teardown (BoardStore detach of the session's work unit, AgentViewStore open_sessions removal + mux window re-sync, current-work-unit pointer clear), spawn backend.destroy_session(sid), and dispatch BackToBoard so the user lands on the board — mirroring the TS merge → summary → cleanupCurrentSessionHandler() + destroySession() + onExit() flow
  #   4. The /isolation slash-command description must be updated to reflect the toggle behavior: 'Toggle worktree isolation (detach or create isolated session)'
  #
  # EXAMPLES:
  #   1. User in a non-isolated session runs /isolation → the CreateSessionDialog opens preselecting 'Yes - Isolated' (today's RPC-060 behavior is preserved)
  #   2. User in an isolated session (store shows is_isolated=true with a worktree path) runs /isolation → backend.detach_session_worktree is called, the session keeps running with effective cwd = project root, the IsolationStateChange(false, None) chunk lands in the store, and the worktree dir is marked prunable
  #   3. User in a non-isolated session whose worktree listing still shows a live worktree for the session runs /isolation → the TUI emits the notice 'session still isolated — use /merge-worktree' and makes no backend detach call
  #   4. User in an isolated session runs /merge-worktree and the merge succeeds with a commit SHA → the scrollback shows the [merge] success notice, the session tab closes, the session is destroyed on the backend, and the user lands back on the board view
  #
  # ========================================
  Background: User Story
    As a developer using the Rust TUI
    I want to toggle worktree isolation with /isolation and have /merge-worktree close the session on success
    So that the UX contract matches the TS reference: /isolation is a real toggle and a successful merge cleans up the session instead of leaving it pointing at a deleted worktree

  @happy-path
  Scenario: /isolation in a non-isolated session opens the create-session dialog preselecting Isolated
    Given an App with open session s-1 whose isolation state is non-isolated
    And the session worktree listing has no row for session s-1
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then a CreateSessionDialog is pushed onto the compositor with preselect "Yes - Isolated"
    And no backend worktree method is called

  @happy-path
  Scenario: /isolation in an isolated session detaches the session from its worktree
    Given an App with open session s-1 whose isolation state is isolated with worktree path "/repo/.fspec/worktrees/s-1"
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then within 1 second backend.detach_session_worktree is called exactly once with session_id "s-1"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[isolation] detached from worktree" is observed on the action bus
    And the CreateSessionDialog is NOT pushed onto the compositor
    And the session stays open in the AgentViewStore

  @happy-path
  Scenario: /isolation in an isolated session surfaces the detach outcome as an error when the backend fails
    Given an App with open session s-1 whose isolation state is isolated with worktree path "/repo/.fspec/worktrees/s-1"
    And the backend's detach_session_worktree returns Err("worktree not found")
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /isolation: worktree not found" is observed on the action bus
    And the session's isolation state remains isolated

  @edge-case
  Scenario: /isolation with a live worktree the store does not track warns without a backend call
    Given an App with open session s-1 whose isolation state is non-isolated
    And the backend's session worktree listing contains a row for session s-1
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text containing "session still isolated" and "/merge-worktree" is observed on the action bus
    And no backend detach_session_worktree call is made
    And the CreateSessionDialog is NOT pushed onto the compositor

  @edge-case
  Scenario: /isolation with no active session is a silent no-op
    Given an App with NO open AgentView session
    When SlashCommandSelected(SlashCommandAction::Isolation) is dispatched
    Then no backend method is called
    And the compositor contains no create-session-dialog

  @happy-path
  Scenario: Successful merge closes the session and returns to the board
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234"), worktree_path: Some("/repo/.fspec/worktrees/s-1") })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text starting with "[merge] success" is observed on the action bus
    And within 1 second backend.destroy_session is called exactly once with session_id "s-1"
    And session s-1 is removed from the AgentViewStore open sessions
    And the active view flips back to the board

  @edge-case
  Scenario: NoChanges merge keeps the session open without any teardown
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: NoChanges, conflicts: [], merge_commit: None, worktree_path: None })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[merge] nothing to merge" is observed on the action bus
    And no backend destroy_session call is made
    And session s-1 remains open in the AgentViewStore
    And the active view does not change

  @conflict-path
  Scenario: Conflict merge seeds the input with the worktree path instead of the session id
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Conflict, conflicts: ["src/a.rs"], merge_commit: None, worktree_path: Some("/repo/.fspec/worktrees/s-1") })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second the seeded input draft contains "Effective worktree: /repo/.fspec/worktrees/s-1"
    And no backend destroy_session call is made
    And session s-1 remains open in the AgentViewStore
