import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-checkpoints',
  description:
    'List all checkpoints for a work unit, showing both automatic and manual checkpoints with visual indicators (🤖 automatic, 📌 manual). Displays checkpoint names, types, and creation timestamps.',
  usage: 'fspec list-checkpoints <workUnitId>',
  whenToUse:
    "Use before restoring a checkpoint to see what's available, to verify a checkpoint was created successfully, to see the history of checkpoints for a work unit, to decide which old checkpoints to cleanup, or to understand automatic vs manual checkpoint patterns.",
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to list checkpoints for (e.g., AUTH-001, UI-002)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec list-checkpoints AUTH-001',
      description: 'List all checkpoints for work unit',
      output:
        'Checkpoints for AUTH-001:\n\n📌  before-refactor (manual)\n   Created: 2025-10-21T14:30:00.000Z\n\n📌  baseline (manual)\n   Created: 2025-10-21T13:15:00.000Z\n\n🤖  AUTH-001-auto-testing (automatic)\n   Created: 2025-10-21T10:00:00.000Z',
    },
    {
      command: 'fspec list-checkpoints UI-002',
      description: 'List checkpoints for work unit with no checkpoints',
      output: 'No checkpoints found for UI-002',
    },
  ],
  prerequisites: ['Work unit must exist', 'Git repository must be initialized'],
  typicalWorkflow: [
    'Create checkpoints: fspec checkpoint AUTH-001 baseline',
    'List checkpoints: fspec list-checkpoints AUTH-001',
    'Choose checkpoint: See available names and timestamps',
    'Restore: fspec restore-checkpoint AUTH-001 baseline',
    'Or cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: 'No checkpoints found for <workUnitId>',
      solution:
        "Not an error - work unit has no checkpoints yet. Create a checkpoint with 'fspec checkpoint' or transition status.",
    },
  ],
  relatedCommands: [
    'fspec checkpoint - Create manual checkpoint',
    'fspec restore-checkpoint - Restore a checkpoint',
    'fspec cleanup-checkpoints - Delete old checkpoints',
    'fspec update-work-unit-status - Auto-creates checkpoints',
  ],
  notes: [
    'All checkpoints are shown by default (both automatic and manual)',
    'Checkpoints are sorted by creation time (newest first)',
    'Automatic checkpoints use naming pattern: {workUnitId}-auto-{state}',
    'Manual checkpoints use user-provided names',
    'No pagination - all checkpoints for work unit are displayed',
    'Empty list is not an error - just means no checkpoints exist yet',
  ],
};

export default config;
