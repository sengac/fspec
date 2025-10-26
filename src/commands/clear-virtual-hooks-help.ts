import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'clear-virtual-hooks',
  description: 'Clear all virtual hooks from a work unit',
  usage: 'fspec clear-virtual-hooks <workUnitId>',
  whenToUse:
    'Use this command when a work unit reaches "done" status and you want to remove all virtual hooks. Clears the entire virtualHooks array and cleans up all generated script files in spec/hooks/.virtual/. Useful for cleanup after story completion or when resetting work unit configuration.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to clear hooks from (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec clear-virtual-hooks AUTH-001',
      description: 'Clear all virtual hooks from AUTH-001',
      output: '✓ Cleared 3 virtual hook(s) from AUTH-001',
    },
    {
      command: 'fspec clear-virtual-hooks BUG-042',
      description: 'Clear hooks from work unit with no hooks',
      output: '✓ Cleared 0 virtual hook(s) from BUG-042',
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      fix: 'Check work unit ID spelling. List all work units: fspec list-work-units',
    },
  ],
  typicalWorkflow:
    'Work unit reaches done → AI asks "Keep or remove hooks?" → User chooses remove → fspec clear-virtual-hooks',
  relatedCommands: [
    'add-virtual-hook',
    'list-virtual-hooks',
    'remove-virtual-hook',
    'update-work-unit-status',
  ],
  notes: [
    'Clears ALL virtual hooks - operation cannot be undone',
    'Automatically cleans up all script files in spec/hooks/.virtual/',
    'Sets virtualHooks array to empty [] (not undefined)',
    'Operation is safe even if work unit has no hooks (clears 0 hooks)',
    'AI prompts for this decision when work unit transitions to "done" status',
    'Consider using remove-virtual-hook to selectively remove specific hooks',
  ],
  prerequisites: ['Work unit must exist'],
  commonPatterns: [
    {
      pattern: 'Cleanup After Story Completion',
      example:
        'fspec update-work-unit-status AUTH-001 done\n# AI asks: "Keep or remove hooks?"\n# User chooses: "remove"\nfspec clear-virtual-hooks AUTH-001',
      description:
        'Standard workflow when completing a work unit and removing temporary quality checks.',
    },
    {
      pattern: 'Reset Work Unit Configuration',
      example:
        'fspec clear-virtual-hooks AUTH-001\n# Start fresh with new hooks:\nfspec add-virtual-hook AUTH-001 post-implementing "<quality-check-commands>" --blocking',
      description:
        'Clear all existing hooks to start with a clean slate and add new configuration.',
    },
    {
      pattern: 'Bulk Cleanup After Release',
      example:
        'fspec list-work-units --status=done\n# For each done work unit:\nfspec clear-virtual-hooks AUTH-001\nfspec clear-virtual-hooks AUTH-002',
      description:
        'Clean up virtual hooks from multiple completed work units after a release.',
    },
  ],
};

export default config;
