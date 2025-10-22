import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-init-files',
  description: 'Remove fspec initialization files for installed agents',
  usage: 'fspec remove-init-files',
  whenToUse:
    'Use when you want to uninstall fspec or switch to a different agent. Auto-detects installed agents and removes their configuration files.',
  prerequisites: [
    'At least one agent must be installed (detected via spec/fspec-config.json or file detection)',
  ],
  examples: [
    {
      command: 'fspec remove-init-files',
      description: 'Remove fspec init files for detected agent',
      output:
        '✓ Successfully removed fspec init files\n  - Removed spec/CLAUDE.md\n  - Removed .claude/commands/fspec.md\n  - Removed spec/fspec-config.json',
    },
    {
      command: 'fspec remove-init-files',
      description: 'When no agent detected',
      output: '✗ No fspec agent installation detected. Nothing to remove.',
    },
  ],
  relatedCommands: ['init'],
  notes: [
    'Auto-detects installed agent using spec/fspec-config.json or file detection',
    'Removes agent-specific files: spec/<AGENT>.md and slash command files',
    'Also removes spec/fspec-config.json',
    'Silently skips files that are already deleted (idempotent behavior)',
    'Does NOT remove spec/features/, spec/work-units.json, or other project files',
    'Use fspec init to reinstall after removal',
  ],
  typicalWorkflow: [
    'detect-agent → remove-files → confirmation',
    'Common pattern: fspec remove-init-files (then fspec init --agent=newagent to switch)',
  ],
};

export default config;
