import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'discover-event-storm',
  description:
    'Emit Event Storm guidance for domain discovery through collaborative workshop',
  usage: 'fspec discover-event-storm <workUnitId>',
  whenToUse:
    'Use BEFORE Example Mapping when domain complexity is high, events/commands/policies are unclear, or you need to model complex business workflows collaboratively.',
  prerequisites: [
    'Work unit must exist',
    'Work unit should be in specifying status',
    'High domain complexity or unclear requirements',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to conduct Event Storm discovery on',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec discover-event-storm AUTH-001',
      description:
        'Start Event Storm discovery session for authentication work unit',
      output:
        '✓ Event Storm guidance emitted\n\nFollow the guidance to capture domain events, commands, policies, and hotspots.',
    },
    {
      command: 'fspec discover-event-storm CHECKOUT-001',
      description: 'Start Event Storm discovery for checkout feature',
      output: '✓ Event Storm guidance emitted',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-policy',
    'add-hotspot',
    'add-aggregate',
    'add-bounded-context',
    'add-external-system',
    'show-event-storm',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
    {
      error: 'Work unit not in specifying status',
      fix: 'Move work unit to specifying: fspec update-work-unit-status <id> specifying',
    },
    {
      error: 'spec/work-units.json not found',
      fix: 'Initialize fspec first: fspec init',
    },
  ],
  commonPatterns: [
    'Run discover-event-storm BEFORE Example Mapping for complex domains',
    'Follow free-form collaborative session (not field-by-field like discover-foundation)',
    'Capture artifacts using add-domain-event, add-command, add-policy, add-hotspot',
    'Stop when shared understanding is reached (~25 minutes)',
    'Transform to Example Mapping: fspec generate-example-mapping-from-event-storm',
  ],
  notes: [
    'Event Storm is a workshop technique invented by Alberto Brandolini',
    'Use sticky notes metaphor: orange (events), blue (commands), purple (policies), red (hotspots)',
    'Two levels: Big Picture (foundation) and Process Modeling (work unit)',
    'Guidance emitted as system-reminder for AI agents',
    'Free-form session - use commands in any order as needed',
    'Stop when: all major events captured, commands identified, policies documented, hotspots recorded',
  ],
};

export default config;
