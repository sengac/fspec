export const checkpointHelp = `
SYNOPSIS
  fspec checkpoint <workUnitId> <checkpointName> [options]

DESCRIPTION
  Create a manual checkpoint for a work unit, capturing all file changes (including
  untracked files) as a git stash. Checkpoints enable safe experimentation by
  providing rollback points you can restore later.

  Checkpoints use git stash with a special message format:
  fspec-checkpoint:{workUnitId}:{checkpointName}:{timestamp}

  All checkpoints persist until explicitly deleted. Use checkpoints to try multiple
  approaches, create baselines for experiments, or save progress before risky changes.

ARGUMENTS
  <workUnitId>
    The ID of the work unit to create a checkpoint for (e.g., AUTH-001, UI-002)

  <checkpointName>
    User-provided name for the checkpoint (e.g., "baseline", "before-refactor")
    Name should be descriptive of why you're creating this checkpoint

OPTIONS
  None

WHEN TO USE
  - Before trying experimental approaches or risky refactoring
  - Creating a baseline before multiple experiments
  - Saving progress before switching contexts
  - Creating named save points during complex implementation
  - Before making changes you might want to revert

PREREQUISITES
  - Work unit must exist (created with 'fspec create-work-unit')
  - Working directory should have uncommitted changes to capture
  - Project must be a git repository

TYPICAL WORKFLOW
  1. Working on implementation: Make changes to code
  2. Create checkpoint: fspec checkpoint AUTH-001 baseline
  3. Try approach A: Make experimental changes
  4. Doesn't work: fspec restore-checkpoint AUTH-001 baseline
  5. Try approach B: Make different changes
  6. Works: Continue with approach B
  7. Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5

EXAMPLES
  # Create checkpoint before refactoring
  $ fspec checkpoint AUTH-001 before-refactor
  ✓ Created checkpoint "before-refactor" for AUTH-001
    Captured 3 file(s)

  # Create baseline for experiments
  $ fspec checkpoint UI-002 baseline
  ✓ Created checkpoint "baseline" for UI-002
    Captured 5 file(s)

  # Create checkpoint before risky change
  $ fspec checkpoint BUG-003 working-version
  ✓ Created checkpoint "working-version" for BUG-003
    Captured 2 file(s)

  # List all checkpoints after creating
  $ fspec list-checkpoints AUTH-001
  Checkpoints for AUTH-001:

  📌  before-refactor (manual)
     Created: 2025-10-21T14:30:00.000Z

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-work-unit'

  Error: Working directory is clean (no changes to checkpoint)
    → You can only create checkpoints when there are uncommitted changes
    → Make some changes first, then create the checkpoint

  Error: Not a git repository
    → Checkpoints require git. Initialize git with 'git init'

COMMON PATTERNS
  # Create baseline, try multiple approaches
  fspec checkpoint AUTH-001 baseline
  # ... try approach A (fails)
  fspec restore-checkpoint AUTH-001 baseline
  # ... try approach B (succeeds)

  # Create checkpoint before each experiment
  fspec checkpoint UI-002 experiment-1-tabs
  # ... implement tabs
  fspec restore-checkpoint UI-002 baseline
  fspec checkpoint UI-002 experiment-2-accordion
  # ... implement accordion

  # Create checkpoint before and after major changes
  fspec checkpoint REFACTOR-001 before-extraction
  # ... extract large function
  fspec checkpoint REFACTOR-001 after-extraction

  # View all checkpoints to decide which to restore
  fspec list-checkpoints AUTH-001

RELATED COMMANDS
  fspec restore-checkpoint <workUnitId> <checkpointName>  Restore a checkpoint
  fspec list-checkpoints <workUnitId>                     List all checkpoints
  fspec cleanup-checkpoints <workUnitId> --keep-last N    Delete old checkpoints
  fspec update-work-unit-status                          Auto-creates checkpoints

NOTES
  - Checkpoints capture ALL changes including untracked files (respecting .gitignore)
  - Checkpoints persist until explicitly deleted (no automatic expiration)
  - Manual checkpoints are marked with 📌 emoji in listings
  - Automatic checkpoints (from status changes) are marked with 🤖 emoji
  - Checkpoint names should be descriptive (avoid generic names like "temp", "test")
  - Use cleanup-checkpoints to manage checkpoint retention

AI AGENT GUIDANCE
  - Create checkpoints before experimental changes or risky refactoring
  - Use descriptive checkpoint names that explain WHY you're saving
  - Checkpoints are cheap - create them liberally before trying new approaches
  - Always create a "baseline" checkpoint before multiple experiments
  - List checkpoints with 'fspec list-checkpoints' before restoring
  - Clean up old checkpoints to avoid clutter: fspec cleanup-checkpoints <id> --keep-last 5
`;
