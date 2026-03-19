@SCHED-013
Feature: Per-Entry Spawned Task for Sub-Minute Loop Intervals

  """
  Uses tokio::spawn per entry — LoopStore becomes active task manager not passive data store
  LoopStore entries: HashMap<String, (LoopEntry, JoinHandle<()>)> — cancel/remove_for_session abort handles
  engine.rs: Remove evaluate_and_fire_loops() and its call in evaluate_and_run() Step 8
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each /loop entry spawns its own tokio task that sleeps for exactly the configured interval — no shared polling tick
  #   2. Cancelling a loop aborts its spawned task (JoinHandle::abort) — no lingering timers
  #   3. Loop tasks skip firing when the session is busy (not idle) — same skip policy as before, no queuing
  #   4. Loop tasks auto-terminate when the session is destroyed or the entry expires
  #   5. The cron engine tick (30s) is untouched — only /loop evaluation is decoupled from it
  #   6. evaluate_and_fire_loops() is removed from the engine tick — loops are fully self-managed
  #   7. LoopStore stores JoinHandle alongside LoopEntry — cancel/remove operations abort the handle
  #   8. Minimum interval is 1 second (enforced at registration) — prevents accidental tight loops
  #
  # EXAMPLES:
  #   1. User runs `/loop 5s check status` → task spawns, prompt fires after exactly 5 seconds, then every 5 seconds thereafter
  #   2. User runs `/loop 1s heartbeat` → prompt fires every 1 second (minimum allowed interval)
  #   3. User cancels loop via `/loop cancel <id>` → task stops immediately, no further firings
  #   4. Session is busy when loop interval elapses → loop skips that tick, tries again after the next interval
  #   5. Session is destroyed while loop is active → loop task self-terminates, no orphaned tasks
  #   6. Cron schedules at 30s tick continue working identically — no regression
  #
  # ========================================

  Background: User Story
    As a user
    I want to set a sub-minute /loop interval and have it fire at the exact cadence specified
    So that I get timely automated prompts without unexpected 30s quantization

  @scheduler @loop
  Scenario: Loop fires at exact sub-minute interval
    Given a session is active and idle
    When the user registers a loop with a 5-second interval
    Then the loop task spawns immediately
    And the prompt fires after exactly 5 seconds
    And the prompt continues firing every 5 seconds thereafter

  @scheduler @loop
  Scenario: Minimum interval is 1 second
    Given a session is active and idle
    When the user registers a loop with a 1-second interval
    Then the prompt fires every 1 second

  @scheduler @loop
  Scenario: Cancel aborts the spawned task immediately
    Given a session has an active loop
    When the user cancels the loop
    Then the loop task is aborted via JoinHandle
    And no further prompts fire for that loop

  @scheduler @loop
  Scenario: Skip firing when session is busy
    Given a session has an active loop with a 5-second interval
    And the session is currently busy
    When the loop interval elapses
    Then the prompt is not sent
    And the loop retries after the next interval

  @scheduler @loop
  Scenario: Auto-terminate when session is destroyed
    Given a session has an active loop
    When the session is destroyed
    Then the loop task self-terminates
    And no orphaned tasks remain in the LoopStore

  @scheduler @loop @cron
  Scenario: Cron engine tick is unaffected
    Given the cron scheduler is running with a 30-second tick
    And loop entries exist with sub-minute intervals
    When the cron tick fires
    Then cron schedules are evaluated as before
    And loop entries are not evaluated by the cron tick

  @scheduler @loop
  Scenario: Expired loop auto-terminates
    Given a session has an active loop that has reached its expiry time
    When the loop task checks expiry before sleeping
    Then the loop task terminates
    And the entry is removed from the LoopStore
