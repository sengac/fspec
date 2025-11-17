import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-example-mapping-from-event-storm',
  description:
    'Generate Example Mapping entries (rules, examples, questions) from Event Storm artifacts',
  usage: 'fspec generate-example-mapping-from-event-storm <workUnitId>',
  whenToUse:
    'Use after completing Big Picture Event Storming to convert Event Storm artifacts (policies, events, hotspots) into Example Mapping for detailed specification.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have Event Storm artifacts (policies, events, or hotspots)',
    'Event Storm section must be initialized',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID containing Event Storm artifacts',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec generate-example-mapping-from-event-storm AUTH-001',
      description: 'Convert Event Storm artifacts to Example Mapping',
      output:
        '✓ Generated 3 rule(s) from policies\n✓ Generated 5 example(s) from events\n✓ Generated 2 question(s) from hotspots',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-policy',
    'add-hotspot',
    'show-event-storm',
    'add-rule',
    'add-example',
    'add-question',
    'generate-scenarios',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
    {
      error: 'No Event Storm artifacts found',
      fix: 'Add Event Storm items first: fspec add-domain-event, add-policy, etc.',
    },
  ],
  commonPatterns: [
    'Policies → Business rules (WHEN/THEN relationships)',
    'Events → Scenario examples (UserRegistered → "User user registered and is logged in")',
    'Hotspots → Questions for humans (concerns → @human questions)',
    'Use after Event Storming, before generate-scenarios',
    'Bridge from strategic discovery (Event Storm) to tactical specification (Example Mapping)',
  ],
  notes: [
    'Policies converted to rules using WHEN/THEN pattern',
    'Events converted to examples using PascalCase → natural language',
    'Hotspot concerns converted to @human questions',
    'Skips deleted Event Storm items (deleted: true)',
    'Generated items include timestamps and proper ID sequencing',
    'After generation, use fspec generate-scenarios to create feature file',
  ],
};

export default config;
