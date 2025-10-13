import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'init',
  description: 'Initialize fspec in a project, creating /fspec and /rspec Claude Code slash commands',
  usage: 'fspec init [options]',
  whenToUse:
    'Use once when setting up fspec in a new project to install Claude Code integration.',
  options: [
    {
      flag: '--type <type>',
      description: 'Installation type: claude-code (default) or custom',
    },
    {
      flag: '--path <path>',
      description: 'Custom installation path (required if --type=custom)',
    },
    {
      flag: '--yes',
      description: 'Skip confirmation prompts',
    },
  ],
  examples: [
    {
      command: 'fspec init',
      description: 'Initialize with Claude Code',
      output: '✓ Initialized fspec\n  Created .claude/commands/fspec.md\n  Created .claude/commands/rspec.md',
    },
  ],
  prerequisites: [
    'Project should have package.json or be a git repository',
  ],
  relatedCommands: ['create-feature', 'create-work-unit'],
  notes: [
    'Creates .claude/commands/ directory',
    'Installs /fspec (forward ACDD) and /rspec (reverse ACDD) commands',
    'Creates spec/ directory structure',
    'Initializes tags.json and foundation.json',
  ],
};

export default config;
