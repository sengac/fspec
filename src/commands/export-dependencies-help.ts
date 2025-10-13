import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'export-dependencies',
  description: 'Export dependency graph visualization in various formats',
  usage: 'fspec export-dependencies <format> <output>',
  arguments: [
    {
      name: 'format',
      description: 'Output format: dot, mermaid, or json',
      required: true,
    },
    {
      name: 'output',
      description: 'Output file path',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec export-dependencies mermaid dependencies.mmd',
      description: 'Export as Mermaid diagram',
      output: '✓ Exported dependencies to dependencies.mmd',
    },
  ],
  relatedCommands: ['add-dependency', 'query-dependency-stats'],
};

export default config;
