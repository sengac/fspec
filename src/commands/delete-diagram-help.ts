import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-diagram',
  description: 'Delete a diagram from foundation.json',
  usage: 'fspec delete-diagram <section> <title>',
  arguments: [
    {
      name: 'section',
      description: 'Section name in foundation.json',
      required: true,
    },
    {
      name: 'title',
      description: 'Diagram title to delete',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec delete-diagram "Architecture Diagrams" "Command Flow"',
      description: 'Delete specific diagram',
      output: '✓ Deleted diagram from foundation.json',
    },
  ],
  relatedCommands: ['add-diagram', 'show-foundation', 'update-foundation'],
};

export default config;
