import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-policy',
  description:
    'Add policy to Event Storm section for reactive business logic (WHEN event THEN command)',
  usage: 'fspec add-policy <workUnitId> <text> [options]',
  whenToUse:
    'Use during Big Picture Event Storming when capturing reactive business logic that triggers automatically when events occur.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Policy should describe WHEN/THEN relationship',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'text',
      description: 'Policy text (brief description)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--when <event>',
      description: 'Event that triggers the policy (e.g., "UserRegistered")',
      required: false,
    },
    {
      flag: '--then <command>',
      description:
        'Command that executes when policy triggers (e.g., "SendWelcomeEmail")',
      required: false,
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline position in milliseconds',
      required: false,
    },
    {
      flag: '--bounded-context <name>',
      description: 'Bounded context association',
      required: false,
    },
  ],
  examples: [
    {
      command:
        'fspec add-policy AUTH-001 "Send welcome email" --when "UserRegistered" --then "SendWelcomeEmail"',
      description: 'Add policy with event/command relationship',
      output: '✓ Added policy "Send welcome email" to AUTH-001 (ID: 0)',
    },
    {
      command:
        'fspec add-policy ORDER-001 "Notify warehouse" --when "OrderPlaced" --then "NotifyWarehouse"',
      description: 'Add reactive policy for order processing',
      output: '✓ Added policy "Notify warehouse" to ORDER-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-hotspot',
    'show-event-storm',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
  ],
  commonPatterns: [
    'Policies represent REACTIVE business logic (automation)',
    'Use WHEN/THEN pattern: WHEN event THEN command',
    'Policies convert to business rules via generate-example-mapping-from-event-storm',
    'Policies often represent system behaviors (not user actions)',
  ],
  notes: [
    'Policies are automation rules triggered by events',
    'Format: "WHEN event happens THEN execute command"',
    'Policies assigned stable IDs starting from 0',
    'Use show-event-storm to view all policies',
  ],
};

export default config;
