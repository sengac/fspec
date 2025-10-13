import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-diagram',
  description: 'Add a Mermaid diagram to foundation.json with automatic syntax validation',
  usage: 'fspec add-diagram <section> <title> <mermaid>',
  arguments: [
    {
      name: 'section',
      description: 'Section name in foundation.json',
      required: true,
    },
    {
      name: 'title',
      description: 'Diagram title',
      required: true,
    },
    {
      name: 'mermaid',
      description: 'Mermaid diagram syntax',
      required: true,
    },
  ],
  examples: [
    {
      command:
        'fspec add-diagram "Architecture Diagrams" "Command Flow" "graph TB\\n  CLI-->Parser\\n  Parser-->Validator"',
      description: 'Add Mermaid diagram with validation',
      output: '✓ Added diagram to foundation.json\n✓ Mermaid syntax validated',
    },
  ],
  relatedCommands: ['delete-diagram', 'show-foundation', 'update-foundation'],
  notes: [
    'Mermaid syntax is validated using mermaid.parse() before adding',
    'Invalid syntax will be rejected with detailed error messages',
    'Use \\n for line breaks in diagram syntax',
  ],
};

export default config;
