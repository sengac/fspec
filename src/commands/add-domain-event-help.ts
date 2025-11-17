import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-domain-event',
  description:
    'Add domain event to Event Storm section of work unit for Big Picture Event Storming',
  usage: 'fspec add-domain-event <workUnitId> <text> [options]',
  whenToUse:
    'Use during Big Picture Event Storming discovery phase when capturing significant domain events that represent state changes in the system.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Event text should describe WHAT HAPPENED (past tense)',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'text',
      description:
        'Event text/name (PascalCase recommended, e.g., "UserRegistered", "OrderPlaced")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--timestamp <ms>',
      description:
        'Timeline timestamp in milliseconds (for temporal ordering of events)',
    },
    {
      flag: '--bounded-context <context>',
      description: 'Bounded context for domain association (DDD concept)',
    },
  ],
  examples: [
    {
      command: 'fspec add-domain-event AUTH-001 "UserRegistered"',
      description: 'Add domain event to work unit',
      output: '✓ Added domain event "UserRegistered" to AUTH-001 (ID: 0)',
    },
    {
      command:
        'fspec add-domain-event AUTH-001 "UserAuthenticated" --bounded-context "Identity"',
      description: 'Add domain event with bounded context',
      output: '✓ Added domain event "UserAuthenticated" to AUTH-001 (ID: 1)',
    },
    {
      command:
        'fspec add-domain-event CHECKOUT-001 "OrderPlaced" --timestamp 1000',
      description: 'Add domain event with timeline timestamp',
      output: '✓ Added domain event "OrderPlaced" to CHECKOUT-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-command',
    'add-policy',
    'add-hotspot',
    'show-event-storm',
    'show-foundation-event-storm',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
    {
      error: 'spec/work-units.json not found',
      fix: 'Initialize fspec first: fspec init',
    },
  ],
  commonPatterns: [
    'Use PascalCase for event names (UserRegistered, OrderPlaced)',
    'Events represent WHAT HAPPENED (past tense, not imperative)',
    'Group related events using --bounded-context flag',
    'Use --timestamp for temporal ordering on Big Picture Event Storm timeline',
    'Generate Example Mapping from events: fspec generate-example-mapping-from-event-storm',
  ],
  notes: [
    'Domain events are immutable facts that occurred in the system',
    'Events assigned stable IDs starting from 0',
    'Deleted events marked with deleted: true (soft delete)',
    'Use show-event-storm to view all Event Storm artifacts',
    'Bounded contexts help organize events by subdomain (DDD pattern)',
  ],
};

export default config;
