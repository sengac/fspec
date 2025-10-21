import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-virtual-hook',
  description: 'Remove a specific virtual hook from a work unit',
  usage: 'fspec remove-virtual-hook <workUnitId> <hookName>',
  whenToUse:
    'Use this command to remove a specific virtual hook from a work unit. Removes the hook configuration from spec/work-units.json and cleans up any generated script files in spec/hooks/.virtual/. Useful for removing broken hooks, hooks that are no longer needed, or correcting mistakes.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to remove hook from (e.g., AUTH-001)',
      required: true,
    },
    {
      name: 'hookName',
      description:
        'Name of the hook to remove (e.g., eslint, prettier, typecheck)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec remove-virtual-hook AUTH-001 eslint',
      description: 'Remove the eslint hook from AUTH-001',
      output:
        "✓ Removed virtual hook 'eslint' from AUTH-001\n  Remaining virtual hooks: 1",
    },
    {
      command: 'fspec remove-virtual-hook BUG-042 typecheck',
      description: 'Remove type check hook from BUG-042',
      output:
        "✓ Removed virtual hook 'typecheck' from BUG-042\n  Remaining virtual hooks: 0",
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      fix: 'Check work unit ID spelling. List all work units: fspec list-work-units',
    },
    {
      error: 'No virtual hooks configured for AUTH-001',
      fix: 'Work unit has no hooks to remove. List hooks: fspec list-virtual-hooks AUTH-001',
    },
    {
      error: "Virtual hook 'eslin' not found in AUTH-001",
      fix: 'Check hook name spelling. List hooks: fspec list-virtual-hooks AUTH-001 (correct name might be "eslint")',
    },
  ],
  typicalWorkflow:
    'fspec list-virtual-hooks → Identify hook to remove → fspec remove-virtual-hook → Verify removal',
  relatedCommands: [
    'add-virtual-hook',
    'list-virtual-hooks',
    'clear-virtual-hooks',
  ],
  notes: [
    'Hook name is auto-generated from command (e.g., "npm run lint" → "npm")',
    'To find hook names, use: fspec list-virtual-hooks <workUnitId>',
    'Automatically cleans up script files in spec/hooks/.virtual/',
    'If multiple hooks have same name at different events, only first is removed',
    'To remove ALL hooks, use: fspec clear-virtual-hooks',
    'Operation is permanent - hook configuration cannot be recovered',
  ],
  prerequisites: [
    'Work unit must exist',
    'Work unit must have virtual hooks configured',
    'Hook name must match exactly (case-sensitive)',
  ],
  commonPatterns: [
    {
      pattern: 'Remove Broken Hook',
      example:
        'fspec remove-virtual-hook AUTH-001 eslin\nfspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --blocking',
      description:
        'Remove hook with typo in command name, then add corrected version.',
    },
    {
      pattern: 'Replace Hook Configuration',
      example:
        'fspec remove-virtual-hook AUTH-001 eslint\nfspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --git-context --blocking',
      description:
        'Remove existing hook and add new one with different configuration (e.g., add git context).',
    },
    {
      pattern: 'Remove After Workflow Transition',
      example:
        'fspec update-work-unit-status AUTH-001 done\n# Hook failed, no longer needed:\nfspec remove-virtual-hook AUTH-001 experimental-check',
      description:
        'Remove experimental or one-time hooks after work unit completion.',
    },
  ],
};

export default config;
