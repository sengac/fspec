import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'board',
  description: 'Display Kanban board showing work units across workflow states',
  usage: 'fspec board [options]',
  whenToUse:
    'Use to visualize work distribution across ACDD workflow states and identify bottlenecks.',
  options: [
    {
      flag: '--epic <epic>',
      description: 'Filter by epic',
    },
    {
      flag: '--prefix <prefix>',
      description: 'Filter by work unit prefix',
    },
  ],
  examples: [
    {
      command: 'fspec board',
      description: 'Show full board',
      output: '┌────────┬────────┬────────┬────────┬────────┬────────┐\n│BACKLOG │SPECIFY │TESTING │IMPL    │VALID   │DONE    │\n├────────┼────────┼────────┼────────┼────────┼────────┤\n│AUTH-003│AUTH-001│        │AUTH-002│        │UI-001  │',
    },
  ],
  relatedCommands: ['list-work-units', 'update-work-unit-status'],
  notes: [
    'Columns: backlog, specifying, testing, implementing, validating, done',
    'Shows first 3 work units per column by default',
  ],
};

export default config;
