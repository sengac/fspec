@done
@testing-framework
@integration-test
@agent-view
@tui
@rust
@RPC-065
Feature: Behaviour-parity test suite for every slash command + keyboard shortcut
  """
  [A] File layout: (1) new module `codelet/fspec-tui/tests/common/harness.rs` declared from `tests/common/mod.rs` via `pub mod harness;`. The harness wraps `App` + `Arc<MockBackend>` (the existing 2876-LoC double already exports every counter we need). (2) new integration test file `codelet/fspec-tui/tests/behaviour_parity_rpc065.rs` declaring `mod common;` to pull in the harness module.
  [B] AppTestHarness API surface (private to tests/common/harness.rs): `pub struct AppTestHarness { pub app: App, pub mock: Arc<MockBackend> }`. Constructors: `pub fn new() -> Self` (seeds session s-1 + focuses it), `pub fn empty() -> Self` (no sessions). Helpers: `add_session(SessionId)`, `seed_chunks(&SessionId, usize)`, `submit_input(&str)` (synchronous: drives `Action::InputSubmitted` through dispatch), `press_key(KeyEvent)` (drives navigator.handle_event → emit → dispatch), `dispatch_slash(SlashCommandAction)` (sugar over Action::SlashCommandSelected), `current_session() -> Option<&SessionId>`, `compositor_contains(id: &str) -> bool`, `active_view() -> ViewMode`, `should_quit() -> bool`, `scrollback_chunk_count(&SessionId) -> usize`, `drain_pending() -> async` (loops until pending_tasks all settle and action_rx is drained — bounded by 1s), `wait_until(predicate, label) -> async`.
  [C] drain_pending implementation: re-uses the well-tested pattern from `slash_clear_rpc046.rs::drain_pending` — repeatedly `tokio::select!` over (action_rx.recv(), tokio::time::sleep(1ms)) and on each received Action call `app.dispatch(action)`; bail out when both action_rx is empty AND pending_tasks is empty AND no new actions appeared after a 10ms quiescence window. Wrap in `tokio::time::timeout(Duration::from_secs(1), …)` to fail fast on stuck tests.
  [D] Test file structure: `behaviour_parity_rpc065.rs` organises tests by SECTION using `mod` blocks (slash_help, slash_clear, slash_quit, slash_model, slash_thinking, slash_role, slash_resume, slash_search, slash_provider, slash_debug, slash_compact, slash_isolation, slash_blocklist, slash_detach, slash_merge_worktree, slash_schedule, slash_loop, key_shift_arrows, key_history_recall, key_tab_turn_selection, key_ctrl_r, key_esc_cascade, key_enter_submit, key_ctrl_c_interrupt, key_pagedown_end). One `#[tokio::test]` per matrix row, each ~10-20 LoC of harness sugar.
  [E] Per-test TS-REF/DEEP-REF convention: each test has a triple-slash doc comment block of the form `/// TS-REF: src/tui/views/AgentView.tsx line 2721 (handleSearchSlash) /// DEEP-REF: tests/search_view_rpc064.rs::picking_search_from_palette_opens_empty_view`. These doubles as searchable breadcrumbs when the TS frontend changes and we need to update parity.
  [F] Backward compatibility: MockBackend currently lives at tests/common/mod.rs and is imported by every test as `use common::MockBackend;`. Adding `pub mod harness;` to tests/common/mod.rs does NOT break any existing test (only adds a new sub-module). The harness IMPORTS MockBackend via `super::MockBackend` so the existing file stays untouched.
  [G] No source code production changes required. Everything in this card lives in `tests/` — including the harness — so the dependency-rule regression tests (no_napi_dependency.rs etc.) remain unaffected. `cargo build -p codelet-fspec-tui` behaviour is unchanged.
  [H] Runtime flavour: every parity test uses `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` because nine of the matrix rows require draining spawned tasks (clear/compact/debug/detach/role-set/thinking-set/schedule/loop/ctrl-c). Synthetic-key smoke tests (help/quit/model-dialog/thinking-dialog/role-dialog/resume/search/blocklist/provider) could use current_thread but uniformity outweighs the minor scheduler overhead — total wall-clock cost <30s per attachment AC #4.
  [I] /merge-worktree quirk: the slash command does NOT push the MergeConfirmDialog directly — it first spawns `backend.inspect_session_changes(session)` and only on a non-zero summary dispatches `Action::OpenMergeConfirmDialog`. So the matrix smoke assertion is `mock.inspect_session_changes_calls() == 1` (NOT compositor.contains(MERGE_CONFIRM_DIALOG_ID)). The dialog-push half of the flow is the canonical merge_worktree_rpc057.rs test.
  [J] /isolation quirk: SlashCommandSelected(Isolation) dispatches `Action::OpenCreateSessionDialog { preselect: Some(CreateSessionOption::Isolated) }` (NOT a separate isolation dialog). Matrix assertion: `compositor_contains(CREATE_SESSION_DIALOG_ID)` after draining the action bus (the dispatch goes through action_tx, not direct compositor push).
  [K] /provider activates the ProviderSettingsView via Action::OpenProviderSettingsView delivered through action_tx → navigator.apply_action. There is NO /providers alias — the TypeScript Ink reference frontend src/tui/utils/slashCommands.ts defines a single entry (name: 'provider'), and the Rust SLASH_COMMANDS registry mirrors that 1:1.
  [L] Ctrl+C requires the session to be in a state where the dispatch path emits `Action::Interrupt` (codelet/fspec-tui/src/views/agent/dispatch.rs line 240). The harness press_key helper synthesises the KeyEvent and routes it through `app.handle_event(Event::Key(_))`. Status gating: the App::dispatch handler for Interrupt only spawns backend.interrupt when the focused session's status is Running/Compacting (per dispatch_esc_cascade.rs). The parity test seeds `SessionStatus::Running` via `mock.push_status_change(s-1, Running)` then drains, then presses Ctrl+C.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AppTestHarness lives in tests/common/harness.rs (a new sibling of tests/common/mod.rs) and wraps an App + Arc<MockBackend>; every parity test constructs it via AppTestHarness::new() and never re-rolls fresh_app()/fresh fixtures inline
  #   2. AppTestHarness exposes ergonomic helpers that hide the seed_chunks/scrollback_text/drain_pending/wait_until boilerplate currently duplicated across slash_clear_rpc046.rs, slash_compact_rpc047.rs, slash_role_rpc063.rs, slash_thinking_rpc048.rs, slash_resume_rpc049.rs, slash_detach_rpc050.rs etc.
  #   3. AppTestHarness::new() seeds a single focused SessionId 's-1' so the smoke tests in the parity matrix don't repeat session-creation setup; tests that need a different topology (no session, multi-session, etc.) construct the harness via AppTestHarness::empty() or call helpers like harness.add_session(SessionId)
  #   4. A single new integration test file `codelet/fspec-tui/tests/behaviour_parity_rpc065.rs` walks the full matrix from the RPC-065 attachment; existing detailed `*_rpcNNN.rs` test files are NOT deleted or migrated
  #   5. Each parity test contains a `// TS-REF:` doc-comment naming the equivalent `src/tui/__tests__/...` file (or the AgentView.tsx line range) AND a `// DEEP-REF:` doc-comment pointing at the canonical `*_rpcNNN.rs` test for that slash command — so future readers always know where the authoritative assertions live
  #   6. Parity tests only assert OBSERVABLE store-state transitions (compositor.contains(...), app.should_quit, MockBackend counter == N, AgentViewStore.search_view.is_some(), etc.) — they do NOT re-assert the deep behaviour already covered by the canonical card test (no scrollback-text golden strings, no error-branch coverage, no debounce/timing logic)
  #   7. Genuinely missing coverage (Tab turn-selection-mode entry, explicit Enter→backend.send_input assertion through SlashCommandSelected dispatch, explicit Ctrl+C→backend.interrupt assertion) is added in this same file as first-class tests (NOT as smoke smoke-tests) since no canonical *_rpcNNN.rs test exists yet
  #   8. Tests use `#[tokio::test(flavor = "current_thread")]` (per the attachment risk note) UNLESS the asserted behaviour requires the multi-thread runtime (spawned-task draining). The harness `drain_pending().await` and `wait_until()` helpers work under both flavours.
  #   9. `cargo test -p codelet-fspec-tui --test behaviour_parity_rpc065` runs the full parity suite in under 30 seconds wall-clock on a stock dev machine (mirrors the attachment AC #4)
  #   10. Add a #[tokio::test] for Tab turn-selection mode but mark it #[ignore = "Tab turn-selection mode pending future RPC card — placeholder behaviour-parity assertion documented but not yet wired in the Rust AgentView"]. The test body should compile (so the assertion contract is locked in) and a future RPC card simply removes the #[ignore] when Tab handling lands. Zero scope creep on RPC-065.
  #
  # EXAMPLES:
  #   1. Dispatching SlashCommandSelected(Help) on a fresh harness pushes a HelpDialog onto the compositor with id 'help-dialog' (the dialog can be dismissed via Esc which removes it from the compositor)
  #   2. Dispatching SlashCommandSelected(Quit) flips app.should_quit from false to true (no backend calls, no scrollback writes)
  #   3. Dispatching SlashCommandSelected(Clear) on a session with seeded scrollback empties the scrollback synchronously AND increments mock.clear_history_calls() to 1 within 1s with last_clear_history_session() == Some(s-1)
  #   4. Dispatching SlashCommandSelected(Model) pushes a ModelSelectorDialog onto the compositor (assertion: compositor.contains(MODEL_SELECTOR_DIALOG_ID))
  #   5. Dispatching SlashCommandSelected(Thinking) pushes a ThinkingLevelDialog onto the compositor (assertion: compositor.contains(THINKING_LEVEL_DIALOG_ID))
  #   6. Submitting the text `/thinking high` through harness.submit_input() does NOT open ThinkingLevelDialog; instead within 1s mock.set_thinking_level_calls() reaches 1 and last_set_thinking_level() reports (s-1, ThinkingLevel::High)
  #   7. Dispatching SlashCommandSelected(Role) pushes a RoleDialog (compositor.contains(ROLE_DIALOG_ID)); submitting `/role You are a security reviewer` skips the dialog and within 1s mock.set_session_role_calls() reaches 1 with last_set_session_role() == Some((s-1, Some("You are a security reviewer")))
  #   8. Dispatching SlashCommandSelected(Resume) sets agent_view_store.resume_view to Some(_) (a ResumeSessionView is now active in the AgentView)
  #   9. Dispatching SlashCommandSelected(Search) sets agent_view_store.search_view to Some(_); pressing Ctrl+R through harness.press_key(Ctrl+R) does the same with NO backend call until the user types
  #   10. Dispatching SlashCommandSelected(Provider) results in agent_view_store.provider_settings_view becoming Some(_) — there is no /providers alias, only the singular /provider command routes to Action::OpenProviderSettingsView
  #   11. Dispatching SlashCommandSelected(Debug) increments mock.toggle_debug_calls() to 1 within 1s with last_toggle_debug() == Some((s-1, _debug_dir))
  #   12. Dispatching SlashCommandSelected(Compact) increments mock.compact_session_calls() to 1 within 1s with last_compact_session() == Some(s-1)
  #   13. Dispatching SlashCommandSelected(Isolation) pushes a CreateSessionDialog onto the compositor pre-selected to the Isolated option (compositor.contains(CREATE_SESSION_DIALOG_ID))
  #   14. Dispatching SlashCommandSelected(Blocklist) sets agent_view_store.blocklist_view to Some(_)
  #   15. Dispatching SlashCommandSelected(Detach) on a session bound to a work-unit context increments mock.set_work_unit_context_calls() to 1 within 1s with last_set_work_unit_context() == Some((s-1, None))
  #   16. Dispatching SlashCommandSelected(Schedule) appends a static help notice into the focused session's scrollback (the bare-popup path documented in dispatch_slash_schedule.rs::handle_slash_schedule_help)
  #   17. Dispatching SlashCommandSelected(Loop) appends a static help notice into the focused session's scrollback (dispatch_slash_loop.rs::handle_slash_loop_help)
  #   18. Submitting `/schedule list` through harness.submit_input() increments mock.schedule_list_calls() to 1 within 1s; submitting `/loop list` increments mock.loop_list_calls() to 1 within 1s (parser-routed paths)
  #   19. Pressing Shift+Right with two open sessions cycles focus from s-1 → s-2 (assertion: agent_view_store.current_session() == Some(s-2)); Shift+Left cycles back
  #   20. Pressing Shift+Up on a focused session with scripted history `["old1", "old2"]` seeded via mock.script_history() loads the most-recent entry into the input within 1s (input value becomes "old2")
  #   21. Pressing Tab on a session with ≥1 scrollback chunk enters turn-selection mode (agent_view_store.turn_selection_mode() == true and a turn-selection cursor is now visible)
  #   22. Pressing Esc with one HelpDialog on the compositor pops that dialog and leaves the input untouched (the 5-level cascade's level-1 case — full cascade tested by keyboard_cascade_rpc051.rs and referenced via DEEP-REF)
  #   23. Submitting plain text `hello world` (no slash prefix) through harness.submit_input() increments mock.send_input_calls() to 1 within 1s with last_send_input() == Some((s-1, "hello world"))
  #   24. Pressing Ctrl+C while a session is running (status == SessionStatus::Running) increments mock.interrupt_calls() to 1 within 1s with last_interrupt() == Some(s-1)
  #   25. Pressing PageDown with 30 seeded scrollback chunks advances the scrollback viewport by one page; pressing End jumps to the bottom (scrollback.is_at_bottom() == true)
  #   26. Dispatching SlashCommandSelected(MergeWorktree) on a session that has uncommitted changes increments mock.inspect_session_changes_calls() to 1 within 1s with the request targeting s-1 (the merge-confirm dialog is only pushed AFTER the inspect response folds via OpenMergeConfirmDialog — covered end-to-end by merge_worktree_rpc057.rs)
  #
  # QUESTIONS (ANSWERED):
  #   Q: The matrix lists `Tab → Turn-selection mode` but the Rust AgentView has no Tab handler / no turn_selection_mode flag today (only Tab cycling inside dialogs). Should this card include implementing Tab turn-selection (out-of-scope creep + 5+ extra pts), or write the test as `#[ignore = "pending turn-selection wiring — RPC-XYZ"]` so the matrix entry is documented but yellow, or drop the row from the matrix entirely?
  #   A: Add a #[tokio::test] for Tab turn-selection mode but mark it #[ignore = "Tab turn-selection mode pending future RPC card — placeholder behaviour-parity assertion documented but not yet wired in the Rust AgentView"]. The test body should compile (so the assertion contract is locked in) and a future RPC card simply removes the #[ignore] when Tab handling lands. Zero scope creep on RPC-065.
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the TS Ink AgentView to the Rust ratatui frontend
    I want to have a single behaviour-parity test suite that drives the AgentView through MockBackend and asserts every slash command and keyboard shortcut produces the same store-state transitions the TS frontend would
    So that I can regress-detect any divergence between the two frontends with one cargo test invocation, and every future RPC card has a reusable AppTestHarness to build on instead of re-rolling fixtures

  # ─────────────────────────────────────────────────────────────────────
  # SLASH COMMANDS — matrix rows 1-18
  # ─────────────────────────────────────────────────────────────────────
  Scenario: /help pushes the HelpDialog onto the compositor
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/help"
    Then the compositor contains a layer with id "help-dialog"

  Scenario: /quit flips the should_quit flag without touching the backend
    Given a fresh AppTestHarness with focused session s-1
    And the app's should_quit flag is false
    When I dispatch the slash command "/quit"
    Then the app's should_quit flag is true
    And the MockBackend has received no calls

  Scenario: /clear resets the focused session's scrollback and calls backend.clear_history
    Given a fresh AppTestHarness with focused session s-1 seeded with 5 scrollback chunks
    When I dispatch the slash command "/clear"
    Then the focused session's scrollback chunk count is 0 synchronously
    And within 1 second MockBackend.clear_history_calls() is 1
    And MockBackend.last_clear_history_session() is Some(s-1)

  Scenario: /model pushes the ModelSelectorDialog onto the compositor
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/model"
    Then the compositor contains a layer with id MODEL_SELECTOR_DIALOG_ID

  Scenario: /thinking (bare) pushes the ThinkingLevelDialog onto the compositor
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/thinking"
    Then the compositor contains a layer with id THINKING_LEVEL_DIALOG_ID

  Scenario: /thinking high inline sets the level without opening the dialog
    Given a fresh AppTestHarness with focused session s-1
    When I submit the input text "/thinking high"
    Then the compositor does NOT contain a layer with id THINKING_LEVEL_DIALOG_ID
    And within 1 second MockBackend.set_thinking_level_calls() is 1
    And MockBackend.last_set_thinking_level() is Some((s-1, ThinkingLevel::High))

  Scenario: /role (bare from palette) pushes the RoleDialog onto the compositor
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/role"
    Then the compositor contains a layer with id ROLE_DIALOG_ID

  Scenario: /role <text> inline sets the session role without opening the dialog
    Given a fresh AppTestHarness with focused session s-1
    When I submit the input text "/role You are a security reviewer"
    Then the compositor does NOT contain a layer with id ROLE_DIALOG_ID
    And within 1 second MockBackend.set_session_role_calls() is 1
    And MockBackend.last_set_session_role() is Some((s-1, Some("You are a security reviewer")))

  Scenario: /resume opens the ResumeSessionView mode view
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/resume"
    Then the AgentView's resume_view is Some(_)

  Scenario: /search opens the SearchHistoryView mode view with no backend call
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/search"
    Then the AgentView's search_view is Some(_)
    And MockBackend.search_history_calls() is 0

  Scenario: /provider activates the ProviderSettings view mode
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/provider"
    And I drain pending tasks and actions
    Then the navigator's active_view is ViewMode::ProviderSettings

  Scenario: /debug calls backend.toggle_debug for the focused session
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/debug"
    Then within 1 second MockBackend.toggle_debug_calls() is 1
    And MockBackend.last_toggle_debug() references session s-1

  Scenario: /compact calls backend.compact_session for the focused session
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/compact"
    Then within 1 second MockBackend.compact_session_calls() is 1
    And MockBackend.last_compact_session() is Some(s-1)

  Scenario: /isolation opens the CreateSessionDialog pre-selected to Isolated
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/isolation"
    And I drain pending tasks and actions
    Then the compositor contains a layer with id CREATE_SESSION_DIALOG_ID

  Scenario: /blocklist activates the Blocklist view mode
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/blocklist"
    And I drain pending tasks and actions
    Then the navigator's active_view is ViewMode::Blocklist

  Scenario: /detach clears the work-unit context for the focused session
    Given a fresh AppTestHarness with focused session s-1 bound to a WorkUnitContext
    When I dispatch the slash command "/detach"
    Then within 1 second MockBackend.set_work_unit_context_calls() is 1
    And MockBackend.last_set_work_unit_context() is Some((s-1, None))

  Scenario: /merge-worktree calls backend.inspect_session_changes for the focused session
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/merge-worktree"
    Then within 1 second MockBackend.inspect_session_changes_calls() is 1

  Scenario: /schedule (bare from palette) emits a static help notice into scrollback
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/schedule"
    Then the focused session's scrollback gains a help notice chunk

  Scenario: /schedule list invokes backend.schedule_list
    Given a fresh AppTestHarness with focused session s-1
    When I submit the input text "/schedule list"
    Then within 1 second MockBackend.schedule_list_calls() is 1

  Scenario: /loop (bare from palette) emits a static help notice into scrollback
    Given a fresh AppTestHarness with focused session s-1
    When I dispatch the slash command "/loop"
    Then the focused session's scrollback gains a help notice chunk

  Scenario: /loop list invokes backend.loop_list
    Given a fresh AppTestHarness with focused session s-1
    When I submit the input text "/loop list"
    Then within 1 second MockBackend.loop_list_calls() is 1

  # ─────────────────────────────────────────────────────────────────────
  # KEYBOARD SHORTCUTS — matrix rows 19-25
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Shift+Right cycles focus forward through open sessions
    Given a fresh AppTestHarness with two open sessions s-1 and s-2, focused on s-1
    When I press Shift+Right
    Then the focused session is s-2
    When I press Shift+Left
    Then the focused session is s-1

  Scenario: Shift+Up loads the most-recent history entry into the input
    Given a fresh AppTestHarness with focused session s-1
    And MockBackend has scripted persistence history ["old1", "old2"] for s-1
    When I press Shift+Up
    Then within 1 second the input value is "old2"

  @ignore
  Scenario: Tab on a session with seeded scrollback enters turn-selection mode (placeholder for future RPC card)
    Given a fresh AppTestHarness with focused session s-1 seeded with 3 scrollback chunks
    When I press Tab
    Then the AgentView is in turn-selection mode
    And a turn-selection cursor is visible

  Scenario: Ctrl+R opens the SearchHistoryView with no backend call
    Given a fresh AppTestHarness with focused session s-1
    When I press Ctrl+R
    Then the AgentView's search_view is Some(_)
    And MockBackend.search_history_calls() is 0

  Scenario: Esc with one HelpDialog on the compositor pops that dialog
    Given a fresh AppTestHarness with focused session s-1 and a HelpDialog on the compositor
    When I press Esc
    Then the compositor does NOT contain a layer with id "help-dialog"
    And the input value is unchanged

  Scenario: Enter on plain text forwards the value to backend.send_input
    Given a fresh AppTestHarness with focused session s-1
    When I submit the input text "hello world"
    Then within 1 second MockBackend.send_input_calls() is 1
    And MockBackend.last_send_input() is Some((s-1, "hello world"))

  Scenario: Ctrl+C while the focused session is Running calls backend.interrupt
    Given a fresh AppTestHarness with focused session s-1
    And the focused session's status is SessionStatus::Running
    When I press Ctrl+C
    Then within 1 second MockBackend.interrupt_calls() is 1
    And MockBackend.last_interrupt() is Some(s-1)

  Scenario: PageDown advances the scrollback viewport and End jumps to the bottom
    Given a fresh AppTestHarness with focused session s-1 seeded with 30 scrollback chunks
    And the scrollback is scrolled to the top
    When I press PageDown
    Then the scrollback viewport has advanced by one page
    When I press End
    Then the scrollback is at the bottom
