import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-domain-event-from-foundation',
  description:
    'Remove a domain event from a foundation bounded context (soft-delete)',
  usage:
    'fspec remove-domain-event-from-foundation <context-name> <event-name>',
  whenToUse:
    'Use to remove a domain event from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.',
  prerequisites: [
    'spec/foundation.json must exist with Event Storm data',
    'The target bounded context must exist and not be deleted',
    'The domain event must exist within the specified bounded context',
  ],
  arguments: [
    {
      name: '<context-name>',
      description: 'Name of the bounded context containing the domain event',
      required: true,
    },
    {
      name: '<event-name>',
      description:
        'Name of the domain event to remove (e.g., "WorkUnitCreated", "UserLoggedIn")',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec remove-domain-event-from-foundation "Work Management" "LegacyEventFired"',
      description: 'Remove a domain event from a bounded context',
      output:
        '✓ Removed domain event "LegacyEventFired" from "Work Management" bounded context\n✓ Regenerated FOUNDATION.md',
    },
    {
      command:
        'fspec remove-domain-event-from-foundation "Identity" "OldUserLoggedIn"',
      description: 'Remove domain event from Identity bounded context',
      output:
        '✓ Removed domain event "OldUserLoggedIn" from "Identity" bounded context\n✓ Regenerated FOUNDATION.md',
    },
  ],
  relatedCommands: [
    'add-domain-event-to-foundation',
    'remove-foundation-bounded-context',
    'remove-aggregate-from-foundation',
    'remove-command-from-foundation',
    'show-foundation-event-storm',
  ],
  commonErrors: [
    {
      error: "Bounded context 'Foo' not found",
      fix: 'Check context names with: fspec show-foundation-event-storm --type bounded-context',
    },
    {
      error: "Domain event 'Bar' not found in bounded context 'Foo'",
      fix: 'Check event names with: fspec show-foundation-event-storm',
    },
    {
      error: 'spec/foundation.json not found',
      fix: 'Initialize foundation first: fspec discover-foundation',
    },
  ],
  commonPatterns: [
    'Requires both context name and event name (matching the add command signature)',
    'Uses soft-delete (sets deleted: true) — items are not permanently erased',
    'FOUNDATION.md is auto-regenerated after removal',
    'Only removes the domain event — other items in the context are unaffected',
  ],
  notes: [
    'Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern',
    'Both the bounded context and domain event must exist and not be deleted',
    'FOUNDATION.md is automatically regenerated after removal',
    'Uses fileManager.transaction() for atomic updates',
    'Domain events are identified by name within their bounded context',
  ],
};

export default config;
