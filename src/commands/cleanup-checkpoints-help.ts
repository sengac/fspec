import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'cleanup-checkpoints',
  description:
    'Delete old checkpoints for a work unit while preserving the most recent N checkpoints. Helps manage checkpoint retention and avoid accumulating too many old save points. Deletion is based on creation timestamp (oldest checkpoints deleted first).',
  usage: 'fspec cleanup-checkpoints <workUnitId> --keep-last <N>',
  whenToUse:
    "Use after many experiments leave too many checkpoints, to manage checkpoint retention periodically, when you're done with a work unit and want to cleanup, to keep only recent checkpoints and remove old ones, or as part of work unit completion workflow.",
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to cleanup checkpoints for (e.g., AUTH-001, UI-002)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--keep-last <N>',
      description:
        'Number of most recent checkpoints to preserve (required). Older checkpoints beyond this count will be deleted.',
    },
  ],
  examples: [
    {
      command: 'fspec cleanup-checkpoints AUTH-001 --keep-last 5',
      description: 'Keep last 5 checkpoints, delete rest',
      output:
        'Cleaning up checkpoints for AUTH-001 (keeping last 5)...\n\nDeleted 7 checkpoint(s):\n  - checkpoint-1 (2025-10-20T10:00:00.000Z)\n  - checkpoint-2 (2025-10-20T11:00:00.000Z)\n  ...\n\nPreserved 5 checkpoint(s):\n  - current-state (2025-10-21T14:30:00.000Z)\n  - after-optimization (2025-10-21T13:15:00.000Z)\n  ...\n\n✓ Cleanup complete: 7 deleted, 5 preserved',
    },
    {
      command: 'fspec cleanup-checkpoints UI-002 --keep-last 1',
      description: 'Keep only last checkpoint',
      output:
        'Cleaning up checkpoints for UI-002 (keeping last 1)...\n\nDeleted 4 checkpoint(s)\nPreserved 1 checkpoint(s)\n\n✓ Cleanup complete: 4 deleted, 1 preserved',
    },
    {
      command: 'fspec cleanup-checkpoints BUG-003 --keep-last 10',
      description: 'No cleanup needed (already have ≤ N checkpoints)',
      output:
        'Checkpoints for BUG-003: 3 total (≤ 10)\nNo cleanup needed - all checkpoints preserved',
    },
  ],
  prerequisites: [
    'Work unit must exist',
    'At least one checkpoint must exist for the work unit',
    'Git repository must be initialized',
  ],
  typicalWorkflow: [
    'Create many checkpoints during experimentation',
    'List checkpoints: fspec list-checkpoints AUTH-001',
    'Decide how many to keep (e.g., last 5)',
    'Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5',
    'Verify: fspec list-checkpoints AUTH-001',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: 'No checkpoints found for AUTH-001',
      solution:
        "Work unit has no checkpoints to cleanup. Create checkpoints first with 'fspec checkpoint'.",
    },
    {
      error: '--keep-last option is required',
      solution:
        'You must specify how many checkpoints to keep. Use: fspec cleanup-checkpoints AUTH-001 --keep-last 5',
    },
  ],
  relatedCommands: [
    'fspec checkpoint - Create manual checkpoint',
    'fspec restore-checkpoint - Restore a checkpoint',
    'fspec list-checkpoints - List all checkpoints',
  ],
  notes: [
    'Deletion is permanent - checkpoints cannot be recovered after cleanup',
    'Checkpoints are deleted oldest-first based on creation timestamp',
    'Both automatic and manual checkpoints are included in cleanup',
    'No dry-run option - deletion happens immediately',
    'Preserves exactly N most recent checkpoints (not N+1)',
    'If work unit has ≤ N checkpoints, no deletion occurs',
  ],
};

export default config;
