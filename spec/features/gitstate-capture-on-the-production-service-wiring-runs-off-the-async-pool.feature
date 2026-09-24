@done
@performance
@git
@watcher
@bug
@rpc
@BUG-187
Feature: GitState capture on the production service wiring runs off the async pool

  """
  BUG-187 companion (service-wiring half of the watcher blocking-pool fix).
  SharedFspecService::with_cwd (rust/rpc/src/lib.rs) wires the capture
  counter hook into the ONE centralized GitStateWatcher (BUG-182), so a
  re-capture that lands after a checkpoint ref is written from another
  terminal is COUNTED and PUBLISHED on the production wiring — proof the
  fs-event → spawn_blocking path actually runs in the service context.
  The off-async-pool proof itself (hook fires on the blocking-pool
  thread) lives in the watcher-level tests of
  spec/features/git-state-watcher-blocking-pool.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The production service wiring (with_cwd) passes the capture hook into the GitStateWatcher; only re-captures dispatched onto the blocking pool count — the synchronous initial (constructor) capture does not.
  #   2. A checkpoint ref written into .git from outside the process produces a fresh GitState frame (checkpoint_counts { manual: 1, auto: 0 }) within the poll/fs-watch window, delivered through the production wiring.
  #
  # EXAMPLES:
  #   1. A manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from another terminal; within 5 seconds the service's snapshot carries the new count AND the capture counter has ticked (the re-capture ran through the wiring's hook).
  #
  # ========================================

  Background: User Story
    As a TUI user supervising agents
    I want git state to stay live through the production wiring
    So that checkpoint and branch updates reach the panes without a restart

  Scenario: A checkpoint written from another terminal produces a counted capture on the production wiring
    Given a GitStateWatcher watching a temp git repo with no checkpoint refs
    When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from outside the process
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } within the poll/fs-watch window (the off-thread capture must not delay or drop the update)
