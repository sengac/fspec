import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'board',
  description:
    'Display Kanban board of all work units grouped by status (backlog, specifying, testing, implementing, validating, done, blocked). Shows work unit IDs, titles, and story point estimates. Calculates total points in progress vs completed.',
  usage: 'fspec board [options]',
  whenToUse:
    'Use to visualize current project state, see what work is in progress, check story point velocity, review work unit distribution across workflow stages, or get an overview of all work.',
  arguments: [],
  options: [
    {
      flag: '--format <format>',
      description:
        'Output format: text (interactive TUI) or json (machine-readable). Default: text',
    },
    {
      flag: '--limit <limit>',
      description: 'Maximum items to show per column in text mode. Default: 25',
    },
  ],
  examples: [
    {
      command: 'fspec board',
      description: 'Display interactive Kanban board (default text format)',
      output:
        'Displays interactive TUI with columns for each status, showing work units with IDs, titles, and estimates. Summary line shows: "15 points in progress, 23 points completed"',
    },
    {
      command: 'fspec board --format=json',
      description: 'Output board data as JSON',
      output:
        '{\n  "columns": { ... },\n  "board": { ... },\n  "summary": "15 points in progress, 23 points completed"\n}',
    },
    {
      command: 'fspec board --limit=10',
      description: 'Show maximum 10 items per column',
      output: 'Displays board with up to 10 work units per status column',
    },
  ],
  prerequisites: [
    'foundation.json must exist (run fspec discover-foundation if needed)',
    'work-units.json will be auto-created if missing',
  ],
  typicalWorkflow: [
    'View current state: fspec board',
    'Check progress: Review points in progress vs completed',
    'Identify bottlenecks: Look for columns with many items',
    'Plan next work: Review backlog column',
    'Track velocity: Monitor completed points over time',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      solution: 'Run: fspec discover-foundation to create foundation.json',
    },
    {
      error: 'Failed to display board',
      solution:
        'Check that work-units.json is valid JSON. Run fspec validate-work-units to check for issues.',
    },
  ],
  relatedCommands: [
    'fspec list-work-units - List work units with filtering',
    'fspec show-work-unit - View detailed work unit information',
    'fspec update-work-unit-status - Move work units through workflow',
    'fspec query-metrics - Get detailed metrics and statistics',
  ],
  notes: [
    'Text format displays interactive TUI with scrolling and mouse support',
    'JSON format useful for automation and integration',
    'Story points are summed per column (in progress vs done)',
    'Blocked work units appear in separate "blocked" column',
    'Empty columns are still shown with zero items',
    'Work units without estimates do not contribute to point totals',
  ],
};

export default config;
