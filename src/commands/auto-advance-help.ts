import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'auto-advance',
  description: 'Automatically advance work units through workflow when criteria are met',
  usage: 'fspec auto-advance [options]',
  whenToUse:
    'Use to automatically move work units forward when they meet criteria (e.g., all questions answered → move to testing).',
  options: [
    {
      flag: '--dry-run',
      description: 'Show what would be advanced without making changes',
    },
  ],
  examples: [
    {
      command: 'fspec auto-advance',
      description: 'Auto-advance eligible work units',
      output: '✓ AUTH-001: specifying → testing (all questions answered)\n✓ AUTH-002: testing → implementing (tests written)\n\nAdvanced 2 work units',
    },
    {
      command: 'fspec auto-advance --dry-run',
      description: 'Preview auto-advance',
      output: 'Would advance: AUTH-001 (specifying → testing)',
    },
  ],
  relatedCommands: ['update-work-unit-status', 'board'],
  notes: [
    'Checks ACDD workflow criteria for each state',
    'Safe to run multiple times (idempotent)',
  ],
};

export default config;
