import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'report-bug-to-github',
  description:
    'Report bugs to GitHub with AI-assisted context gathering. Automatically collects system information, git status, work unit context, and recent error logs to create a pre-filled GitHub issue.',
  usage: 'fspec report-bug-to-github [options]',
  whenToUse:
    'Use when you encounter a bug in fspec and want to report it to the maintainers. This command gathers all relevant context automatically, making it easy to create comprehensive bug reports without manually collecting system information.',
  arguments: [],
  options: [
    {
      flag: '--project-root <path>',
      description: 'Project root directory (default: auto-detected)',
    },
    {
      flag: '--bug-description <text>',
      description: 'Brief description of the bug',
    },
    {
      flag: '--expected-behavior <text>',
      description: 'What you expected to happen',
    },
    {
      flag: '--actual-behavior <text>',
      description: 'What actually happened',
    },
    {
      flag: '--interactive',
      description: 'Enable interactive mode with prompts for bug details',
    },
  ],
  examples: [
    {
      command: 'fspec report-bug-to-github',
      description:
        'Generate bug report with automatic context gathering and open in browser',
      output:
        'Gathering system context...\n✓ fspec version: 0.6.0\n✓ Node version: v22.20.0\n✓ Platform: linux\n✓ Git branch: feature-branch\n✓ Work unit: CLI-014 - Report bug to GitHub\n\nOpening GitHub issue in browser...\n✓ Browser opened with pre-filled issue',
    },
    {
      command:
        'fspec report-bug-to-github --bug-description "Validation command crashes"',
      description: 'Report bug with description',
      output:
        'Gathering system context...\n✓ Generated bug report\n✓ Opening browser with pre-filled issue',
    },
    {
      command: 'fspec report-bug-to-github --interactive',
      description: 'Interactive bug reporting with prompts',
      output:
        'What command were you running? fspec validate\nWhat did you expect to happen? Successful validation\nWhat actually happened? Command crashed\n\nGenerating bug report...\n✓ Opening browser with pre-filled issue',
    },
  ],
  prerequisites: [
    'fspec must be installed (to gather version information)',
    'Git repository (optional, for git context)',
    'Work units file (optional, for work unit context)',
    'Browser installed (to open GitHub issue)',
  ],
  typicalWorkflow: [
    'Encounter a bug while using fspec',
    'Run: fspec report-bug-to-github',
    'System automatically gathers context (version, platform, git status, work unit)',
    'Browser opens with pre-filled GitHub issue',
    'Review and submit the issue',
  ],
  commonErrors: [
    {
      error: 'Browser failed to open',
      solution:
        'Check that you have a default browser configured. The issue URL is still generated and can be copied manually.',
    },
    {
      error: 'No work unit context found',
      solution:
        "This is normal if you don't have work units. The bug report will still be generated with system context.",
    },
  ],
  relatedCommands: [
    'fspec validate - Validate feature files',
    'fspec check - Run comprehensive validation',
    'fspec show-work-unit - View work unit details',
  ],
  notes: [
    'Automatically includes: fspec version, Node version, OS platform',
    'Git context: current branch, uncommitted changes status',
    'Work unit context: ID, title, status, related feature file',
    'Error logs: recent errors from .fspec/error-logs/',
    'All data is URL-encoded for GitHub issue form',
    'The command does NOT automatically submit the issue - you review first',
    'Special characters and code blocks are properly escaped',
    'Supports both interactive and non-interactive modes',
  ],
};

export default config;
