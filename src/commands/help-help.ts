import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'help',
  description: 'Display help information for fspec commands',
  usage: 'fspec help [command]',
  arguments: [
    {
      name: 'command',
      description: 'Command to get help for',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec help',
      description: 'Show general help',
      output: 'Usage: fspec [options] [command]\n\nOptions:\n  -h, --help...',
    },
    {
      command: 'fspec help validate',
      description: 'Show help for validate command',
      output: 'Command: validate\nDescription: Validate Gherkin syntax...',
    },
    {
      command: 'fspec validate --help',
      description: 'Alternative help syntax',
      output: 'Command: validate\nDescription: Validate Gherkin syntax...',
    },
  ],
  relatedCommands: ['init'],
  notes: [
    'All commands support --help flag',
    'Use --help after any command for detailed information',
  ],
};

export default config;
