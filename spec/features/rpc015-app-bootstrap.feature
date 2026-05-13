@done
@RPC-015
@rust
@tui
@rpc
@board-view
@bootstrap
@state-management
Feature: RPC-015 App bootstrap dispatches CheckpointCountsLoaded into the BoardStore

  """
  RPC-015 (slice 2b of 3) — App bootstrap fires off `backend.checkpoint_counts()`
  alongside `list_work_units()` and emits `Action::CheckpointCountsLoaded(counts)`
  onto the action bus. `App::dispatch` handles the action by calling
  `board_store.set_checkpoint_counts(counts)`, after which the BoardView header
  paints the live counts on the next render.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want App::bootstrap to fetch and dispatch the checkpoint counts into the BoardStore
    So that the BoardView header paints live counts on the very first frame after bootstrap completes

  Scenario: BoardStore.checkpoint_counts is updated by Action::CheckpointCountsLoaded
    Given an App constructed with a backend that returns CheckpointCounts { manual: 2, auto: 3 } from checkpoint_counts()
    When App::bootstrap is awaited and the bootstrap task's spawned future delivers Action::CheckpointCountsLoaded
    And App::dispatch processes the action
    Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 3 }
