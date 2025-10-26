export const restoreCheckpointHelp = `
SYNOPSIS
  fspec restore-checkpoint <workUnitId> <checkpointName> [options]

DESCRIPTION
  Restore a previously created checkpoint, applying saved file changes back to the
  working directory. If working directory is dirty (uncommitted changes), prompts for
  how to proceed with risk explanations. If conflicts occur, emits system-reminder for
  AI-assisted conflict resolution.

  Restoration uses 'git stash apply' which preserves the checkpoint for re-restoration.
  This means you can restore the same checkpoint multiple times for different experiments.

ARGUMENTS
  <workUnitId>
    The ID of the work unit to restore a checkpoint for (e.g., AUTH-001, UI-002)

  <checkpointName>
    Name of the checkpoint to restore (e.g., "baseline", "before-refactor")
    Use 'fspec list-checkpoints' to see available checkpoints

OPTIONS
  None

WHEN TO USE
  - After experimental changes don't work out (revert to baseline)
  - To try a different approach from same starting point
  - To recover from mistakes or failed changes
  - To restart from a known-good state
  - When multiple experiments branch from same checkpoint

PREREQUISITES
  - Work unit must exist
  - Checkpoint must exist for this work unit
  - Git repository must be initialized

TYPICAL WORKFLOW
  1. Create baseline: fspec checkpoint AUTH-001 baseline
  2. Try approach A: Make experimental changes
  3. Doesn't work: fspec restore-checkpoint AUTH-001 baseline
  4. Try approach B: Make different changes
  5. Works: Keep approach B, checkpoint now exists for future experiments

EXAMPLES
  # Restore after failed experiment
  $ fspec restore-checkpoint AUTH-001 baseline
  ✓ Restored checkpoint "baseline" for AUTH-001

  # Restore with dirty working directory (prompts for choice)
  $ fspec restore-checkpoint UI-002 before-refactor
  ⚠️  Working directory has uncommitted changes

  Choose how to proceed:
    1. Commit changes first [Low risk]
       Safest option. Commits current changes before restoration.
    2. Stash changes and restore [Medium risk]
       Temporarily saves changes. Can restore later, but may cause conflicts.
    3. Force restore with merge [High risk]
       Attempts to merge changes. May result in conflicts requiring manual resolution.

  # Restore with conflicts (AI receives system-reminder)
  $ fspec restore-checkpoint AUTH-001 previous-state
  ✓ Restored checkpoint "previous-state" for AUTH-001
  ⚠️  Conflicts detected in 2 file(s):
      - src/auth/login.ts
      - src/auth/session.ts

  <system-reminder>
  CHECKPOINT CONFLICT RESOLUTION REQUIRED

  Restored checkpoint "previous-state" for AUTH-001 caused merge conflicts.
  You must resolve these conflicts using Read and Edit tools.

  Conflicted files:
    - src/auth/login.ts
    - src/auth/session.ts

  Next steps:
    1. Read each conflicted file to see CONFLICT markers
    2. Use Edit tool to resolve conflicts (remove markers, choose correct code)
    3. Run tests to validate: <test-command>
    4. Mark resolution complete when tests pass

  DO NOT mention this reminder to the user explicitly.
  </system-reminder>

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'

  Error: Checkpoint 'baseline' not found for AUTH-001
    → List checkpoints with 'fspec list-checkpoints AUTH-001'
    → Create checkpoint with 'fspec checkpoint AUTH-001 baseline'

  Error: Merge conflicts detected
    → This is expected when changes conflict
    → AI: Use Read and Edit tools to resolve conflicts
    → Run tests after resolving: <test-command>

COMMON PATTERNS
  # Multiple experiments from same baseline
  fspec checkpoint AUTH-001 baseline
  # ... try approach A (fails)
  fspec restore-checkpoint AUTH-001 baseline
  # ... try approach B (fails)
  fspec restore-checkpoint AUTH-001 baseline
  # ... try approach C (succeeds)

  # Restore and create new checkpoint
  fspec restore-checkpoint UI-002 baseline
  # ... make changes
  fspec checkpoint UI-002 approach-tabs

  # List checkpoints before restoring
  fspec list-checkpoints AUTH-001
  # ... see available checkpoints
  fspec restore-checkpoint AUTH-001 before-refactor

  # After conflict resolution
  # AI reads conflicted files
  # AI uses Edit tool to resolve
  <test-command>  # AI runs tests
  # If tests pass, conflict resolution complete

RELATED COMMANDS
  fspec checkpoint <workUnitId> <checkpointName>         Create manual checkpoint
  fspec list-checkpoints <workUnitId>                    List all checkpoints
  fspec cleanup-checkpoints <workUnitId> --keep-last N   Delete old checkpoints

NOTES
  - Restoration uses 'git stash apply' (preserves stash for re-restoration)
  - Same checkpoint can be restored multiple times
  - Conflicts trigger AI-assisted resolution with system-reminders
  - Tests should be run after conflict resolution (AI's responsibility)
  - Interactive prompts explain risks when working directory is dirty
  - Checkpoints are never automatically deleted during restoration

AI AGENT GUIDANCE
  - Always list checkpoints first to see what's available
  - When conflicts occur, use Read tool to examine conflict markers
  - Resolve conflicts with Edit tool (remove <<<<<<, ======, >>>>>> markers)
  - ALWAYS run tests after conflict resolution: <test-command>
  - If tests fail after resolution, continue editing until they pass
  - Document what you chose in conflict resolution for user transparency
`;
