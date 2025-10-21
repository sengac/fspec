import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'copy-virtual-hooks',
  description: 'Copy virtual hooks from one work unit to another',
  usage:
    'fspec copy-virtual-hooks --from <sourceId> --to <targetId> [--hook-name <name>]',
  whenToUse:
    'Use this command to copy virtual hooks between work units. Useful when multiple related work units need the same quality checks (e.g., all authentication stories need eslint + prettier). Copies hook configuration including event, command, blocking status, and git context settings.',
  arguments: [],
  options: [
    {
      flag: '--from <workUnitId>',
      description:
        'Source work unit ID to copy hooks from (e.g., AUTH-001). Required.',
    },
    {
      flag: '--to <workUnitId>',
      description:
        'Target work unit ID to copy hooks to (e.g., AUTH-002). Required.',
    },
    {
      flag: '--hook-name <name>',
      description:
        'Copy only a specific hook by name (e.g., eslint). Optional - omit to copy all hooks.',
    },
  ],
  examples: [
    {
      command: 'fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002',
      description: 'Copy all virtual hooks from AUTH-001 to AUTH-002',
      output: '✓ Copied 3 virtual hook(s) from AUTH-001 to AUTH-002',
    },
    {
      command:
        'fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint',
      description: 'Copy only the eslint hook from AUTH-001 to AUTH-002',
      output: '✓ Copied 1 virtual hook(s) from AUTH-001 to AUTH-002',
    },
    {
      command: 'fspec copy-virtual-hooks --from FEAT-100 --to FEAT-101',
      description: 'Copy hooks to a related feature work unit',
      output: '✓ Copied 2 virtual hook(s) from FEAT-100 to FEAT-101',
    },
  ],
  commonErrors: [
    {
      error: '--from option is required',
      fix: 'Specify source work unit: --from AUTH-001',
    },
    {
      error: '--to option is required',
      fix: 'Specify target work unit: --to AUTH-002',
    },
    {
      error: "Source work unit 'AUTH-999' does not exist",
      fix: 'Check source work unit ID. List work units: fspec list-work-units',
    },
    {
      error: "Target work unit 'AUTH-888' does not exist",
      fix: 'Create target work unit first: fspec create-work-unit AUTH "Title"',
    },
    {
      error: 'No virtual hooks configured for source work unit AUTH-001',
      fix: 'Source must have hooks. Add hooks: fspec add-virtual-hook AUTH-001 ...',
    },
    {
      error: "Hook 'eslin' not found in AUTH-001",
      fix: 'Check hook name spelling. List hooks: fspec list-virtual-hooks AUTH-001',
    },
  ],
  typicalWorkflow:
    'Create related work units → Configure hooks on first → Copy to others → Verify with list-virtual-hooks',
  relatedCommands: [
    'add-virtual-hook',
    'list-virtual-hooks',
    'remove-virtual-hook',
  ],
  notes: [
    'Copies hook configuration, not generated script files',
    'Target work unit can have existing hooks - copied hooks are appended',
    'Copied hooks maintain all settings: event, command, blocking, git context',
    'Operation creates deep copy - modifying source hooks does not affect copies',
    'Use --hook-name to copy selectively, omit to copy all hooks',
    'Script files for git context hooks are NOT copied (regenerated on execution)',
  ],
  prerequisites: [
    'Source work unit must exist',
    'Target work unit must exist',
    'Source work unit must have virtual hooks configured',
  ],
  commonPatterns: [
    {
      pattern: 'Copy All Hooks to Related Stories',
      example:
        'fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002\nfspec copy-virtual-hooks --from AUTH-001 --to AUTH-003',
      description:
        'Apply same quality checks to all authentication-related work units.',
    },
    {
      pattern: 'Copy Specific Hook to Multiple Work Units',
      example:
        'fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint\nfspec copy-virtual-hooks --from AUTH-001 --to AUTH-003 --hook-name eslint',
      description:
        'Apply only linting check to multiple work units, skip other hooks.',
    },
    {
      pattern: 'Create Template and Copy to New Work Units',
      example:
        '# Set up template:\nfspec add-virtual-hook TEMPLATE-001 post-implementing "npm run lint" --blocking\nfspec add-virtual-hook TEMPLATE-001 pre-validating "npm run typecheck" --blocking\n\n# Copy to actual work units:\nfspec copy-virtual-hooks --from TEMPLATE-001 --to AUTH-010\nfspec copy-virtual-hooks --from TEMPLATE-001 --to BUG-020',
      description:
        'Create a template work unit with standard hooks, then copy to new work units.',
    },
    {
      pattern: 'Verify After Copy',
      example:
        'fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002\nfspec list-virtual-hooks AUTH-002',
      description:
        'Copy hooks and immediately verify they were copied correctly.',
    },
  ],
};

export default config;
