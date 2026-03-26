import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-aggregate-from-foundation',
  description:
    'Remove an aggregate from a foundation bounded context (soft-delete)',
  usage:
    'fspec remove-aggregate-from-foundation <context-name> <aggregate-name>',
  whenToUse:
    'Use to remove an aggregate from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.',
  prerequisites: [
    'spec/foundation.json must exist with Event Storm data',
    'The target bounded context must exist and not be deleted',
    'The aggregate must exist within the specified bounded context',
  ],
  arguments: [
    {
      name: '<context-name>',
      description: 'Name of the bounded context containing the aggregate',
      required: true,
    },
    {
      name: '<aggregate-name>',
      description: 'Name of the aggregate to remove (e.g., "WorkUnit", "User")',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec remove-aggregate-from-foundation "Work Management" "LegacyItem"',
      description: 'Remove an aggregate from a bounded context',
      output:
        '✓ Removed aggregate "LegacyItem" from "Work Management" bounded context\n✓ Regenerated FOUNDATION.md',
    },
    {
      command: 'fspec remove-aggregate-from-foundation "Identity" "OldUser"',
      description: 'Remove aggregate from Identity bounded context',
      output:
        '✓ Removed aggregate "OldUser" from "Identity" bounded context\n✓ Regenerated FOUNDATION.md',
    },
  ],
  relatedCommands: [
    'add-aggregate-to-foundation',
    'remove-foundation-bounded-context',
    'remove-domain-event-from-foundation',
    'remove-command-from-foundation',
    'show-foundation-event-storm',
  ],
  commonErrors: [
    {
      error: "Bounded context 'Foo' not found",
      fix: 'Check context names with: fspec show-foundation-event-storm --type bounded-context',
    },
    {
      error: "Aggregate 'Bar' not found in bounded context 'Foo'",
      fix: 'Check aggregate names with: fspec show-foundation-event-storm',
    },
    {
      error: 'spec/foundation.json not found',
      fix: 'Initialize foundation first: fspec discover-foundation',
    },
  ],
  commonPatterns: [
    'Requires both context name and aggregate name (matching the add command signature)',
    'Uses soft-delete (sets deleted: true) — items are not permanently erased',
    'FOUNDATION.md is auto-regenerated after removal',
    'Only removes the aggregate — other items in the context are unaffected',
  ],
  notes: [
    'Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern',
    'Both the bounded context and aggregate must exist and not be deleted',
    'FOUNDATION.md is automatically regenerated after removal',
    'Uses fileManager.transaction() for atomic updates',
    'Aggregates are identified by name within their bounded context',
  ],
};

export default config;
