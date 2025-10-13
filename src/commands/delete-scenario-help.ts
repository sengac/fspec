import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-scenario',
  description: 'Delete a scenario from a feature file',
  usage: 'fspec delete-scenario <file> <scenario> [options]',
  arguments: [
    {
      name: 'file',
      description: 'Feature file path',
      required: true,
    },
    {
      name: 'scenario',
      description: 'Scenario name to delete',
      required: true,
    },
  ],
  options: [
    {
      flag: '--force',
      description: 'Skip confirmation',
    },
  ],
  examples: [
    {
      command: 'fspec delete-scenario spec/features/login.feature "Deprecated scenario"',
      description: 'Delete scenario',
      output: '✓ Deleted scenario "Deprecated scenario"',
    },
  ],
  relatedCommands: ['add-scenario', 'delete-scenarios'],
  notes: [
    'Scenario name must match exactly (case-sensitive)',
  ],
};

export default config;
