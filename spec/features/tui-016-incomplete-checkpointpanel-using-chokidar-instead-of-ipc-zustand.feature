@critical
@tui
@ipc
@state-management
@bug-fix
@BUG-065
Feature: TUI-016 incomplete: CheckpointPanel using chokidar instead of IPC+Zustand
  """
  Complete TUI-016 IPC integration by connecting three components: Zustand store (add checkpointCounts state + loadCheckpointCounts action), BoardView (add IPC server lifecycle), CheckpointPanel (REMOVE chokidar, use store). Existing IPC infrastructure (src/utils/ipc.ts) and command integration (checkpoint.ts, update-work-unit-status.ts, cleanup-checkpoints.ts) already complete and tested. Files to modify: src/tui/store/fspecStore.ts (add state), src/tui/components/BoardView.tsx (add IPC server), src/tui/components/CheckpointPanel.tsx (remove chokidar lines 8,61,64-92, replace with store hooks).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CheckpointPanel MUST NOT use chokidar file-watching
  #   2. CheckpointPanel MUST read checkpoint counts from Zustand store
  #   3. Zustand store MUST have checkpointCounts state with manual and auto counts
  #   4. Zustand store MUST have loadCheckpointCounts() action to read from .git/fspec-checkpoints-index/
  #   5. BoardView MUST create IPC server on mount to listen for checkpoint-changed messages
  #   6. BoardView MUST call loadCheckpointCounts() when receiving checkpoint-changed IPC message
  #   7. BoardView MUST cleanup IPC server on unmount
  #
  # EXAMPLES:
  #   1. User runs 'fspec checkpoint TUI-016 baseline' in terminal A, TUI in terminal B sees count update from 20 to 21 manual checkpoints immediately
  #   2. User runs 'fspec update-work-unit-status TUI-016 implementing' triggering auto checkpoint, TUI shows count update from 58 to 59 auto checkpoints immediately
  #   3. User starts TUI, CheckpointPanel loads initial counts from filesystem showing 21 manual and 59 auto checkpoints
  #   4. User runs 'fspec cleanup-checkpoints TUI-016 --keep-last 5' deleting 15 checkpoints, TUI shows counts drop from 21 to 6 manual and 59 to 49 auto immediately
  #   5. CheckpointPanel imports from Zustand store, NOT chokidar - no chokidar import exists in file
  #   6. Zustand store has checkpointCounts state accessible via useFspecStore(state => state.checkpointCounts)
  #
  # ========================================
  Background: User Story
    As a TUI user monitoring checkpoint counts
    I want to see real-time checkpoint updates via IPC
    So that checkpoint display stays accurate without file-watching overhead

  Scenario: Manual checkpoint triggers IPC update to TUI
    Given TUI is running with BoardView IPC server listening
    Given CheckpointPanel displays 20 manual and 59 auto checkpoints
    When user runs 'fspec checkpoint TUI-016 baseline' in separate terminal
    Then checkpoint command sends IPC message type 'checkpoint-changed'
    Then BoardView IPC server receives the message
    Then store loadCheckpointCounts() is called
    Then CheckpointPanel displays 21 manual and 59 auto checkpoints

  Scenario: Auto checkpoint triggers IPC update to TUI
    Given TUI is running with BoardView IPC server listening
    Given CheckpointPanel displays 21 manual and 58 auto checkpoints
    When user runs 'fspec update-work-unit-status TUI-016 implementing' triggering auto checkpoint
    Then CheckpointPanel displays 21 manual and 59 auto checkpoints

  Scenario: TUI loads initial checkpoint counts on startup
    Given .git/fspec-checkpoints-index/ contains 21 manual and 59 auto checkpoint files
    When user starts TUI
    Then CheckpointPanel calls loadCheckpointCounts() on mount
    Then CheckpointPanel displays 21 manual and 59 auto checkpoints

  Scenario: Cleanup checkpoints triggers IPC update to TUI
    Given TUI is running with CheckpointPanel displaying 21 manual and 59 auto checkpoints
    When user runs 'fspec cleanup-checkpoints TUI-016 --keep-last 5' deleting 15 manual and 10 auto checkpoints
    Then CheckpointPanel displays 6 manual and 49 auto checkpoints

  Scenario: CheckpointPanel does not import chokidar
    Given CheckpointPanel component file exists at src/tui/components/CheckpointPanel.tsx
    When file is inspected for imports
    Then no import statement for 'chokidar' exists
    Then no chokidar watcher setup exists

  Scenario: Zustand store exposes checkpoint counts state
    Given Zustand store is defined in src/tui/store/fspecStore.ts
    When store interface is inspected
    Then checkpointCounts state exists with manual and auto number properties
    Then loadCheckpointCounts action exists as async function
    Then state is accessible via useFspecStore selector
