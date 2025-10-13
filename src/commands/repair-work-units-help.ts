import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'repair-work-units',
  description: 'Repair work unit data integrity issues',
  usage: 'fspec repair-work-units [options]',
  whenToUse:
    'Use when validate-work-units reports issues that need fixing, such as broken references or invalid data.',
  options: [
    {
      flag: '--dry-run',
      description: 'Show what would be repaired without making changes',
    },
  ],
  examples: [
    {
      command: 'fspec repair-work-units',
      description: 'Repair all issues',
      output: '✓ Repaired 3 work units\n  - Fixed broken dependency: AUTH-001 → AUTH-999 (deleted)\n  - Reset invalid status: UI-002',
    },
  ],
  relatedCommands: ['validate-work-units'],
  notes: [
    'Use --dry-run first to preview changes',
    'Creates backup before modifying',
  ],
};

export default config;
