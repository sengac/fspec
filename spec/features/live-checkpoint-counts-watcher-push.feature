@BUG-582
@done
@bug-181
@BUG-355
@header
@tui
@rpc
@checkpoint
@watcher
@bug
@parity
@regression
@source-shape
Feature: Live checkpoint counts watcher push
  """
  Split out of live-checkpoint-counts-in-the-board-header.feature by BUG-355 so each feature maps to exactly one test file (1 feature file = 1 test file). These scenarios pin the GitStateWatcher half of BUG-181 (superseded by BUG-182): the watcher publishes full GitState snapshots whose checkpoint_counts track every checkpoint ref mutation, degrades to an empty GitState outside a git repository, and is exposed through a transport-agnostic FspecBackend push surface. The App and transport delivery scenarios stay in live-checkpoint-counts-in-the-board-header.feature.
  Test file: rust/fspec-tui/tests/bug181_checkpoint_watcher_push.rs (helpers shared through tests/common/checkpoint_repo.rs).
  """

  Background: User Story
    As a developer using the fspec TUI board
    I want the git-state watcher to publish fresh checkpoint counts whenever a checkpoint ref is added or removed, from any code path
    So that every TUI transport has one live source for the board header's checkpoint counter

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

  Scenario: FspecBackend exposes a transport-agnostic git-state push surface
    Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-182 lands
    When a developer scans the source trees
    Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method git_state_changed_rx returning broadcast::Receiver<GitState>
    And SharedFspecService in rpc/src/lib.rs exposes git_state_changed_rx and git_state_snapshot
    And the GitStateWatcher type exists in codelet-core and app/bootstrap.rs subscribes to git_state_changed_rx
