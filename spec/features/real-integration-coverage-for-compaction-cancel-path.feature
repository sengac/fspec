@done
@test-coverage
@test-maintenance
@resilience
@context-window
@testing
@cli
@CMPCT-030
Feature: Replace tautological compaction tests with real integration coverage

  """
  Tests exercise the public compaction helpers exported by `codelet_cli::interactive` (classify_compaction_branch, begin_compaction_recovery, flush_partial_state_before_compaction, compaction_retry_prompt, execute_compaction, validate_no_orphan_tool_calls, reconcile_session_messages, inject_synthetic_tool_results_for_orphans). No production code changes — test-only refactor. Shared OnceLock tempdir helper mirrors the pattern used by compaction_error_cascade_test.rs / compaction_tool_call_preservation_test.rs. Structural grep assertions back up invariants (Gemini continuation using the unified helper, watchdog helpers being invoked) that would otherwise require extracting a process_stream function from run_agent_stream_internal — an explicit non-goal of this card.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tautological tests (e.g. `assert!("PromptCancelled".contains("PromptCancelled"))`) must be deleted because they pass without invoking any production code
  #   2. Every replacement test must invoke at least one real production helper (imported from codelet_cli / codelet_core) and assert on its observable output
  #   3. No production code refactoring is in scope — only test files may be created, deleted, or modified
  #   4. Structural/grep assertions are acceptable when they protect a contract that cannot otherwise be exercised without a production refactor (e.g. 'gemini_continuation.rs calls classify_compaction_branch')
  #   5. Tests that reuse debug-capture must call the shared `ensure_test_data_dir()` OnceLock helper (mirrors CMPCT-027/028/029 pattern) rather than re-implementing it
  #
  # EXAMPLES:
  #   1. Delete the entire gemini_continuation_compaction_test.rs file — all three scenarios assert `true == true` or `"PromptCancelled".contains("PromptCancelled")`
  #   2. Replace with a new integration file that (a) grep-asserts gemini_continuation.rs calls classify_compaction_branch + begin_compaction_recovery and (b) directly invokes those helpers to verify policy propagation
  #   3. Replace empty_turn_compaction_fix_test.rs — all three tests build `should_compact` locally instead of calling production logic
  #   4. New empty-turn tests must invoke the real `convert_messages_to_turns` production helper and assert on its output, not reconstruct the gate locally
  #   5. Delete `test_normal_compaction_no_watchdog` from compaction_convergence_watchdog_test.rs — it sets a local AtomicBool to false and asserts it is false; the remaining tests in that file use real helpers and should stay
  #
  # ========================================

  Background: User Story
    As a fspec developer
    I want to replace the tautological compaction placeholder tests with real integration coverage that invokes production helpers
    So that each CMPCT-023..029 fix has a regression test that will actually fail if the implementation regresses

  Scenario: Gemini continuation uses the unified classify_compaction_branch helper on stream error
    Given the source file codelet/cli/src/interactive/gemini_continuation.rs is readable
    When the source is scanned for a classify_compaction_branch call
    Then the call is present and the source does NOT reimplement a bespoke string-match on the error


  Scenario: Gemini continuation invokes begin_compaction_recovery with pop_user_prompt=false on Recover
    Given a classify_compaction_branch call site has been located in gemini_continuation.rs
    When the following block is inspected for the unified recovery helper call
    Then begin_compaction_recovery is invoked with pop_user_prompt=false


  Scenario: Stream-end-with-compaction-flag path in Gemini continuation selects ResumeFromPartial when partial text was saved
    Given a fresh session and an empty streaming display
    When flush_partial_state_before_compaction is invoked and the selected policy is chosen per the in-file rule
    Then the selected CompactionRecoveryPolicy is ResumeFromPartial
    And a non-empty assistant_text buffer carrying partial text


  Scenario: Stream-end-with-compaction-flag path in Gemini continuation selects EmbedInInstruction when no partial text was saved
    Given a fresh session and an empty streaming display
    When flush_partial_state_before_compaction is invoked and the selected policy is chosen per the in-file rule
    Then the selected CompactionRecoveryPolicy is EmbedInInstruction
    And an empty assistant_text buffer


  Scenario: Tautological gemini_continuation_compaction_test.rs has been deleted from the cli tests directory
    Given a clean workspace with the cli tests directory present
    When the tests directory is listed
    Then gemini_continuation_compaction_test.rs is absent


  Scenario: convert_messages_to_turns returns an empty slice for a session with only system reminders
    Given a rig Message vec that contains only a system reminder User message
    When convert_messages_to_turns is invoked
    Then the returned slice is empty and the compaction gate `has_compactable_turns` is false


  Scenario: convert_messages_to_turns returns one turn per user-assistant pair
    Given a rig Message vec with three User-followed-by-Assistant text pairs
    When convert_messages_to_turns is invoked
    Then the returned slice contains exactly three turns


  Scenario: Tautological empty_turn_compaction_fix_test.rs has been deleted from the cli tests directory
    Given a clean workspace with the cli tests directory present
    When the tests directory is listed
    Then empty_turn_compaction_fix_test.rs is absent


  Scenario: Tautological test_normal_compaction_no_watchdog has been deleted from compaction_convergence_watchdog_test.rs
    Given the compaction_convergence_watchdog_test.rs source file is present
    When the file is scanned for a fn named test_normal_compaction_no_watchdog
    Then no such function definition is found

