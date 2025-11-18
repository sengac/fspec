import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-foundation-bounded-context',
  description:
    'Add bounded context to foundation-level Big Picture Event Storm for strategic domain boundaries',
  usage: 'fspec add-foundation-bounded-context <text>',
  whenToUse:
    'Use during Big Picture Event Storming (foundation level) to identify major strategic boundaries (Bounded Contexts) in your domain architecture.',
  prerequisites: [
    'spec/foundation.json must exist',
    'Bounded context should represent a strategic domain boundary',
    'Should be used for high-level domain architecture (not work unit-level)',
  ],
  arguments: [
    {
      name: 'text',
      description:
        'Bounded context name (e.g., "Work Management", "Specification", "Testing & Validation")',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec add-foundation-bounded-context "Work Management"',
      description: 'Add bounded context to foundation Event Storm',
      output:
        '✓ Added bounded context "Work Management" to foundation Event Storm\n✓ Regenerated FOUNDATION.md',
    },
    {
      command: 'fspec add-foundation-bounded-context "Specification"',
      description: 'Add specification bounded context',
      output:
        '✓ Added bounded context "Specification" to foundation Event Storm\n✓ Regenerated FOUNDATION.md',
    },
    {
      command: 'fspec add-foundation-bounded-context "Testing & Validation"',
      description: 'Add testing bounded context',
      output:
        '✓ Added bounded context "Testing & Validation" to foundation Event Storm\n✓ Regenerated FOUNDATION.md',
    },
  ],
  relatedCommands: [
    'add-aggregate-to-foundation',
    'add-domain-event-to-foundation',
    'add-command-to-foundation',
    'show-foundation-event-storm',
    'add-bounded-context',
  ],
  commonErrors: [
    {
      error: 'spec/foundation.json not found',
      fix: 'Run foundation discovery first: fspec discover-foundation',
    },
  ],
  commonPatterns: [
    'Foundation bounded contexts are strategic (Big Picture level)',
    'Work unit bounded contexts are tactical (Process Modeling level)',
    'Typically 3-8 major bounded contexts for most domains',
    'Each bounded context has its own ubiquitous language',
    'Foundation Event Storm informs tag ontology generation',
    'Auto-regenerates FOUNDATION.md after adding bounded context',
  ],
  notes: [
    'Foundation-level bounded contexts define strategic domain architecture',
    'Used for Big Picture Event Storming (not Process Modeling)',
    'Bounded contexts assigned stable IDs starting from 1',
    'Stored in foundation.json eventStorm section',
    'Auto-regenerates FOUNDATION.md documentation',
    'Use show-foundation-event-storm to view all foundation Event Storm items',
    'Foundation bounded contexts used for tag derivation (EXMAP-004)',
  ],
};

export default config;
