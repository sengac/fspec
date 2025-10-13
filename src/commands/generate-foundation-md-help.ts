import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-foundation-md',
  description: 'Generate spec/FOUNDATION.md from foundation.json',
  usage: 'fspec generate-foundation-md',
  examples: [
    {
      command: 'fspec generate-foundation-md',
      description: 'Generate human-readable foundation documentation',
      output:
        '✓ Generated spec/FOUNDATION.md from foundation.json\n✓ Mermaid diagrams included',
    },
  ],
  relatedCommands: [
    'show-foundation',
    'update-foundation',
    'validate-foundation-schema',
  ],
  notes: [
    'FOUNDATION.md is human-readable documentation',
    'foundation.json is the machine-readable source of truth',
    'Generated file includes project metadata and Mermaid diagrams',
  ],
};

export default config;
