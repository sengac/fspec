import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-foundation-event-storm',
  description:
    'Display foundation Event Storm artifacts as JSON (no semantic interpretation)',
  usage: 'fspec show-foundation-event-storm [--type <type>]',
  whenToUse:
    'Use to view foundation-level Event Storm artifacts (bounded contexts, pivotal events, aggregates) stored in foundation.json.',
  prerequisites: [
    'foundation.json must exist',
    'Big Picture Event Storm must be initialized in foundation',
  ],
  arguments: [],
  options: [
    {
      flag: '--type <type>',
      description:
        'Filter by Event Storm item type (bounded-context, pivotal-event, aggregate)',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec show-foundation-event-storm',
      description: 'Display all foundation Event Storm artifacts',
      output:
        '{\n  "boundedContexts": [...],\n  "pivotalEvents": [...],\n  "aggregates": [...]\n}',
    },
    {
      command: 'fspec show-foundation-event-storm --type bounded-context',
      description: 'Display only bounded contexts',
      output:
        '{\n  "boundedContexts": [\n    {"id": 0, "name": "Identity", "description": "User authentication and authorization"}\n  ]\n}',
    },
  ],
  relatedCommands: [
    'show-event-storm',
    'show-foundation',
    'discover-foundation',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      fix: 'Initialize foundation first: fspec discover-foundation',
    },
    {
      error: 'No Event Storm data in foundation',
      fix: 'Add Big Picture Event Storm artifacts to foundation.json',
    },
  ],
  commonPatterns: [
    'Use for strategic-level Event Storming (foundation scope)',
    'View bounded contexts and their relationships',
    'Check pivotal events that cross context boundaries',
    'Verify aggregate structures',
    'Foundation Event Storm is for BIG PICTURE, work unit Event Storm is for TACTICAL',
  ],
  notes: [
    'Foundation Event Storm is stored in foundation.json (not work-units.json)',
    'Represents strategic domain model (DDD bounded contexts)',
    'Output is raw JSON format',
    'Use --type to filter specific artifact types',
    'Foundation-level artifacts define system architecture',
  ],
};

export default config;
