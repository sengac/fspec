import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'restore-checkpoint',
  description:
    "Restore a previously created checkpoint, applying saved file changes back to the working directory. Uses 'git stash apply' which preserves the checkpoint for re-restoration. If working directory is dirty, prompts for how to proceed. If conflicts occur, emits system-reminder for AI-assisted resolution.",
  usage: 'fspec restore-checkpoint <workUnitId> <checkpointName>',
  whenToUse:
    "Use after experimental changes don't work out (revert to baseline), to try a different approach from same starting point, to recover from mistakes or failed changes, to restart from a known-good state, or when multiple experiments branch from same checkpoint.",
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to restore a checkpoint for (e.g., AUTH-001, UI-002)',
      required: true,
    },
    {
      name: 'checkpointName',
      description:
        'Name of the checkpoint to restore (e.g., "baseline", "before-refactor"). Use \'fspec list-checkpoints\' to see available checkpoints.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec restore-checkpoint AUTH-001 baseline',
      description: 'Restore after failed experiment',
      output: '✓ Restored checkpoint "baseline" for AUTH-001',
    },
    {
      command: 'fspec restore-checkpoint UI-002 before-refactor',
      description: 'Restore with dirty working directory (prompts for choice)',
      output:
        '⚠️  Working directory has uncommitted changes\n\nChoose how to proceed:\n  1. Commit changes first [Low risk]\n     Safest option. Commits current changes before restoration.\n  2. Stash changes and restore [Medium risk]\n     Temporarily saves changes. Can restore later, but may cause conflicts.\n  3. Force restore with merge [High risk]\n     Attempts to merge changes. May result in conflicts requiring manual resolution.',
    },
    {
      command: 'fspec restore-checkpoint AUTH-001 previous-state',
      description: 'Restore with conflicts (AI receives system-reminder)',
      output:
        '✓ Restored checkpoint "previous-state" for AUTH-001\n⚠️  Conflicts detected in 2 file(s):\n    - src/auth/login.ts\n    - src/auth/session.ts\n\n<system-reminder>\nCHECKPOINT CONFLICT RESOLUTION REQUIRED\nUse Read and Edit tools to resolve conflicts.\n</system-reminder>',
    },
  ],
  prerequisites: [
    'Work unit must exist',
    'Checkpoint must exist for this work unit',
    'Git repository must be initialized',
  ],
  typicalWorkflow: [
    'Create baseline: fspec checkpoint AUTH-001 baseline',
    'Try approach A: Make experimental changes',
    "Doesn't work: fspec restore-checkpoint AUTH-001 baseline",
    'Try approach B: Make different changes',
    'Works: Keep approach B, checkpoint now exists for future experiments',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: "Checkpoint 'baseline' not found for AUTH-001",
      solution:
        "List checkpoints with 'fspec list-checkpoints AUTH-001' or create checkpoint with 'fspec checkpoint AUTH-001 baseline'",
    },
    {
      error: 'Merge conflicts detected',
      solution:
        'This is expected when changes conflict. AI: Use Read and Edit tools to resolve conflicts. Run tests after resolving.',
    },
  ],
  relatedCommands: [
    'fspec checkpoint - Create manual checkpoint',
    'fspec list-checkpoints - List all checkpoints',
    'fspec cleanup-checkpoints - Delete old checkpoints',
  ],
  notes: [
    "Restoration uses 'git stash apply' (preserves stash for re-restoration)",
    'Same checkpoint can be restored multiple times',
    'Conflicts trigger AI-assisted resolution with system-reminders',
    "Tests should be run after conflict resolution (AI's responsibility)",
    'Interactive prompts explain risks when working directory is dirty',
    'Checkpoints are never automatically deleted during restoration',
  ],
};

export default config;
