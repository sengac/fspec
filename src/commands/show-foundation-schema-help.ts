import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-foundation-schema',
  description: 'Display JSON schema for foundation.json',
  usage: 'fspec show-foundation-schema',
  examples: [
    {
      command: 'fspec show-foundation-schema',
      description: 'Show foundation.json schema',
      output:
        '{\n  "type": "object",\n  "properties": {\n    "projectName": {...}\n  }\n}',
    },
  ],
  relatedCommands: [
    'validate-foundation-schema',
    'show-foundation',
    'update-foundation',
  ],
  notes: [
    'Schema is used by Ajv for validation',
    'Defines structure and validation rules for foundation.json',
  ],
};

export default config;
