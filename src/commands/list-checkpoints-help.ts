export const listCheckpointsHelp = `
SYNOPSIS
  fspec list-checkpoints <workUnitId>

DESCRIPTION
  List all checkpoints for a work unit, showing both automatic and manual checkpoints
  with visual indicators. Displays checkpoint names, types, and creation timestamps.

  Visual indicators:
  - 🤖 Automatic checkpoints (created during workflow status transitions)
  - 📌 Manual checkpoints (created with 'fspec checkpoint' command)

ARGUMENTS
  <workUnitId>
    The ID of the work unit to list checkpoints for (e.g., AUTH-001, UI-002)

OPTIONS
  None

WHEN TO USE
  - Before restoring a checkpoint (to see what's available)
  - To verify a checkpoint was created successfully
  - To see the history of checkpoints for a work unit
  - To decide which old checkpoints to cleanup
  - To understand automatic vs manual checkpoint patterns

PREREQUISITES
  - Work unit must exist
  - Git repository must be initialized

TYPICAL WORKFLOW
  1. Create checkpoints: fspec checkpoint AUTH-001 baseline
  2. List checkpoints: fspec list-checkpoints AUTH-001
  3. Choose checkpoint: See available names and timestamps
  4. Restore: fspec restore-checkpoint AUTH-001 baseline
  5. Or cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5

EXAMPLES
  # List all checkpoints for work unit
  $ fspec list-checkpoints AUTH-001

  Checkpoints for AUTH-001:

  📌  before-refactor (manual)
     Created: 2025-10-21T14:30:00.000Z

  📌  baseline (manual)
     Created: 2025-10-21T13:15:00.000Z

  🤖  AUTH-001-auto-testing (automatic)
     Created: 2025-10-21T10:00:00.000Z

  # List checkpoints for work unit with no checkpoints
  $ fspec list-checkpoints UI-002

  No checkpoints found for UI-002

  # List checkpoints after status transition
  $ fspec update-work-unit-status AUTH-001 implementing
  🤖 Auto-checkpoint: "AUTH-001-auto-testing" created before transition
  ✓ Work unit AUTH-001 status updated to implementing

  $ fspec list-checkpoints AUTH-001

  Checkpoints for AUTH-001:

  🤖  AUTH-001-auto-testing (automatic)
     Created: 2025-10-21T14:45:00.000Z

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-work-unit'

  No checkpoints found for <workUnitId>
    → Not an error - work unit has no checkpoints yet
    → Create a checkpoint with 'fspec checkpoint' or transition status

COMMON PATTERNS
  # List before restoring
  fspec list-checkpoints AUTH-001
  # ... see available checkpoints
  fspec restore-checkpoint AUTH-001 baseline

  # List to verify checkpoint created
  fspec checkpoint AUTH-001 before-refactor
  fspec list-checkpoints AUTH-001
  # ... confirm "before-refactor" appears

  # List to decide what to cleanup
  fspec list-checkpoints AUTH-001
  # ... see many old checkpoints
  fspec cleanup-checkpoints AUTH-001 --keep-last 5

  # List to understand automatic checkpoint pattern
  fspec list-checkpoints AUTH-001
  # ... see "AUTH-001-auto-testing", "AUTH-001-auto-specifying", etc.

RELATED COMMANDS
  fspec checkpoint <workUnitId> <checkpointName>         Create manual checkpoint
  fspec restore-checkpoint <workUnitId> <checkpointName> Restore a checkpoint
  fspec cleanup-checkpoints <workUnitId> --keep-last N   Delete old checkpoints
  fspec update-work-unit-status                          Auto-creates checkpoints

NOTES
  - All checkpoints are shown by default (both automatic and manual)
  - Checkpoints are sorted by creation time (newest first)
  - Automatic checkpoints use naming pattern: {workUnitId}-auto-{state}
  - Manual checkpoints use user-provided names
  - No pagination - all checkpoints for work unit are displayed

AI AGENT GUIDANCE
  - Always list checkpoints before restoring to see what's available
  - Use output to explain checkpoint history to user if asked
  - Automatic checkpoints show workflow progression (testing → implementing → etc.)
  - Manual checkpoints show user's experimental save points
  - Empty list is not an error - just means no checkpoints exist yet
`;
