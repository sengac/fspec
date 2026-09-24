@done
@git
@watcher
@bug
@performance
@rpc
@BUG-187
Feature: GitStateWatcher pins a tokio worker for the whole 60s capture window — full GitState snapshot runs synchronously on an async worker every 10s + on every debounced .git fs-event

  """
  Fix lives in rust/core/src/git_state.rs (GitStateWatcher::with_interval poll task + build_debouncer callback). R1: wrap re_snapshot_and_publish in tokio::task::spawn_blocking on BOTH paths. R2: std::sync::atomic::AtomicBool in-flight guard — tick (and fs-event) that find a capture in flight are dropped, not queued; guard is cleared when the blocking work completes. R3: the notify debouncer callback is sync-only, so it captures a tokio::runtime::Handle (when available) and spawns the blocking work from it; when no runtime handle is available it degrades to a no-op (poll-only), matching the existing no-runtime warning. capture_git_state + re_snapshot_and_publish bodies stay unchanged (dedup + broadcast stay in place).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The GitState capture (capture_git_state / re_snapshot_and_publish) MUST run on the blocking pool (tokio::task::spawn_blocking), never inline on an async worker, on BOTH the periodic-poll path and the debouncer-callback path. A slow capture must not pin a tokio-rt-worker thread and starve other async tasks.
  #   2. R2: In-flight guard — when a tick fires while a capture is already in flight, the tick is DROPPED (skipped), never queued. An AtomicBool (or equivalent) tracks the in-flight capture so at most one capture runs at a time and the blocking pool never builds an unbounded backlog.
  #   3. R3: The debouncer (notify) callback is sync-only, so it must spawn the blocking capture from a captured tokio::runtime::Handle (degrading gracefully when no runtime is available) with the same in-flight skip guard — it must NOT call re_snapshot_and_publish inline.
  #   4. R4: macOS FSEvents reports event paths through the RESOLVED symlink form of the watched path (e.g. a workspace under `/var/folders/...` — where `/var` -> `/private/var` — yields `.git` events whose paths start with `/private/var/folders/...`). The `.git`-relevance check must therefore match against BOTH the raw watch prefix and its canonicalized form; an un-canonicalized prefix match silently drops every fs-event (the capture then only lands on the 10s poll tick).
  #
  # EXAMPLES:
  #   1. While the agent is busy (a long LLM turn is streaming) and the repo's git snapshot takes several seconds to compute, the TUI stays responsive: keystrokes still register, the agent's stream keeps flowing, and no 'frozen' hang is observed (previously a single slow capture pinned a runtime worker for the whole capture duration).
  #   2. When nothing in the repo changes and the 10-second poll keeps firing, the TUI observes zero wasted work: no repeated re-fetches of the changed-files / checkpoints panes, and no visible flicker (the capture still runs off-thread, and the dedup means no re-broadcast).
  #   3. When a checkpoint ref is written into .git from another terminal, the changed-files and checkpoints panes (if visible) update within the poll/fs-watch window as before — moving the capture off the async pool must NOT delay or drop the update.
  #   4. A workspace whose cwd root sits behind a filesystem symlink (macOS temp dirs: `/var/folders/...` -> `/private/var/folders/...`) updates its git branch via the fs-event path just as fast as a non-symlinked workspace — the branch change lands in well under one poll interval instead of waiting for the 10-second tick.
  #
  # ========================================

  Background: User Story
    As a TUI user supervising agents
    I want to keep the TUI responsive
    So that a slow GitState capture never pins a tokio worker and starves LLM stream dispatch / RPCs / the TUI

  Scenario: A slow GitState capture never blocks the other async work on the runtime
    Given a GitStateWatcher with a short poll interval watching a temp git repo
    And a concurrent async timer task on the same runtime firing at a short cadence
    When the watcher keeps taking GitState snapshots for several poll intervals
    Then the concurrent timer task fires on schedule throughout (its ticks are not delayed by the captures)
    And no tokio async worker is pinned by the capture work — the capture runs on the blocking pool (R1)


  Scenario: A tick that fires while a capture is in flight is dropped, not queued
    Given a GitStateWatcher whose capture takes longer than the poll interval
    When several poll ticks fire while the first capture is still running
    Then the in-flight guard skips those ticks — at most one capture ever runs at a time (R2)
    And after the in-flight capture completes, the next tick runs a fresh capture (the guard is cleared, not stuck)


  Scenario: Steady-state unchanged ticks still cost the user nothing observable
    Given a GitStateWatcher watching a temp git repo whose snapshot never changes
    When many poll ticks elapse
    Then exactly one frame (the initial one) was ever broadcast — the off-thread capture still dedups unchanged snapshots and publishes nothing further


  Scenario: A checkpoint written from another terminal still lands within the watch window
    Given a GitStateWatcher watching a temp git repo with no checkpoint refs
    When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from outside the process
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } within the poll/fs-watch window (the off-thread capture must not delay or drop the update)

  Scenario: A branch change in a symlinked-cwd workspace still lands via the fs-event path
    Given a GitStateWatcher whose workspace cwd sits behind a filesystem symlink (raw form differs from its canonicalized form)
    When the branch is switched to feature-x from another terminal
    Then the watcher publishes a GitState frame with git_branch feature-x within well under one poll interval — the fs-event path is NOT silently dropped by a raw-prefix-only match (R4)
