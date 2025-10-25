import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-work-unit',
  description: 'Delete a work unit and all its data',
  usage: 'fspec delete-work-unit <workUnitId> [options]',
  whenToUse:
    'Use this command to permanently remove a work unit from the project. This deletes all associated data including Example Mapping results, dependencies, and state tracking. Use with caution as this operation cannot be undone.',
  prerequisites: ['Work unit must exist in spec/work-units.json'],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to delete (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Force deletion without validation checks',
    },
    {
      flag: '--skip-confirmation',
      description: 'Skip confirmation prompt (use in scripts)',
    },
    {
      flag: '--cascade-dependencies',
      description: 'Remove all dependency relationships before deleting',
    },
  ],
  examples: [
    {
      command: 'fspec delete-work-unit AUTH-999',
      description: 'Delete work unit (with confirmation)',
      output:
        'Are you sure? (y/N): y\n✓ Work unit AUTH-999 deleted successfully',
    },
    {
      command: 'fspec delete-work-unit AUTH-999 --skip-confirmation',
      description: 'Delete without confirmation',
      output: '✓ Work unit AUTH-999 deleted successfully',
    },
    {
      command: 'fspec delete-work-unit AUTH-999 --cascade-dependencies',
      description: 'Delete and remove all dependencies',
      output:
        '✓ Work unit AUTH-999 deleted successfully\n⚠ This work unit blocks 2 work unit(s): API-001, UI-001',
    },
    {
      command: 'fspec delete-work-unit AUTH-999 --force --skip-confirmation',
      description: 'Force delete without any checks',
      output: '✓ Work unit AUTH-999 deleted successfully',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Work unit AUTH-999 does not exist',
      fix: 'Verify work unit ID with: fspec list-work-units',
    },
    {
      error:
        'Error: Cannot delete work unit with children: AUTH-002, AUTH-003. Delete children first or remove parent relationship.',
      fix: 'Either delete child work units first, or remove the parent relationship from children',
    },
    {
      error:
        'Error: Work unit AUTH-001 has dependencies. Use --cascade-dependencies flag to remove dependencies and delete.',
      fix: 'Add --cascade-dependencies flag to remove all dependency relationships before deletion',
    },
  ],
  typicalWorkflow:
    '1. Verify work unit: fspec show-work-unit <id> → 2. Check dependencies: fspec dependencies <id> → 3. Delete: fspec delete-work-unit <id> --cascade-dependencies',
  commonPatterns: [
    {
      pattern: 'Safe Deletion with Checks',
      example:
        '# Check what will be deleted\nfspec show-work-unit AUTH-999\nfspec dependencies AUTH-999\n\n# Delete with cascading\nfspec delete-work-unit AUTH-999 --cascade-dependencies',
    },
    {
      pattern: 'Scripted Deletion',
      example:
        '# Delete in automated scripts\nfspec delete-work-unit AUTH-999 --skip-confirmation --cascade-dependencies',
    },
  ],
  relatedCommands: [
    'list-work-units',
    'create-story',
    'create-bug',
    'create-task',
    'show-work-unit',
    'dependencies',
  ],
  notes: [
    'Deletion is permanent and cannot be undone',
    'Deletes all Example Mapping data associated with the work unit',
    'Removes work unit from all state arrays (backlog, specifying, etc.)',
    "Removes work unit from parent's children array if it has a parent",
    'Cannot delete work units that have children (delete children first)',
    'Use --cascade-dependencies to remove all dependency relationships automatically',
    '--force flag bypasses validation checks (use with caution)',
    '--skip-confirmation is useful for automated scripts',
  ],
};

export default config;
