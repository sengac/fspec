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
      output:
        '? Select your AI agent(s):\n  ❯◯ Claude Code\n   ◯ Cursor\n   ◯ Windsurf\n   ...',
    },
    {
      command: 'fspec init --agent=claude',
      description: 'Initialize with Claude Code directly',
      output:
        '✓ Installed fspec for Claude Code\n\nNext steps:\nRun /fspec in Claude Code to activate',
    },
    {
      command: 'fspec init --agent=cursor',
      description: 'Initialize with Cursor directly',
      output:
        '✓ Installed fspec for Cursor\n\nNext steps:\nOpen .cursor/commands/ in Cursor to activate',
    },
  ],
  prerequisites: ['Project should have package.json or be a git repository'],
  relatedCommands: [
    'create-feature',
    'create-story',
    'create-bug',
    'create-task',
  ],
  notes: [
    'Supports 18 AI agents: Claude Code, Cursor, Windsurf, Cline, Aider, and more',
    'Creates agent-specific slash command (e.g., .claude/commands/fspec.md)',
    'Creates agent-specific workflow docs (e.g., spec/CLAUDE.md)',
    'Creates spec/fspec-config.json for runtime agent detection',
    'Embeds current fspec version in slash command file as "fspec --sync-version <version>" command',
    'Version sync command runs as first command to auto-update files on version mismatch',
    'On version mismatch: updates both slash command file AND spec doc file, shows restart message, exits 1',
    'On version match WITHOUT tool config: emits system-reminders about missing config, exits 1 (blocks workflow)',
    'On version match WITH tool config: emits tool configuration system-reminders, exits 0 (continues workflow)',
    'Shows agent-specific activation instructions (customized per agent)',
    'Auto-detects existing agent installations and prompts to switch when different agent requested',
    'Updating config file automatically when switching agents (preserves project files)',
    'Interactive mode pre-selects detected agent for easy confirmation or switching',
    'Same agent reinstall is idempotent (refreshes files without prompts)',
    'Can be run multiple times with different --agent to support multiple agents',
    'Use "fspec remove-init-files" to uninstall or clean up before switching',
    'Use "fspec reverse" command for reverse ACDD workflow on existing codebases',
    'Creates spec/ directory structure',
    'Initializes tags.json and foundation.json',
  ],
};

export default config;
