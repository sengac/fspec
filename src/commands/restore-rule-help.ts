import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'restore-rule',
  description:
    'Restore soft-deleted business rule by stable ID (undeletes item and clears deletedAt timestamp)',
  usage: 'fspec restore-rule <workUnitId> <index>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'index',
      description: 'Rule ID (stable index from show-work-unit or show-deleted)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--ids <ids>',
      description:
        'Restore multiple rules with comma-separated IDs (e.g., "2,5,7")',
    },
  ],
  examples: [
    {
      command: 'fspec restore-rule AUTH-001 2',
      description: 'Restore deleted rule with ID 2',
      output:
        '✓ Restored rule from AUTH-001\nRule reappears at index [2] in show-work-unit',
    },
    {
      command: 'fspec restore-rule AUTH-001 --ids 2,5,7',
      description: 'Restore multiple rules at once (bulk restore)',
      output: '✓ Restored 3 rules from AUTH-001',
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
    'remove-rule',
    'show-work-unit',
    'show-deleted',
    'compact-work-unit',
  ],
};

export default config;
