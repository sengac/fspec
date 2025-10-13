import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-foundation',
  description: 'Update foundation.json metadata fields',
  usage: 'fspec update-foundation <field> <value>',
  arguments: [
    {
      name: 'field',
      description: 'Field to update (e.g., projectName, version, description)',
      required: true,
    },
    {
      name: 'value',
      description: 'New value',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec update-foundation version "2.0.0"',
      description: 'Update project version',
      output: '✓ Updated foundation.json: version = "2.0.0"',
    },
    {
      command:
        'fspec update-foundation description "A CLI tool for managing Gherkin specifications"',
      description: 'Update project description',
      output: '✓ Updated foundation.json: description = "A CLI tool..."',
    },
  ],
  relatedCommands: [
    'show-foundation',
    'validate-foundation-schema',
    'generate-foundation-md',
  ],
  notes: [
    'Changes must comply with foundation.json schema',
    'Run validate-foundation-schema after updates',
  ],
};

export default config;
