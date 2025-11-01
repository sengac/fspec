import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'compact-work-unit',
  description:
    'Permanently remove soft-deleted items and renumber IDs sequentially (destructive operation)',
  usage: 'fspec compact-work-unit <workUnitId>',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description:
        'Skip confirmation prompt and compact during non-done status (use with caution)',
    },
  ],
  examples: [
    {
      command: 'fspec compact-work-unit AUTH-001',
      description:
        'Compact work unit with confirmation prompt (safe, requires user input)',
      output:
        '⚠ Warning: This will permanently remove 3 deleted items\nContinue? (y/N) y\n✓ Compacted AUTH-001\nRemoved 3 deleted items, renumbered remaining 7 items to IDs [0-6]',
    },
    {
      command: 'fspec compact-work-unit AUTH-001 --force',
      description:
        'Force compact without confirmation (use during non-done status)',
      output:
        '⚠ Warning: Compacting during "specifying" status permanently removes deleted items\n✓ Compacted AUTH-001\nRemoved 3 deleted items',
    },
  ],
  notes: [
    'Compaction is PERMANENT and cannot be undone',
    'Removes all soft-deleted items (deleted: true)',
    'Renumbers remaining items sequentially starting from 0',
    'Resets nextId counters to match remaining item count',
    'Preserves chronological order (sorted by createdAt timestamp)',
    'Auto-compact triggers when moving work unit to "done" status',
    'Requires --force flag when compacting during non-done status',
  ],
  aiGuidance: [
    'Use show-deleted to preview what will be removed before compacting',
    'Compaction is destructive - items cannot be restored after compaction',
    'Auto-compact happens automatically on "done" status transition',
    'Manual compact is useful for cleaning up during development',
    'Use --force only when you understand the consequences',
  ],
  relatedCommands: ['show-deleted', 'restore-rule', 'update-work-unit-status'],
};

export default config;
