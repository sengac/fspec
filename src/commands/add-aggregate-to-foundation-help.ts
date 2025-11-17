import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-aggregate-to-foundation',
  description:
    'Add an aggregate to a foundation bounded context in Big Picture Event Storm',
  usage:
    'fspec add-aggregate-to-foundation <context-name> <aggregate-name> [--description <text>]',
  whenToUse:
    'Use during Big Picture Event Storming to add aggregates (business entities with state and behavior) to specific bounded contexts in the foundation domain model.',
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
      name: '<aggregate-name>',
      description:
        'Name of the aggregate to add (e.g., "WorkUnit", "User", "Order")',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <text>',
      description: "Optional description explaining the aggregate's purpose",
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec add-aggregate-to-foundation "Work Management" "WorkUnit"',
      description: 'Add WorkUnit aggregate to Work Management bounded context',
      output:
        '✓ Added aggregate "WorkUnit" to "Work Management" bounded context',
    },
    {
      command:
        'fspec add-aggregate-to-foundation "Work Management" "WorkUnit" --description "Tracks a discrete piece of work"',
      description: 'Add aggregate with description',
      output:
        '✓ Added aggregate "WorkUnit" to "Work Management" bounded context',
    },
    {
      command: 'fspec add-aggregate-to-foundation "Identity" "User"',
      description: 'Add User aggregate to Identity bounded context',
      output: '✓ Added aggregate "User" to "Identity" bounded context',
    },
  ],
  relatedCommands: [
    'add-foundation-bounded-context',
    'add-domain-event-to-foundation',
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
    'Add aggregates during Big Picture Event Storm sessions',
    'Group related aggregates within same bounded context',
    'Aggregates typically map to core business entities (User, Order, WorkUnit, etc.)',
    'Each aggregate should have clear state and behavior boundaries',
    'Use descriptions to document aggregate responsibilities',
  ],
  notes: [
    'Aggregates are stored in foundation.json eventStorm.items array',
    'Each aggregate is linked to a bounded context via boundedContextId',
    'Aggregates have color="yellow" by default (Event Storming convention)',
    'FOUNDATION.md is automatically regenerated after adding aggregate',
    'Foundation aggregates define STRATEGIC model, work unit aggregates are TACTICAL',
  ],
};

export default config;
