import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-foundation-schema',
  description: 'Validate foundation.json against its JSON schema using Ajv',
  usage: 'fspec validate-foundation-schema',
  examples: [
    {
      command: 'fspec validate-foundation-schema',
      description: 'Validate foundation.json structure',
      output:
        '✓ foundation.json is valid\n✓ All required fields present\n✓ All diagrams have valid structure',
    },
  ],
  relatedCommands: [
    'show-foundation-schema',
    'show-foundation',
    'update-foundation',
  ],
  notes: [
    'Uses Ajv for JSON Schema validation',
    'Checks required fields, types, and structure',
    'Run after any manual edits to foundation.json',
  ],
};

export default config;
