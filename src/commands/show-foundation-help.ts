import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-foundation',
  description: 'Display contents of foundation.json',
  usage: 'fspec show-foundation [section] [options]',
  arguments: [
    {
      name: 'section',
      description: 'Specific section to show (optional)',
      required: false,
    },
  ],
  options: [
    {
      flag: '--list-sections',
      description: 'List section names only (without content)',
    },
    {
      flag: '--line-numbers',
      description: 'Show line numbers in output',
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
      command: 'fspec show-foundation --list-sections',
      description: 'List available sections',
      output:
        'Available sections:\n  - Architecture Diagrams\n  - System Overview\n  - Project Goals',
    },
    {
      command: 'fspec show-foundation "Architecture Diagrams"',
      description: 'Show specific section',
      output: '{\n  "Architecture Diagrams": [\n    {...}\n  ]\n}',
    },
    {
      command: 'fspec show-foundation "System Overview" --line-numbers',
      description: 'Show section with line numbers',
      output: '1: {\n2:   "System Overview": {\n3:     ...\n4:   }\n5: }',
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
    '--list-sections helps discover available section names',
    '--line-numbers useful for debugging or referencing specific lines',
  ],
};

export default config;
