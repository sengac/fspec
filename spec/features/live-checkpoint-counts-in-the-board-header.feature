@done
@bug-181
@header
@tui
@rpc
@checkpoint
@watcher
@bug
@parity
@regression
@source-shape
Feature: Live checkpoint counts in the board header

  """
  BUG-181: the BoardView header checkpoint counter (Checkpoints: N Manual, M Auto) only updated at bootstrap and after in-TUI restore/delete; checkpoints added/removed via agent tool calls, CLI in another terminal, or direct git ref ops never reached BoardStore.checkpoint_counts. Original fix: a CheckpointsWatcher in codelet-core published fresh CheckpointCounts on a tokio::sync::broadcast channel (capacity 64, full-snapshot payloads so lagging subscribers simply resync). Missing .git / non-repo cwd yielded zero counts with no error (matches count_checkpoints ENOENT tolerance). This single mechanism covered every mutation source: in-process agent tool calls (refs written on disk), CLI in another terminal, and raw git update-ref.

  SUPERSEDED by BUG-182 (spec/features/git-state-watcher.feature): the standalone CheckpointsWatcher is replaced by the ONE centralized GitStateWatcher. The checkpoint-changed push is folded into the GitState frame stream — every scenario below now describes the GitState stream (the checkpoint counts arrive as GitState.checkpoint_counts on the same debounced fs-watch), and the App subscriber path is the git-state stream folding onto Action::CheckpointCountsLoaded.
  """

  Background: User Story
    As a developer using the fspec TUI board
    I want to see the checkpoint counter update immediately whenever a checkpoint is added or removed, from any code path
    So that the board header is a trustworthy live view of checkpoint state instead of a stale bootstrap snapshot


  Scenario: Manual checkpoint creation publishes incremented counts on the checkpoint-changed push
    Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel


  Scenario: Auto-checkpoint creation publishes incremented counts on the checkpoint-changed push
    Given a GitStateWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 1 } on its broadcast channel


  Scenario: Checkpoint delete publishes decremented counts on the checkpoint-changed push
    Given a GitStateWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    Then the watcher publishes a GitState frame with checkpoint_counts { manual: 0, auto: 1 } on its broadcast channel


  Scenario: Watcher degrades to an empty GitState when the watched directory is not a git repository
    Given a GitStateWatcher watching a plain temp directory that has no .git
    When the watcher's initial snapshot is read and a file is later created inside that directory
    Then the initial snapshot is an empty GitState (zero checkpoint counts, empty changed_files and checkpoints lists)


  Scenario: App bootstrap subscriber folds pushed checkpoint counts into the BoardStore
    Given an App constructed with a backend whose git-state push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    When a GitState frame with checkpoint_counts { manual: 1, auto: 0 } is pushed onto the channel
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 }
    And bootstrap spawns exactly 7 subscriber tasks with the git-state stream replacing the checkpoint-changed stream (no separate checkpoint-changed subscriber remains)


  Scenario: Embedded transport delivers a live checkpoint-count frame end-to-end into the BoardStore
    Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with no checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    When a manual ghost-checkpoint ref is written into the repo while the App is running
    Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } without any TUI restart


  Scenario: WebSocket transport degrades gracefully when the git-state push is not forwarded
    Given an App wired to a WebSocketFspecBackend (remote transport, git-state push not forwarded), bootstrapped so the BoardStore holds the bootstrap CheckpointCounts { manual: 1, auto: 1 }
    When a checkpoint is added to the repo on the server side after bootstrap
    Then the App's git-state subscriber exits cleanly on a closed receiver without panicking or hanging


  Scenario: Opening and closing the Checkpoints view re-polls the checkpoint counts
    Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    When Action::OpenCheckpointsView is dispatched and then Action::CloseCheckpointsView is dispatched
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } so the header repaints with the live counts


  Scenario: RefreshCheckpointCounts re-reads the counts and updates the BoardStore
    Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    When Action::RefreshCheckpointCounts is dispatched (the follow-up the Checkpoints view emits after a successful in-view restore or delete)
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 }


  Scenario: FspecBackend exposes a transport-agnostic git-state push surface
    Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-182 lands
    When a developer scans the source trees
    Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method git_state_changed_rx returning broadcast::Receiver<GitState>
    And SharedFspecService in rpc/src/lib.rs exposes git_state_changed_rx and git_state_snapshot
    And the GitStateWatcher type exists in codelet-core and app/bootstrap.rs subscribes to git_state_changed_rx
