import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-external-system',
  description:
    'Add external system to Event Storm section of work unit to identify third-party integrations and boundaries',
  usage: 'fspec add-external-system <workUnitId> <text> [options]',
  whenToUse:
    'Use during Event Storming when identifying external systems, third-party services, databases, message queues, or file systems that your domain interacts with.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'External system should represent a clear external dependency',
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
        'External system name (e.g., "Payment Gateway", "Email Service", "User Database")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--type <type>',
      description:
        'External system type: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM',
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline timestamp in milliseconds (for temporal ordering)',
    },
    {
      flag: '--bounded-context <context>',
      description: 'Bounded context for domain association',
    },
  ],
  examples: [
    {
      command: 'fspec add-external-system CHECKOUT-001 "Payment Gateway"',
      description: 'Add external system to work unit',
      output:
        '✓ Added external system "Payment Gateway" to CHECKOUT-001 (ID: 0)',
    },
    {
      command:
        'fspec add-external-system CHECKOUT-001 "Stripe API" --type REST_API',
      description: 'Add external system with type',
      output: '✓ Added external system "Stripe API" to CHECKOUT-001 (ID: 1)',
    },
    {
      command:
        'fspec add-external-system AUTH-001 "User Database" --type DATABASE --bounded-context "Identity"',
      description: 'Add external system with type and bounded context',
      output: '✓ Added external system "User Database" to AUTH-001 (ID: 0)',
    },
    {
      command:
        'fspec add-external-system NOTIFY-001 "RabbitMQ" --type MESSAGE_QUEUE',
      description: 'Add message queue external system',
      output: '✓ Added external system "RabbitMQ" to NOTIFY-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-policy',
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
      error: 'Invalid external system type',
      fix: 'Use one of: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM',
    },
    {
      error: 'spec/work-units.json not found',
      fix: 'Initialize fspec first: fspec init',
    },
  ],
  commonPatterns: [
    'External systems help identify integration points and dependencies',
    'Use --type to categorize external systems for better documentation',
    'External systems represent boundaries between your domain and the outside world',
    'Consider anti-corruption layers when integrating external systems',
    'External systems have pink sticky notes in physical Event Storming',
  ],
  notes: [
    'External systems assigned stable IDs starting from 0',
    'Deleted external systems marked with deleted: true (soft delete)',
    'Use show-event-storm to view all Event Storm artifacts',
    'External systems help identify where to place adapters/ports in clean architecture',
    'Color: pink (matching physical Event Storming convention)',
  ],
};

export default config;
