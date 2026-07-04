@done
@codelet
@validation
@session
@concurrency
@testing-framework
@rust
@PROV-132
Feature: Flaky rpc081 restore_messages test: intra-process race on global set_data_directory / default-model state

  """
  Isolation mechanism: a file-scoped `static DATA_DIR_GUARD: Mutex<()> = Mutex::new(())` in each offending test file (rpc081, prov101), locked at the top of every #[test] via `.lock().unwrap_or_else(PoisonError::into_inner)` and held (as `let _guard`) across the whole body. Chosen OVER per-test isolated data dir because the data directory is a single process-global (codelet/common/src/data_dir.rs `static DATA_DIRECTORY: Mutex<Option<PathBuf>>`) — swapping it is inherently non-isolatable across threads, so serialization is the only correct fix; it mirrors the proven PROV-118/119/123/127/129/130 guard pattern already in-tree.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every #[test] in a codelet-sessions integration binary that calls codelet_common::set_data_directory must acquire the file-scoped DATA_DIR_GUARD Mutex before mutating global state
  #   2. The guard must be held for the whole synchronous critical section that reads global state (data dir + default-model derived from it), released only after the test's assertions on that state complete
  #   3. Guard acquisition must be poison-tolerant (recover via PoisonError::into_inner) so one panicking test does not cascade-fail every serialized sibling
  #   4. Under repeated full-suite parallel runs, the malformed-envelope restore test and the no-default-model create_session decline test must pass deterministically (no first-run flake)
  #
  # EXAMPLES:
  #   1. rpc081 restore_messages_returns_err_on_malformed_envelope_json runs concurrently with prov101 create_session_declines_when_no_default_model; without a guard, prov101 seeds a data dir with NO default-model.json while rpc081 (in another binary) sets one, so a racing SessionManager::new() loads the wrong default and prov101's decline assertion fails on the first full-suite run
  #   2. Within the single rpc081 binary, 8 multi_thread tests each call set_data_directory + set_default_model concurrently; test A swaps the global data dir out from under test B mid-flight, so B's SessionManager::new() loads A's persisted default-model.json — serializing via DATA_DIR_GUARD makes each test see only its own seeded state
  #   3. After the fix, running the full codelet-sessions suite 3 times consecutively yields 3x all-green with zero flakes in rpc081 and prov101
  #
  # ========================================

  Background: User Story
    As a codelet-sessions integration test suite
    I want to serialize every test that swaps the process-global data directory and default-model state via a shared DATA_DIR_GUARD
    So that full-suite parallel runs are deterministic and the malformed-envelope and no-default-model tests never flake

  Scenario: rpc081 restore-messages tests serialize global data-dir access via a poison-tolerant guard
    Given the rpc081_restore_session_messages integration test file declares a file-scoped DATA_DIR_GUARD Mutex
    When any restore-messages test acquires the guard before calling set_data_directory
    Then the guard is locked poison-tolerantly via PoisonError::into_inner
    And the guard is bound to a live _guard binding held across the whole test body

  Scenario: prov101 no-default-model tests serialize global data-dir access via a poison-tolerant guard
    Given the prov101_no_selection_fallbacks integration test file declares a file-scoped DATA_DIR_GUARD Mutex
    When any no-selection-fallback test acquires the guard before seeding its data dir
    Then the guard is locked poison-tolerantly via PoisonError::into_inner
    And the guard is bound to a live _guard binding held across the whole test body

  Scenario: full codelet-sessions suite is deterministic across repeated parallel runs
    Given the offending rpc081 and prov101 tests each hold the DATA_DIR_GUARD across their critical section
    When the full codelet-sessions test suite runs three times consecutively in parallel
    Then every run reports zero failures
    And the malformed-envelope and no-default-model tests pass on every run with no first-run flake
