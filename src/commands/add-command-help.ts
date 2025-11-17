import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-command',
  description:
    'Add command to Event Storm section of work unit for Big Picture Event Storming',
  usage: 'fspec add-command <workUnitId> <text> [options]',
  whenToUse:
    'Use during Big Picture Event Storming discovery phase when capturing user intentions or system commands that trigger domain events.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Command text should describe an ACTION (imperative verb)',
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
        'Command text/name (PascalCase recommended, e.g., "RegisterUser", "PlaceOrder")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--actor <actor>',
      description:
        'Actor who executes the command (e.g., "User", "Admin", "System")',
      required: false,
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline timestamp in milliseconds (for temporal ordering)',
      required: false,
    },
    {
      flag: '--bounded-context <context>',
      description: 'Bounded context for domain association (DDD concept)',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec add-command AUTH-001 "RegisterUser"',
      description: 'Add command to work unit',
      output: '✓ Added command "RegisterUser" to AUTH-001 (ID: 0)',
    },
    {
      command: 'fspec add-command AUTH-001 "LoginUser" --actor "User"',
      description: 'Add command with actor specification',
      output: '✓ Added command "LoginUser" to AUTH-001 (ID: 1)',
    },
    {
      command:
        'fspec add-command CHECKOUT-001 "PlaceOrder" --actor "Customer" --bounded-context "Sales"',
      description: 'Add command with actor and bounded context',
      output: '✓ Added command "PlaceOrder" to CHECKOUT-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-domain-event',
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
    'Use PascalCase for command names (RegisterUser, PlaceOrder)',
    'Commands represent user INTENTIONS (imperative verbs)',
    'Commands typically trigger domain events (PlaceOrder → OrderPlaced)',
    'Use --actor to specify WHO executes the command',
    'Group related commands using --bounded-context flag',
  ],
  notes: [
    'Commands represent intentions that trigger state changes',
    'Commands assigned stable IDs starting from 0',
    'Deleted commands marked with deleted: true (soft delete)',
    'Use show-event-storm to view all Event Storm artifacts',
    'Bounded contexts help organize commands by subdomain (DDD pattern)',
  ],
};

export default config;
