import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'restore-example',
  description:
    'Restore soft-deleted example by stable ID (undeletes item and clears deletedAt timestamp)',
  usage: 'fspec restore-example <workUnitId> <index>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description:
        'Example ID (stable index from show-work-unit or show-deleted)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--ids <ids>',
      description:
        'Restore multiple examples with comma-separated IDs (e.g., "2,5,7")',
    },
  ],
  examples: [
    {
      command: 'fspec restore-example AUTH-001 2',
      description: 'Restore deleted example with ID 2',
      output:
        '✓ Restored example from AUTH-001\nExample reappears at index [2] in show-work-unit',
    },
    {
      command: 'fspec restore-example AUTH-001 --ids 2,5,7',
      description: 'Restore multiple examples at once (bulk restore)',
      output: '✓ Restored 3 examples from AUTH-001',
    },
  ],
  notes: [
    'Uses stable IDs that never shift when items are removed',
    'Restoring an already-active item is idempotent (succeeds with message)',
    'Non-existent IDs will fail with clear error message',
    'Bulk restore validates all IDs before restoring any item',
  ],
  aiGuidance: [
    'Use show-deleted to see deleted items with IDs before restoring',
    'IDs are stable and never reused, even after compaction',
    'Restore operations set deleted=false and clear deletedAt field',
    'Bulk restore is atomic: all succeed or all fail',
  ],
  relatedCommands: [
    'remove-example',
    'show-work-unit',
    'show-deleted',
    'compact-work-unit',
  ],
};

export default config;
