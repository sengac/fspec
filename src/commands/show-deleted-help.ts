import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-deleted',
  description:
    'Display all soft-deleted items with IDs, text, and deletedAt timestamps for debugging and selective restoration',
  usage: 'fspec show-deleted <workUnitId>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-deleted AUTH-001',
      description: 'Show all deleted items with timestamps',
      output:
        'Deleted items for AUTH-001:\n\n[1] Old business rule (deleted: 2025-01-31T12:00:00.000Z)\n[3] Obsolete example (deleted: 2025-01-31T13:30:00.000Z)\n\nTotal: 2 deleted items',
    },
  ],
  notes: [
    'Shows items with deleted: true flag',
    'Displays stable IDs that can be used with restore commands',
    'Shows deletedAt timestamps in ISO 8601 format',
    'Useful for debugging and selective restoration',
    'Returns empty list if no deleted items exist',
  ],
  aiGuidance: [
    'Use before restore-* commands to see available deleted items',
    'IDs shown are stable and can be used directly with restore commands',
    'Check timestamps to understand when items were deleted',
    'Use before compact-work-unit to see what will be permanently removed',
  ],
  relatedCommands: [
    'restore-rule',
    'restore-example',
    'restore-question',
    'restore-architecture-note',
    'compact-work-unit',
    'show-work-unit',
  ],
};

export default config;
