@git
@watcher
@bug
@BUG-182
Feature: Git State Watcher Core

  """
  `GitStateWatcher` (codelet-core, `src/git_state.rs`) is the ONE
  centralized git polling mechanism for the workspace cwd. This feature
  covers the WATCHER-level behavior (the TUI fold / mux / refresh
  scenarios live in spec/features/git-state-watcher.feature):

  - A debounced `notify` watch over `.git/` (recursive when a repo,
    non-recursive root watch for `.git` creation otherwise);
  - PLUS a `tokio::time::interval` periodic poll (default 10 seconds;
    R8: the interval is configurable via `with_interval` so tests inject
    short intervals such as 100-250ms — the dedup scenario below uses a
    100ms poll interval exactly for this reason);
  - A `tokio::sync::broadcast` channel (capacity 64) on which a fresh
    snapshot is published ONLY when it differs from the last published
    one (dedup — unchanged poll ticks never re-broadcast).

  The snapshot combines `git_branch` (codelet_git::status),
  `checkpoint_counts` (codelet_git::ghost_commit::count_checkpoints),
  `changed_files` (staged + unstaged + untracked) and `checkpoints`
  (most-recent-first, capped at 200). Non-repo cwd degrades to an empty
  `GitState` + a root watch for `.git` creation; debouncer setup failure
  degrades to poll-only with the static snapshot.
  """

  Background: User Story
    As a TUI developer
    I want a single watcher-level contract for the GitState stream
    So that the App/transport fold (git-state-watcher.feature) has one reliable source of fresh snapshots

  Scenario: Manual checkpoint creation publishes a GitState frame with incremented counts on the fs-watch
    Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel


  Scenario: The periodic poll catches a working-tree change that emits no .git event
    Given a GitStateWatcher watching a temp git repo with a clean working tree
    When the modified file's content is rewritten WITHOUT any .git event and the watcher's poll interval elapses
    Then the watcher publishes a GitState frame whose changed_files list reflects the new state


  Scenario: GitStateWatcher dedups unchanged snapshots across poll ticks
    Given a GitStateWatcher watching a temp git repo with a fixed snapshot and a 100ms poll interval
    When four poll ticks elapse without any repository change
    Then only the initial snapshot was broadcast — a subscriber that counts frames for 600ms observes exactly 1 frame
