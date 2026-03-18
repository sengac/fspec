@done
@WATCH-024
Feature: Refactor watcher terminology to supervisor/subordinate with ChainOfCommand graph
  """
  Rust scope: 13 files — session_manager.rs (~200+ occurrences, primary), types.rs (StreamChunk variants), navigation.rs (WatchGraph usage), 5 test files. Do NOT touch work_units_watcher.rs or lib.rs (filesystem watcher, different concept).
  TypeScript scope: ~69 files — 8 core components (WatcherCreateView, WatcherTemplateList, WatcherTemplateForm, SplitSessionView, SessionHeader, AgentView, TurnContentModal, BoardView), 7 utils, 4 hooks, 2 store files, 2 types files, ~30 test files, and ~10 non-TUI TypeScript files
  Feature files scope: 28 domain-relevant feature files + their .coverage counterparts. ~8 filesystem/git watcher features must NOT be touched (different concept). 3 tags to retag: @watcher, @watcher-management, @watcher-input
  StreamChunk JSON wire format changes: 'watcherInput' → 'supervisorInput', 'watcherPendingInjection' → 'supervisorPendingInjection'. All TypeScript chunk type checks and chunkProcessor.ts regex parsing must be updated to match.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rename WatchGraph to ChainOfCommand — same two-HashMap structure, same cycle prevention, just renamed fields: subordinate_to_supervisors (1:N) and supervisor_to_subordinate (1:1)
  #   2. Remove the RoleAuthority enum entirely (Peer/Supervisor) — the brief field provides all behavioral instruction, no artificial authority levels needed
  #   3. Rename SessionRole to SupervisorRole — fields: name, brief (renamed from description), auto_inject. Drop the authority field.
  #   4. Rename all NAPI bindings: sessionCreateWatcher → sessionCreateSupervisor, sessionGetWatchers → sessionGetSupervisors, sessionGetParent → sessionGetSubordinate, watcherInject → supervisorInject
  #   5. Rename watcher_agent_loop → supervisor_agent_loop, watcher_loop_tick → supervisor_loop_tick, run_watcher_loop → run_supervisor_loop, WatcherState → SupervisorState, WatcherOutput → SupervisorOutput
  #   6. Rename WatcherInput → SupervisorInput, WatcherPendingInjection → SupervisorPendingInjection in StreamChunk enum variants
  #   7. Rename broadcast channel field: watcher_broadcast → supervisor_broadcast, and injection channel: watcher_input_tx/rx → supervisor_input_tx/rx
  #   8. Update TUI components: WatcherCreateView → SupervisorCreateView, WatcherTemplateList → SupervisorTemplateList, WatcherTemplateForm → SupervisorTemplateForm, useWatcherHeaderInfo → useSupervisorHeaderInfo
  #   9. Update watcher template storage: watcher-templates.json → supervisor-templates.json, WatcherTemplate type → SupervisorTemplate, all storage utility function names accordingly
  #   10. Remove the authority field from TUI template forms, NAPI bindings (sessionSetRole), injection message formatting, and evaluation prompts — everywhere RoleAuthority was used
  #   11. Update injection message prefix format from [WATCHER: role | Authority: level | Session: id] to [SUPERVISOR: role | Session: id]
  #   12. Update navigation.rs: build_navigation_list() references to watcher → supervisor, and split view header from [WATCHER] to [SUPERVISOR]
  #   13. Pure refactoring — zero behavioral changes. All existing tests must pass after renaming. No new functionality introduced.
  #   14. Remove the /parent command entirely — it's pointless. Users can navigate with Shift+Arrow.
  #   15. Remove sessionClearRole entirely — it's dead code with no consumers.
  #   16. The function is called internally in Rust by auto-inject (line 5899). Keep the function (rename to supervisor_inject), but remove the #[napi] export — no TypeScript consumer needs it.
  #
  # EXAMPLES:
  #   1. ChainOfCommand.add_supervisor(subordinate_id, supervisor_id) registers the relationship and prevents cycles — same logic as old WatchGraph.add_watcher but with renamed methods
  #   2. SupervisorRole has 3 fields: name (String), brief (Option<String>), auto_inject (bool) — authority field is gone
  #   3. Injection message reads [SUPERVISOR: security-reviewer | Session: abc-123] instead of [WATCHER: security-reviewer | Authority: Peer | Session: abc-123]
  #   4. TUI template form has 4 fields: Name, Model, Brief, Auto-inject — Authority toggle is removed
  #
  # QUESTIONS (ANSWERED):
  #   Q: The /watcher slash command needs renaming to /supervisor, but what about /parent? From a supervisor session, /parent takes you back to the subordinate — but /subordinate is ugly as a user command. Should it become /main or stay /parent?
  #   A: Remove the /parent command entirely — it's pointless. Users can navigate with Shift+Arrow.
  #
  #   Q: work_units_watcher.rs is a FILE SYSTEM watcher (watches spec/work-units.json for changes), NOT a session watcher. It has its own WatcherState struct (name collision). This file, useWorkUnitsWatcher hook, startWorkUnitsWatcher/stopWorkUnitsWatcher NAPI bindings must NOT be renamed — they are a completely different concept.
  #   A: Not relevant to this card. work_units_watcher.rs is a filesystem watcher, completely separate concept. Do not touch it.
  #
  #   Q: 28 feature files reference watcher/parent/authority terminology. Should these feature files be renamed too (e.g., watcher-injection-message-format.feature → supervisor-injection-message-format.feature), or just update the content inside them?
  #   A: Don't rename existing feature files — they're done cards, leave them as-is. Only new feature files for new functionality should use the new supervisor/subordinate naming.
  #
  #   Q: Tags @watcher, @watcher-management, @watcher-input need re-registering as @supervisor, @supervisor-management, @supervisor-input. Should the old tags be removed via retag command or left as aliases?
  #   A: Keep existing @watcher tags on old feature files. Register new @supervisor, @supervisor-management, @supervisor-input tags for new feature files going forward.
  #
  #   Q: sessionClearRole is exported from NAPI but has ZERO consumers in any TypeScript file. Is it dead code that should be removed, or is it intentionally available for future use?
  #   A: Remove sessionClearRole entirely — it's dead code with no consumers.
  #
  #   Q: watcherInject is exported from NAPI but has ZERO TypeScript consumers (only in index.d.ts). Auto-inject handles injection internally in Rust. Is manual inject from TypeScript needed, or is this dead code?
  #   A: The function is called internally in Rust by auto-inject (line 5899). Keep the function (rename to supervisor_inject), but remove the #[napi] export — no TypeScript consumer needs it.
  #
  #   Q: codelet/napi/index.d.ts is auto-generated from Rust NAPI bindings — renaming the Rust functions will auto-update this file. It should NOT be manually edited.
  #   A: Yes, auto-generated from #[napi] macros. Rust renames will auto-update it on build. Do not manually edit.
  #
  # ========================================
  Background: User Story
    As a developer working on the codelet codebase
    I want to see consistent supervisor/subordinate terminology throughout the watcher system instead of parent/watcher
    So that the code accurately reflects the architectural relationship — supervisors observe and interject, subordinates do the work

  @supervisor
  Scenario: ChainOfCommand replaces WatchGraph with renamed methods
    Given the WatchGraph struct has been renamed to ChainOfCommand
    When I call ChainOfCommand.add_supervisor(subordinate_id, supervisor_id)
    Then the relationship is registered in subordinate_to_supervisors and supervisor_to_subordinate maps
    And cycle prevention still works identically to the old add_watcher logic

  @supervisor
  Scenario: SupervisorRole replaces SessionRole without authority field
    Given the SessionRole struct has been renamed to SupervisorRole
    Then SupervisorRole has a name field of type String
    And SupervisorRole has a brief field of type Option<String>
    And SupervisorRole has an auto_inject field of type bool
    And SupervisorRole does not have an authority field

  @supervisor
  Scenario: RoleAuthority enum is removed entirely
    Given the RoleAuthority enum previously had Peer and Supervisor variants
    When the refactoring is complete
    Then the RoleAuthority enum no longer exists in the codebase
    And no code references Peer or Supervisor authority levels
    And the brief field provides all behavioral instruction instead

  @supervisor
  Scenario: Injection message uses simplified supervisor prefix
    Given a supervisor session with role "security-reviewer" and session ID "abc-123"
    When the supervisor injects a message into the subordinate session
    Then the message prefix reads "[SUPERVISOR: security-reviewer | Session: abc-123]"
    And the prefix does not include an Authority field

  @supervisor
  Scenario: TUI template form removes Authority toggle
    Given the supervisor template creation form is displayed
    Then the form has a Name field
    And the form has a Model field
    And the form has a Brief field
    And the form has an Auto-inject toggle
    And the form does not have an Authority toggle

  @supervisor
  Scenario: NAPI bindings use supervisor terminology
    Given the Rust NAPI bindings have been renamed
    Then sessionCreateSupervisor is exported instead of sessionCreateWatcher
    And sessionGetSupervisors is exported instead of sessionGetWatchers
    And sessionGetSubordinate is exported instead of sessionGetParent
    And sessionClearRole is no longer exported

  @supervisor
  Scenario: supervisor_inject is internal Rust only
    Given the watcher_inject function has been renamed to supervisor_inject
    Then supervisor_inject does not have a #[napi] annotation
    And supervisor_inject is called internally by the auto-inject path in the supervisor agent loop
    And no TypeScript code imports or calls supervisorInject

  @supervisor
  Scenario: StreamChunk variants use supervisor naming
    Given the StreamChunk enum has been updated
    Then the WatcherInput variant is renamed to SupervisorInput
    And the WatcherPendingInjection variant is renamed to SupervisorPendingInjection
    And the JSON wire format emits "supervisorInput" instead of "watcherInput"
    And the JSON wire format emits "supervisorPendingInjection" instead of "watcherPendingInjection"

  @supervisor
  Scenario: Supervisor agent loop uses renamed functions and types
    Given the watcher agent loop code has been refactored
    Then watcher_agent_loop is renamed to supervisor_agent_loop
    And watcher_loop_tick is renamed to supervisor_loop_tick
    And run_watcher_loop is renamed to run_supervisor_loop
    And WatcherState is renamed to SupervisorState
    And WatcherOutput is renamed to SupervisorOutput

  @supervisor
  Scenario: BackgroundSession fields use supervisor naming
    Given the BackgroundSession struct has been updated
    Then the watcher_broadcast field is renamed to supervisor_broadcast
    And the watcher_input_tx field is renamed to supervisor_input_tx
    And the watcher_input_rx field is renamed to supervisor_input_rx

  @supervisor
  Scenario: TUI components use supervisor naming
    Given the TUI component files have been renamed
    Then WatcherCreateView is renamed to SupervisorCreateView
    And WatcherTemplateList is renamed to SupervisorTemplateList
    And WatcherTemplateForm is renamed to SupervisorTemplateForm
    And useWatcherHeaderInfo is renamed to useSupervisorHeaderInfo

  @supervisor
  Scenario: Template storage uses supervisor naming
    Given the template storage system has been updated
    Then templates are stored in supervisor-templates.json instead of watcher-templates.json
    And the WatcherTemplate type is renamed to SupervisorTemplate
    And loadWatcherTemplates is renamed to loadSupervisorTemplates
    And saveWatcherTemplates is renamed to saveSupervisorTemplates

  @supervisor
  Scenario: Slash command renamed from /watcher to /supervisor
    Given the slash command registry has been updated
    Then the /watcher command is renamed to /supervisor
    And the /parent command is removed entirely

  @supervisor
  Scenario: Navigation references use supervisor terminology
    Given navigation.rs has been updated
    Then build_navigation_list references supervisor instead of watcher
    And the split view header displays [SUPERVISOR] instead of [WATCHER]

  @supervisor
  Scenario: Filesystem watcher is not affected by refactoring
    Given work_units_watcher.rs is a filesystem watcher for spec/work-units.json
    When the supervisor/subordinate refactoring is complete
    Then work_units_watcher.rs is unchanged
    And useWorkUnitsWatcher hook is unchanged
    And startWorkUnitsWatcher NAPI binding is unchanged
    And stopWorkUnitsWatcher NAPI binding is unchanged

  @supervisor
  Scenario: All existing tests pass after renaming
    Given all watcher terminology has been renamed to supervisor/subordinate
    When the full test suite is executed
    Then all tests pass with zero behavioral changes
