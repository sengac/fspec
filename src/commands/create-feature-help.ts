import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-feature',
  description:
    'Create a new feature file with proper Gherkin structure template',
  usage: 'fspec create-feature <name>',
  whenToUse:
    'Use when starting a new feature specification in ACDD workflow. Creates a well-structured template with Background section, placeholder scenarios, and proper formatting.',
  arguments: [
    {
      name: 'name',
      description:
        'Feature name in sentence case (e.g., "User Authentication")',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec create-feature "User Authentication"',
      description: 'Create new feature file',
      output: '✓ Created spec/features/user-authentication.feature',
    },
    {
      command: 'fspec create-feature "Payment Processing"',
      description: 'Create feature with multi-word name',
      output: '✓ Created spec/features/payment-processing.feature',
    },
  ],
  relatedCommands: ['add-scenario', 'add-step', 'validate', 'format'],
  notes: [
    'Filename is automatically kebab-cased from the feature name',
    "Creates spec/features/ directory if it doesn't exist",
    'Template includes Background section placeholder',
    'Template includes one example Scenario placeholder',
    'File is created with proper Gherkin formatting',
    'Automatically creates corresponding .feature.coverage file for coverage tracking',
    'Detects placeholder text ([role], [action], [benefit]) and provides CLI commands to fix',
    'Returns structured information about prefill detection, coverage file creation, and file naming',
  ],
};

export default config;
