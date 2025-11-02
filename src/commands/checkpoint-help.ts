import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'checkpoint',
  description:
    'Create a manual checkpoint for a work unit, capturing all file changes (including untracked files) as a git stash. Checkpoints enable safe experimentation by providing rollback points you can restore later.',
  usage: 'fspec checkpoint <workUnitId> <checkpointName>',
  whenToUse:
    'Use checkpoints before trying experimental approaches or risky refactoring, creating a baseline before multiple experiments, saving progress before switching contexts, or before making changes you might want to revert.',
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to create a checkpoint for (e.g., AUTH-001, UI-002)',
      required: true,
    },
    {
      name: 'checkpointName',
      description:
        'User-provided name for the checkpoint (e.g., "baseline", "before-refactor"). Name should be descriptive of why you\'re creating this checkpoint.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec checkpoint AUTH-001 before-refactor',
      description: 'Create checkpoint before refactoring',
      output:
        '✓ Created checkpoint "before-refactor" for AUTH-001\n  Captured 3 file(s)',
    },
    {
      command: 'fspec checkpoint UI-002 baseline',
      description: 'Create baseline for experiments',
      output:
        '✓ Created checkpoint "baseline" for UI-002\n  Captured 5 file(s)',
    },
    {
      command: 'fspec checkpoint BUG-003 working-version',
      description: 'Create checkpoint before risky change',
      output:
        '✓ Created checkpoint "working-version" for BUG-003\n  Captured 2 file(s)',
    },
  ],
  prerequisites: [
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
    'Working directory should have uncommitted changes to capture',
    'Project must be a git repository',
  ],
  typicalWorkflow: [
    'Working on implementation: Make changes to code',
    'Create checkpoint: fspec checkpoint AUTH-001 baseline',
    'Try approach A: Make experimental changes',
    "Doesn't work: fspec restore-checkpoint AUTH-001 baseline",
    'Try approach B: Make different changes',
    'Works: Continue with approach B',
    'Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: 'Working directory is clean (no changes to checkpoint)',
      solution:
        'You can only create checkpoints when there are uncommitted changes. Make some changes first, then create the checkpoint.',
    },
    {
      error: 'Not a git repository',
      solution: "Checkpoints require git. Initialize git with 'git init'",
    },
  ],
  relatedCommands: [
    'fspec restore-checkpoint - Restore a checkpoint',
    'fspec list-checkpoints - List all checkpoints',
    'fspec cleanup-checkpoints - Delete old checkpoints',
    'fspec update-work-unit-status - Auto-creates checkpoints',
  ],
  notes: [
    'Checkpoints capture ALL changes including untracked files (respecting .gitignore)',
    'Checkpoints persist until explicitly deleted (no automatic expiration)',
    'Manual checkpoints are marked with 📌 emoji in listings',
    'Automatic checkpoints (from status changes) are marked with 🤖 emoji',
    'Checkpoint names should be descriptive (avoid generic names like "temp", "test")',
    'Use cleanup-checkpoints to manage checkpoint retention',
    'Checkpoints use git stash with format: fspec-checkpoint:{workUnitId}:{checkpointName}:{timestamp}',
  ],
};

export default config;
