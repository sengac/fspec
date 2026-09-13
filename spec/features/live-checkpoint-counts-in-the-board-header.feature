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
  BUG-181: the BoardView header checkpoint counter (Checkpoints: N Manual, M Auto) only updates at bootstrap and after in-TUI restore/delete; checkpoints added/removed via agent tool calls, CLI in another terminal, or direct git ref ops never reach BoardStore.checkpoint_counts. Fix (shared-layer push, Rust-native): (1) a new CheckpointsWatcher in codelet-core (next to WorkUnitsWatcher) runs a debounced notify watch over the checkpoint locations (.git/refs/fspec-checkpoints/ recursively, .git/fspec-checkpoints-index/ recursively, .git/packed-refs) in the workspace cwd, re-runs codelet_git::ghost_commit::count_checkpoints on each debounced event, and publishes fresh CheckpointCounts on a tokio::sync::broadcast channel (capacity 64, full-snapshot payloads so lagging subscribers simply resync). Missing .git / non-repo cwd yields zero counts with no error (matches count_checkpoints ENOENT tolerance). This single mechanism covers every mutation source: in-process agent tool calls (refs written on disk), CLI in another terminal, and raw git update-ref. (2) SharedFspecService exposes checkpoint_counts_changed_rx() / checkpoint_counts_snapshot() mirroring watcher_rx()/watcher_snapshot(); the service builds the watcher from its cwd (no cwd → zero counts, no watcher). (3) FspecBackend gains checkpoint_counts_changed_rx() returning broadcast::Receiver<CheckpointCounts> with a CLOSED-receiver default (mirrors checkpoints_progress_rx, TUI-109) so test doubles (MockBackend) and the WebSocket transport compile unchanged; EmbeddedFspecBackend forwards the service receiver. (4) App::spawn_subscriber_tasks gains a subscriber that folds frames onto the EXISTING Action::CheckpointCountsLoaded → BoardStore::set_checkpoint_counts path (no new Action variant, no second writer path) and exits cleanly on RecvError::Closed. (5) WebSocket degradation (documented): the remote transport returns the closed-receiver default so the subscriber exits immediately — no panic, no hang; the header falls back to the bootstrap value plus re-polls on Open/CloseCheckpointsView (App::dispatch re-polls backend.checkpoint_counts() on both arms). The existing in-view RefreshCheckpointCounts follow-ups after restore/delete (RPC-365/366) remain unchanged. (6) Out of scope: auto-checkpoint index-entry write (Checkpoints-view ordering) and WebSocket envelope fan-out for remote-TUI live push — separate cards.
  """

  Background: User Story
    As a developer using the fspec TUI board
    I want to see the checkpoint counter update immediately whenever a checkpoint is added or removed, from any code path
    So that the board header is a trustworthy live view of checkpoint state instead of a stale bootstrap snapshot


  Scenario: Manual checkpoint creation publishes incremented counts on the checkpoint-changed push
    Given a CheckpointsWatcher watching a temp git repo that has no checkpoint refs
    When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    Then the watcher publishes CheckpointCounts { manual: 1, auto: 0 } on its broadcast channel


  Scenario: Auto-checkpoint creation publishes incremented counts on the checkpoint-changed push
    Given a CheckpointsWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    Then the watcher publishes CheckpointCounts { manual: 1, auto: 1 } on its broadcast channel


  Scenario: Checkpoint delete publishes decremented counts on the checkpoint-changed push
    Given a CheckpointsWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    Then the watcher publishes CheckpointCounts { manual: 0, auto: 1 } on its broadcast channel


  Scenario: Watcher degrades to zero counts when the watched directory is not a git repository
    Given a CheckpointsWatcher watching a plain temp directory that has no .git
    When the watcher's initial snapshot is read and a file is later created inside that directory
    Then the initial snapshot is CheckpointCounts { manual: 0, auto: 0 }


  Scenario: App bootstrap subscriber folds pushed checkpoint counts into the BoardStore
    Given an App constructed with a backend whose checkpoint-changed push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    When a frame CheckpointCounts { manual: 1, auto: 0 } is pushed onto the channel
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 }
    And bootstrap spawns a 7th subscriber task on the checkpoint-changed channel alongside the existing six


  Scenario: Embedded transport delivers a live checkpoint-count frame end-to-end into the BoardStore
    Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with no checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    When a manual ghost-checkpoint ref is written into the repo while the App is running
    Then the App's checkpoint-changed subscriber forwards the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } without any TUI restart


  Scenario: WebSocket transport degrades gracefully when the checkpoint-changed push is not forwarded
    Given an App wired to a WebSocketFspecBackend (remote transport, checkpoint-changed push not forwarded), bootstrapped so the BoardStore holds the bootstrap CheckpointCounts { manual: 1, auto: 1 }
    When a checkpoint is added to the repo on the server side after bootstrap
    Then the App's checkpoint-changed subscriber exits cleanly on a closed receiver without panicking or hanging


  Scenario: Opening and closing the Checkpoints view re-polls the checkpoint counts
    Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    When Action::OpenCheckpointsView is dispatched and then Action::CloseCheckpointsView is dispatched
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } so the header repaints with the live counts


  Scenario: RefreshCheckpointCounts re-reads the counts and updates the BoardStore
    Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    When Action::RefreshCheckpointCounts is dispatched (the follow-up the Checkpoints view emits after a successful in-view restore or delete)
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 }


  Scenario: FspecBackend exposes a transport-agnostic checkpoint-changed push surface
    Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-181 lands
    When a developer scans the source trees
    Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method checkpoint_counts_changed_rx returning broadcast::Receiver<CheckpointCounts>
    And SharedFspecService in rpc/src/lib.rs exposes checkpoint_counts_changed_rx and checkpoint_counts_snapshot
    And the CheckpointsWatcher type exists in codelet-core and app/bootstrap.rs subscribes to checkpoint_counts_changed_rx

