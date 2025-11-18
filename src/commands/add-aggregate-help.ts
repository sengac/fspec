import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-aggregate',
  description:
    'Add aggregate to Event Storm section of work unit for Process Modeling Event Storming',
  usage: 'fspec add-aggregate <workUnitId> <text> [options]',
  whenToUse:
    'Use during Process Modeling Event Storming when identifying core domain entities (Aggregates) that enforce business rules and maintain consistency boundaries.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Aggregate text should be a noun representing a domain entity (e.g., Order, User, Account)',
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
        'Aggregate name (PascalCase recommended, e.g., "Order", "ShoppingCart", "UserAccount")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--responsibilities <list>',
      description:
        'Comma-separated list of aggregate responsibilities (e.g., "Validate order, Calculate total")',
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline timestamp in milliseconds (for temporal ordering)',
    },
    {
      flag: '--bounded-context <context>',
      description: 'Bounded context for domain association (DDD concept)',
    },
  ],
  examples: [
    {
      command: 'fspec add-aggregate CHECKOUT-001 "Order"',
      description: 'Add aggregate to work unit',
      output: '✓ Added aggregate "Order" to CHECKOUT-001 (ID: 0)',
    },
    {
      command:
        'fspec add-aggregate CHECKOUT-001 "ShoppingCart" --responsibilities "Add items, Calculate total, Apply discounts"',
      description: 'Add aggregate with responsibilities',
      output: '✓ Added aggregate "ShoppingCart" to CHECKOUT-001 (ID: 1)',
    },
    {
      command:
        'fspec add-aggregate AUTH-001 "UserAccount" --bounded-context "Identity"',
      description: 'Add aggregate with bounded context',
      output: '✓ Added aggregate "UserAccount" to AUTH-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-bounded-context',
    'show-event-storm',
    'generate-example-mapping-from-event-storm',
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
    'Use PascalCase for aggregate names (Order, ShoppingCart, UserAccount)',
    'Aggregates are nouns representing domain entities (not actions)',
    'Group related aggregates using --bounded-context flag',
    'Define responsibilities to clarify aggregate behavior',
    'Aggregates enforce business invariants and consistency boundaries',
  ],
  notes: [
    'Aggregates are DDD pattern representing consistency boundaries',
    'Aggregates assigned stable IDs starting from 0',
    'Deleted aggregates marked with deleted: true (soft delete)',
    'Use show-event-storm to view all Event Storm artifacts',
    'Aggregates have yellow sticky notes in physical Event Storming',
  ],
};

export default config;
