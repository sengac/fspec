@done
@cli
@checkpoint
@high
@cross-platform
@refactoring
@ipc
@tui
@TUI-016
Feature: Refactor checkpoint counts to be command-triggered instead of file-watching
  """
  Uses Node.js net module for cross-platform IPC (Unix domain sockets on Linux/Mac, named pipes on Windows). Shared utility in src/utils/ipc.ts provides getIPCPath(), createIPCServer(), sendIPCMessage(), cleanupIPCServer(). TUI runs IPC server in BoardView (lifecycle: mount → listen, unmount → cleanup). Checkpoint commands (checkpoint.ts, update-work-unit-status.ts, restore-checkpoint.ts, cleanup-checkpoints.ts) send IPC messages after operations. Zustand store holds checkpointCounts state with loadCheckpointCounts() action. CheckpointPanel renders from store (reactive). Removes chokidar dependency entirely.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CheckpointPanel must update counts when manual checkpoints are created via 'fspec checkpoint' command
  #   2. CheckpointPanel must update counts when automatic checkpoints are created during status transitions
  #   3. CheckpointPanel must update counts when checkpoints are restored via 'fspec restore-checkpoint' command
  #   4. CheckpointPanel must update counts when checkpoints are cleaned up via 'fspec cleanup-checkpoints' command
  #   5. Chokidar file-watching dependency must be removed from CheckpointPanel
  #   6. Yes, TUI performs initial checkpoint counting on mount by calling useFspecStore.getState().loadCheckpointCounts(). This loads current state from disk. Subsequent updates come from IPC notifications.
  #   7. IPC solves this. When checkpoint commands execute in another terminal, they send IPC message to TUI's server (net.connect to socket/pipe). If TUI not running, send fails silently. No file-watching needed.
  #   8. Shared IPC utility (src/utils/ipc.ts) provides getIPCPath(), createIPCServer(), sendIPCMessage(), and cleanupIPCServer() functions
  #   9. Zustand store must have checkpointCounts state and loadCheckpointCounts() action that reads from .git/fspec-checkpoints-index
  #   10. IPC server must start when BoardView mounts and cleanup when unmounts (socket unlink on Unix, auto-cleanup on Windows)
  #   11. CheckpointPanel must ALWAYS render from zustand store state (useFspecStore) to ensure reactivity - never read directly from filesystem
  #
  # EXAMPLES:
  #   1. User creates manual checkpoint 'baseline' for TUI-016, CheckpointPanel shows updated manual count immediately
  #   2. User moves work unit from testing to implementing, auto checkpoint created, CheckpointPanel shows updated auto count immediately
  #   3. User runs 'fspec cleanup-checkpoints TUI-016', checkpoints deleted, CheckpointPanel shows updated counts immediately
  #   4. TUI starts on Linux, IPC server listens at /tmp/fspec-tui.sock, user creates checkpoint in another terminal, command sends message to socket, TUI updates counts
  #   5. TUI starts on Windows, IPC server listens at \\.\pipe\fspec-tui, user creates checkpoint, command connects to named pipe, sends message, TUI updates counts
  #   6. TUI not running, user creates checkpoint in terminal, IPC send fails silently (connection refused), checkpoint command completes successfully without error
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we use an event emitter pattern to notify the TUI of checkpoint changes, or should CheckpointPanel poll for updates at regular intervals?
  #   A: true
  #
  #   Q: When checkpoint commands are executed from within the TUI (if that's possible), should updates be synchronous or asynchronous?
  #   A: true
  #
  #   Q: Should the TUI continue to perform initial checkpoint counting on mount (like current implementation), or should all counts come from command-triggered updates?
  #   A: true
  #
  #   Q: If checkpoint commands are executed outside the TUI (in another terminal), how should the TUI detect those changes without file-watching?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Use cross-platform IPC (Unix domain sockets on Linux/Mac, named pipes on Windows) via Node.js net module. TUI runs IPC server, checkpoint commands send messages. No event emitter pattern, no polling - direct IPC communication.
  #   2. Updates are asynchronous. IPC messages trigger zustand store loadCheckpointCounts() which updates state asynchronously, causing CheckpointPanel to re-render via React hooks.
  #
  # ========================================
  Background: User Story
    As a developer viewing the TUI board
    I want to see checkpoint counts update immediately when checkpoint commands execute
    So that I have real-time visibility into checkpoints without file-watching overhead

  Scenario: Manual checkpoint creation triggers IPC update
    Given the TUI is running with IPC server listening
    And the CheckpointPanel displays "0 Manual, 0 Auto" checkpoints
    When I run "fspec checkpoint TUI-016 baseline" in a terminal
    Then the checkpoint command sends IPC message "checkpoint-changed"
    And the zustand store calls loadCheckpointCounts()
    And the CheckpointPanel displays "1 Manual, 0 Auto" checkpoints

  Scenario: Automatic checkpoint creation during status transition
    Given the TUI is running with IPC server listening
    And work unit TUI-016 is in "testing" status
    And the CheckpointPanel displays "0 Manual, 0 Auto" checkpoints
    When I run "fspec update-work-unit-status TUI-016 implementing"
    Then an automatic checkpoint is created with name "TUI-016-auto-testing"
    And the checkpoint command sends IPC message "checkpoint-changed"
    And the zustand store calls loadCheckpointCounts()
    And the CheckpointPanel displays "0 Manual, 1 Auto" checkpoints

  Scenario: Checkpoint cleanup triggers IPC update
    Given the TUI is running with IPC server listening
    And TUI-016 has 5 checkpoints (3 manual, 2 auto)
    And the CheckpointPanel displays "3 Manual, 2 Auto" checkpoints
    When I run "fspec cleanup-checkpoints TUI-016 --keep-last 2"
    Then 3 checkpoints are deleted
    And the checkpoint command sends IPC message "checkpoint-changed"
    And the zustand store calls loadCheckpointCounts()
    And the CheckpointPanel displays updated counts

  Scenario: IPC communication on Linux using Unix domain socket
    Given the TUI starts on Linux
    When the IPC server initializes
    Then it listens at "/tmp/fspec-tui.sock"
    And when a checkpoint command runs in another terminal
    Then the command connects to "/tmp/fspec-tui.sock"
    And sends JSON message {"type": "checkpoint-changed"}
    And the TUI receives the message and updates counts

  Scenario: IPC communication on Windows using named pipe
    Given the TUI starts on Windows
    When the IPC server initializes
    Then it listens at "\\\\.\\pipe\\fspec-tui"
    And when a checkpoint command runs
    Then the command connects to "\\\\.\\pipe\\fspec-tui"
    And sends JSON message {"type": "checkpoint-changed"}
    And the TUI receives the message and updates counts

  Scenario: Checkpoint command runs when TUI is not running
    Given the TUI is not running
    And no IPC server is listening
    When I run "fspec checkpoint TUI-016 baseline"
    Then the checkpoint command attempts to send IPC message
    And the connection fails with "ECONNREFUSED" error
    And the error is silently ignored
    And the checkpoint command completes successfully with exit code 0
    And the checkpoint is created in .git/fspec-checkpoints-index
