import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'init',
  description:
    'Initialize fspec in a project with AI agent integration (supports 18 agents)',
  usage: 'fspec init [options]',
  whenToUse:
    'Use once when setting up fspec in a new project. Shows interactive menu to select AI agent(s), or use --agent flag to specify directly.',
  options: [
    {
      flag: '--agent <agent>',
      description:
        'Specify agent directly: claude, cursor, windsurf, cline, aider, etc. (18 agents supported)',
    },
    {
      flag: '--yes',
      description: 'Skip confirmation prompts',
    },
  ],
  examples: [
    {
      command: 'fspec init',
      description: 'Interactive agent selection (menu)',
      output: '? Select your AI agent(s):\n  ❯◯ Claude Code\n   ◯ Cursor\n   ◯ Windsurf\n   ...',
    },
    {
      command: 'fspec init --agent=claude',
      description: 'Initialize with Claude Code directly',
      output: '✓ Installed fspec for Claude Code\n\nNext steps:\nRun /fspec in Claude Code to activate',
    },
    {
      command: 'fspec init --agent=cursor',
      description: 'Initialize with Cursor directly',
      output: '✓ Installed fspec for Cursor\n\nNext steps:\nOpen .cursor/commands/ in Cursor to activate',
    },
  ],
  prerequisites: ['Project should have package.json or be a git repository'],
  relatedCommands: ['create-feature', 'create-work-unit'],
  notes: [
    'Supports 18 AI agents: Claude Code, Cursor, Windsurf, Cline, Aider, and more',
    'Creates agent-specific slash command (e.g., .claude/commands/fspec.md)',
    'Creates agent-specific workflow docs (e.g., spec/CLAUDE.md)',
    'Creates spec/fspec-config.json for runtime agent detection',
    'Shows agent-specific activation instructions (customized per agent)',
    'Can be run multiple times with different --agent to support multiple agents',
    'Use "fspec reverse" command for reverse ACDD workflow on existing codebases',
    'Creates spec/ directory structure',
    'Initializes tags.json and foundation.json',
  ],
};

export default config;
