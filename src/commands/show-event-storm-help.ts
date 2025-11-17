import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-event-storm',
  description:
    'Display Event Storm artifacts as JSON (no semantic interpretation)',
  usage: 'fspec show-event-storm <work-unit-id>',
  whenToUse:
    'Use to view all Event Storm artifacts (events, commands, policies, hotspots) for a specific work unit in JSON format.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
  ],
  arguments: [
    {
      name: 'work-unit-id',
      description: 'Work unit ID to query',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec show-event-storm AUTH-001',
      description: 'Display Event Storm artifacts for work unit',
      output:
        '{\n  "items": [\n    {"id": 0, "type": "domain-event", "text": "UserRegistered"},\n    {"id": 1, "type": "command", "text": "RegisterUser"},\n    {"id": 2, "type": "policy", "text": "Send welcome email", "when": "UserRegistered", "then": "SendWelcomeEmail"}\n  ],\n  "nextItemId": 3\n}',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-policy',
    'add-hotspot',
    'show-foundation-event-storm',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
    {
      error: 'No Event Storm artifacts',
      fix: 'Add Event Storm items first: fspec add-domain-event <id> <text>',
    },
  ],
  commonPatterns: [
    'Use for debugging Event Storm structure',
    'Verify Event Storm artifacts before generating Example Mapping',
    'Check item IDs and types',
    'View all artifacts in a single work unit',
  ],
  notes: [
    'Output is raw JSON (not formatted for human reading)',
    'Shows ALL items including deleted ones (soft-delete pattern)',
    'Use show-foundation-event-storm for foundation-level Event Storm',
    'Use generate-example-mapping-from-event-storm to convert to Example Mapping',
  ],
};

export default config;
