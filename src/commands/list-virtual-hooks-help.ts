import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-virtual-hooks',
  description: 'List all virtual hooks configured for a work unit',
  usage: 'fspec list-virtual-hooks <workUnitId>',
  whenToUse:
    'Use this command to view all virtual hooks attached to a specific work unit. Shows hooks grouped by event with their configuration (blocking, git context, command). Useful for reviewing quality checks before workflow transitions or debugging hook execution.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to list hooks for (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec list-virtual-hooks AUTH-001',
      description: 'List all virtual hooks for AUTH-001',
      output: `Virtual Hooks for AUTH-001:

  post-implementing:
    • eslint [blocking]
      npm run lint
    • prettier [non-blocking]
      prettier --check .

  pre-validating:
    • typecheck [blocking]
      npm run typecheck`,
    },
    {
      command: 'fspec list-virtual-hooks BUG-042',
      description: 'List hooks for a work unit with no virtual hooks',
      output: 'No virtual hooks configured for BUG-042',
    },
    {
      command: 'fspec list-virtual-hooks FEAT-123',
      description: 'List hooks including git context hooks',
      output: `Virtual Hooks for FEAT-123:

  post-implementing:
    • eslint [blocking] [git-context]
      spec/hooks/.virtual/FEAT-123-eslint.sh`,
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      fix: 'Check work unit ID spelling or create work unit first: fspec create-work-unit AUTH "Title"',
    },
  ],
  typicalWorkflow:
    'Add virtual hooks → fspec list-virtual-hooks → Review configuration → Update hooks if needed → Execute workflow transition',
  relatedCommands: [
    'add-virtual-hook',
    'remove-virtual-hook',
    'clear-virtual-hooks',
    'show-work-unit',
  ],
  notes: [
    'Hooks are displayed grouped by event for clarity',
    '[blocking] badge indicates hook failure prevents workflow transition',
    '[git-context] badge indicates hook uses git staged/unstaged files',
    'Hooks execute in the order they were added (top to bottom)',
    'Virtual hooks also appear in "fspec show-work-unit" output',
    'Empty output means no virtual hooks configured for the work unit',
  ],
  prerequisites: ['Work unit must exist'],
  commonPatterns: [
    {
      pattern: 'Review Quality Gates Before Implementing',
      example:
        'fspec list-virtual-hooks AUTH-001\n# Review hooks, then:\nfspec update-work-unit-status AUTH-001 implementing',
      description:
        'Check what quality checks will run before starting implementation.',
    },
    {
      pattern: 'Debug Hook Execution',
      example:
        'fspec list-virtual-hooks AUTH-001\n# If hook fails, review command and fix',
      description:
        'When a hook fails, list hooks to see the exact command being executed.',
    },
    {
      pattern: 'Compare Hooks Across Work Units',
      example:
        'fspec list-virtual-hooks AUTH-001\nfspec list-virtual-hooks AUTH-002',
      description:
        'Compare quality checks between related work units to ensure consistency.',
    },
  ],
};

export default config;
