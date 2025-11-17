import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-command-to-foundation',
  description:
    'Add a command to a foundation bounded context in Big Picture Event Storm',
  usage:
    'fspec add-command-to-foundation <context-name> <command-name> [--description <text>]',
  whenToUse:
    'Use during Big Picture Event Storming to add commands (user intentions or actions) to specific bounded contexts in the foundation domain model.',
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
      name: '<command-name>',
      description:
        'Name of the command to add (e.g., "CreateWorkUnit", "UpdateStatus")',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <text>',
      description: "Optional description explaining the command's intent",
      required: false,
    },
  ],
  examples: [
    {
      command:
        'fspec add-command-to-foundation "Work Management" "CreateWorkUnit"',
      description:
        'Add CreateWorkUnit command to Work Management bounded context',
      output:
        '✓ Added command "CreateWorkUnit" to "Work Management" bounded context',
    },
    {
      command:
        'fspec add-command-to-foundation "Work Management" "UpdateStatus" --description "Changes work unit status"',
      description: 'Add command with description',
      output:
        '✓ Added command "UpdateStatus" to "Work Management" bounded context',
    },
    {
      command: 'fspec add-command-to-foundation "Identity" "AuthenticateUser"',
      description: 'Add AuthenticateUser command to Identity bounded context',
      output:
        '✓ Added command "AuthenticateUser" to "Identity" bounded context',
    },
  ],
  relatedCommands: [
    'add-foundation-bounded-context',
    'add-aggregate-to-foundation',
    'add-domain-event-to-foundation',
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
    'Use imperative verb + noun for command names (CreateWorkUnit, UpdateStatus)',
    'Commands represent user intentions or system actions',
    'Add commands during Big Picture Event Storm sessions',
    'Commands typically trigger domain events (CreateWorkUnit → WorkUnitCreated)',
    'Use descriptions to document command purpose and behavior',
  ],
  notes: [
    'Commands are stored in foundation.json eventStorm.items array',
    'Each command is linked to a bounded context via boundedContextId',
    'Commands have color="blue" by default (Event Storming convention)',
    'FOUNDATION.md is automatically regenerated after adding command',
    'Foundation commands define STRATEGIC model, work unit commands are TACTICAL',
    'Commands should use imperative naming convention (verb + noun)',
  ],
};

export default config;
