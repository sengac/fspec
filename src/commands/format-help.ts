import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'format',
  description: 'Format feature files with custom AST-based Gherkin formatter',
  usage: 'fspec format [file]',
  whenToUse:
    'Use to ensure consistent formatting across all feature files. Automatically formats indentation, spacing, and structure according to Gherkin best practices.',
  arguments: [
    {
      name: 'file',
      description:
        'Specific feature file to format. If omitted, formats all .feature files in spec/features/',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec format',
      description: 'Format all feature files',
      output:
        '✓ Formatted spec/features/login.feature\n✓ Formatted spec/features/signup.feature\n\nFormatted 2 files',
    },
    {
      command: 'fspec format spec/features/login.feature',
      description: 'Format specific file',
      output: '✓ Formatted spec/features/login.feature',
    },
  ],
  relatedCommands: ['validate', 'create-feature', 'check'],
  notes: [
    'Uses custom AST-based formatter (NOT prettier-plugin-gherkin)',
    'Automatically fixes indentation (2 spaces for scenarios, 4 for steps)',
    'Preserves doc strings (""") and data tables (|)',
    'Maintains tag formatting',
    'Safe to run multiple times (idempotent)',
    'Run before committing feature files',
  ],
};

export default config;
