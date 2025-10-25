export const cleanupCheckpointsHelp = `
SYNOPSIS
  fspec cleanup-checkpoints <workUnitId> --keep-last <N>

DESCRIPTION
  Delete old checkpoints for a work unit while preserving the most recent N checkpoints.
  Helps manage checkpoint retention and avoid accumulating too many old save points.

  Deletion is based on creation timestamp (oldest checkpoints deleted first).
  Displays summary of deleted and preserved checkpoints with timestamps.

ARGUMENTS
  <workUnitId>
    The ID of the work unit to cleanup checkpoints for (e.g., AUTH-001, UI-002)

OPTIONS
  --keep-last <N>
    Number of most recent checkpoints to preserve (required)
    Older checkpoints beyond this count will be deleted

WHEN TO USE
  - After many experiments leave too many checkpoints
  - To manage checkpoint retention periodically
  - When you're done with a work unit and want to cleanup
  - To keep only recent checkpoints and remove old ones
  - As part of work unit completion workflow

PREREQUISITES
  - Work unit must exist
  - At least one checkpoint must exist for the work unit
  - Git repository must be initialized

TYPICAL WORKFLOW
  1. Create many checkpoints during experimentation
  2. List checkpoints: fspec list-checkpoints AUTH-001
  3. Decide how many to keep (e.g., last 5)
  4. Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5
  5. Verify: fspec list-checkpoints AUTH-001

EXAMPLES
  # Keep last 5 checkpoints, delete rest
  $ fspec cleanup-checkpoints AUTH-001 --keep-last 5

  Cleaning up checkpoints for AUTH-001 (keeping last 5)...

  Deleted 7 checkpoint(s):
    - checkpoint-1 (2025-10-20T10:00:00.000Z)
    - checkpoint-2 (2025-10-20T11:00:00.000Z)
    - checkpoint-3 (2025-10-20T12:00:00.000Z)
    - baseline (2025-10-20T13:00:00.000Z)
    - before-refactor-old (2025-10-20T14:00:00.000Z)
    - experiment-failed (2025-10-20T15:00:00.000Z)
    - test-checkpoint (2025-10-20T16:00:00.000Z)

  Preserved 5 checkpoint(s):
    - current-state (2025-10-21T14:30:00.000Z)
    - after-optimization (2025-10-21T13:15:00.000Z)
    - working-version (2025-10-21T12:00:00.000Z)
    - AUTH-001-auto-implementing (2025-10-21T10:45:00.000Z)
    - AUTH-001-auto-testing (2025-10-21T09:30:00.000Z)

  ✓ Cleanup complete: 7 deleted, 5 preserved

  # Keep only last checkpoint
  $ fspec cleanup-checkpoints UI-002 --keep-last 1

  Cleaning up checkpoints for UI-002 (keeping last 1)...

  Deleted 4 checkpoint(s):
    - baseline (2025-10-20T10:00:00.000Z)
    - experiment-1 (2025-10-20T11:00:00.000Z)
    - experiment-2 (2025-10-20T12:00:00.000Z)
    - before-final (2025-10-20T13:00:00.000Z)

  Preserved 1 checkpoint(s):
    - final-version (2025-10-21T14:00:00.000Z)

  ✓ Cleanup complete: 4 deleted, 1 preserved

  # No cleanup needed (already have ≤ N checkpoints)
  $ fspec cleanup-checkpoints BUG-003 --keep-last 10

  Checkpoints for BUG-003: 3 total (≤ 10)
  No cleanup needed - all checkpoints preserved

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'

  Error: No checkpoints found for AUTH-001
    → Work unit has no checkpoints to cleanup
    → Create checkpoints first with 'fspec checkpoint'

  Error: --keep-last option is required
    → You must specify how many checkpoints to keep
    → Use: fspec cleanup-checkpoints AUTH-001 --keep-last 5

COMMON PATTERNS
  # Cleanup after experimentation phase
  # (You tried 15 approaches, want to keep last 3)
  fspec list-checkpoints AUTH-001
  # ... see 15 checkpoints
  fspec cleanup-checkpoints AUTH-001 --keep-last 3

  # Cleanup when marking work unit done
  fspec update-work-unit-status AUTH-001 done
  fspec cleanup-checkpoints AUTH-001 --keep-last 2
  # ... keep final state + one backup

  # Regular checkpoint maintenance
  fspec cleanup-checkpoints AUTH-001 --keep-last 5
  fspec cleanup-checkpoints UI-002 --keep-last 5
  fspec cleanup-checkpoints BUG-003 --keep-last 5

  # Cleanup all work units (manual loop)
  # AI can iterate through work units and cleanup each one
  for id in AUTH-001 UI-002 BUG-003; do
    fspec cleanup-checkpoints $id --keep-last 5
  done

RELATED COMMANDS
  fspec checkpoint <workUnitId> <checkpointName>         Create manual checkpoint
  fspec restore-checkpoint <workUnitId> <checkpointName> Restore a checkpoint
  fspec list-checkpoints <workUnitId>                    List all checkpoints

NOTES
  - Deletion is permanent - checkpoints cannot be recovered after cleanup
  - Checkpoints are deleted oldest-first based on creation timestamp
  - Both automatic and manual checkpoints are included in cleanup
  - No dry-run option - deletion happens immediately
  - Preserves exactly N most recent checkpoints (not N+1)
  - If work unit has ≤ N checkpoints, no deletion occurs

AI AGENT GUIDANCE
  - Cleanup periodically to avoid accumulating too many checkpoints
  - Typical retention: --keep-last 5 is reasonable for active work
  - For completed work units: --keep-last 1 or 2 is sufficient
  - Always list checkpoints first if you want to see what will be deleted
  - Cleanup is permanent - make sure user doesn't need old checkpoints
  - Consider asking user before cleanup if many checkpoints will be deleted
`;
