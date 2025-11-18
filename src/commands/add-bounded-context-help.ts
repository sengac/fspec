import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-bounded-context',
  description:
    'Add bounded context to Event Storm section of work unit for organizing domain boundaries',
  usage: 'fspec add-bounded-context <workUnitId> <text> [options]',
  whenToUse:
    'Use during Event Storming when identifying strategic boundaries (Bounded Contexts) that separate distinct subdomains or business capabilities in your system.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Bounded context should represent a clear strategic boundary in the domain',
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
        'Bounded context name (e.g., "Order Management", "Inventory", "Identity")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--description <text>',
      description: 'Description of bounded context purpose and scope',
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline timestamp in milliseconds (for temporal ordering)',
    },
  ],
  examples: [
    {
      command: 'fspec add-bounded-context CHECKOUT-001 "Order Management"',
      description: 'Add bounded context to work unit',
      output:
        '✓ Added bounded context "Order Management" to CHECKOUT-001 (ID: 0)',
    },
    {
      command:
        'fspec add-bounded-context CHECKOUT-001 "Inventory" --description "Manages product stock and warehouse operations"',
      description: 'Add bounded context with description',
      output: '✓ Added bounded context "Inventory" to CHECKOUT-001 (ID: 1)',
    },
    {
      command:
        'fspec add-bounded-context AUTH-001 "Identity" --description "User authentication and authorization"',
      description: 'Add identity bounded context',
      output: '✓ Added bounded context "Identity" to AUTH-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-aggregate',
    'add-domain-event',
    'add-foundation-bounded-context',
    'show-event-storm',
    'show-foundation-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
    {
      error: 'Cannot add Event Storm items to work unit in done state',
      fix: 'Work unit must be in specifying, testing, implementing, or validating state',
    },
    {
      error: 'spec/work-units.json not found',
      fix: 'Initialize fspec first: fspec init',
    },
  ],
  commonPatterns: [
    'Bounded contexts represent strategic boundaries in your domain',
    'Each bounded context has its own ubiquitous language',
    'Use bounded contexts to organize aggregates, events, and commands',
    'Bounded contexts communicate through well-defined interfaces',
    'Foundation-level bounded contexts (Big Picture) vs work unit-level (Process Modeling)',
  ],
  notes: [
    'Bounded Contexts are a core DDD pattern for managing complexity',
    'Work unit bounded contexts are tactical (Process Modeling level)',
    'Foundation bounded contexts are strategic (Big Picture level)',
    'Bounded contexts assigned stable IDs starting from 0',
    'Deleted bounded contexts marked with deleted: true (soft delete)',
    'Use show-event-storm to view all Event Storm artifacts',
  ],
};

export default config;
