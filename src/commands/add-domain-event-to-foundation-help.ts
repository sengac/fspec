import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-domain-event-to-foundation',
  description:
    'Add a domain event to a foundation bounded context in Big Picture Event Storm',
  usage:
    'fspec add-domain-event-to-foundation <context-name> <event-name> [--description <text>]',
  whenToUse:
    'Use during Big Picture Event Storming to add domain events (significant business occurrences) to specific bounded contexts in the foundation domain model.',
  prerequisites: [
    'foundation.json must exist',
    'Big Picture Event Storm must be initialized',
    'Target bounded context must already exist',
  ],
  arguments: [
    {
      name: '<context-name>',
      description: 'Name of the bounded context (must already exist)',
      required: true,
    },
    {
      name: '<event-name>',
      description:
        'Name of the domain event to add (e.g., "WorkUnitCreated", "UserLoggedIn")',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <text>',
      description: "Optional description explaining the event's significance",
      required: false,
    },
  ],
  examples: [
    {
      command:
        'fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"',
      description:
        'Add WorkUnitCreated event to Work Management bounded context',
      output:
        '✓ Added domain event "WorkUnitCreated" to "Work Management" bounded context',
    },
    {
      command:
        'fspec add-domain-event-to-foundation "Work Management" "WorkUnitCompleted" --description "Signals work unit reached done status"',
      description: 'Add event with description',
      output:
        '✓ Added domain event "WorkUnitCompleted" to "Work Management" bounded context',
    },
    {
      command: 'fspec add-domain-event-to-foundation "Identity" "UserLoggedIn"',
      description: 'Add UserLoggedIn event to Identity bounded context',
      output:
        '✓ Added domain event "UserLoggedIn" to "Identity" bounded context',
    },
  ],
  relatedCommands: [
    'add-foundation-bounded-context',
    'add-aggregate-to-foundation',
    'add-command-to-foundation',
    'show-foundation-event-storm',
    'generate-foundation-md',
  ],
  commonErrors: [
    {
      error: "Bounded context 'Foo' not found",
      fix: 'Create the bounded context first: fspec add-foundation-bounded-context "Foo"',
    },
    {
      error: 'foundation.json not found',
      fix: 'Initialize foundation first: fspec discover-foundation',
    },
  ],
  commonPatterns: [
    'Use past tense for event names (WorkUnitCreated, not CreateWorkUnit)',
    'Events represent facts that have already occurred',
    'Add events during Big Picture Event Storm sessions',
    'Events often trigger reactions in other bounded contexts',
    'Use descriptions to document business significance',
  ],
  notes: [
    'Domain events are stored in foundation.json eventStorm.items array',
    'Each event is linked to a bounded context via boundedContextId',
    'Events have color="orange" by default (Event Storming convention)',
    'FOUNDATION.md is automatically regenerated after adding event',
    'Foundation events define STRATEGIC model, work unit events are TACTICAL',
    'Events should use past tense naming convention',
  ],
};

export default config;
