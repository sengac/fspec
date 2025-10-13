import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-foundation',
  description: 'Display contents of foundation.json',
  usage: 'fspec show-foundation [section]',
  arguments: [
    {
      name: 'section',
      description: 'Specific section to show (optional)',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec show-foundation',
      description: 'Show all foundation data',
      output:
        '{\n  "projectName": "fspec",\n  "version": "1.0.0",\n  "sections": {...}\n}',
    },
    {
      command: 'fspec show-foundation "Architecture Diagrams"',
      description: 'Show specific section',
      output: '{\n  "Architecture Diagrams": [\n    {...}\n  ]\n}',
    },
  ],
  relatedCommands: [
    'update-foundation',
    'add-diagram',
    'validate-foundation-schema',
  ],
  notes: [
    'foundation.json is the machine-readable source of truth',
    'Use generate-foundation-md for human-readable output',
  ],
};

export default config;
