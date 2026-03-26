import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-command-from-foundation',
  description:
    'Remove a command from a foundation bounded context (soft-delete)',
  usage: 'fspec remove-command-from-foundation <context-name> <command-name>',
  whenToUse:
    'Use to remove a command from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.',
  prerequisites: [
    'spec/foundation.json must exist with Event Storm data',
    'The target bounded context must exist and not be deleted',
    'The command must exist within the specified bounded context',
  ],
  arguments: [
    {
      name: '<context-name>',
      description: 'Name of the bounded context containing the command',
      required: true,
    },
    {
      name: '<command-name>',
      description:
        'Name of the command to remove (e.g., "CreateWorkUnit", "UpdateStatus")',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec remove-command-from-foundation "Work Management" "DeprecatedAction"',
      description: 'Remove a command from a bounded context',
      output:
        '✓ Removed command "DeprecatedAction" from "Work Management" bounded context\n✓ Regenerated FOUNDATION.md',
    },
    {
      command:
        'fspec remove-command-from-foundation "Identity" "OldAuthenticateUser"',
      description: 'Remove command from Identity bounded context',
      output:
        '✓ Removed command "OldAuthenticateUser" from "Identity" bounded context\n✓ Regenerated FOUNDATION.md',
    },
  ],
  relatedCommands: [
    'add-command-to-foundation',
    'remove-foundation-bounded-context',
    'remove-aggregate-from-foundation',
    'remove-domain-event-from-foundation',
    'show-foundation-event-storm',
  ],
  commonErrors: [
    {
      error: "Bounded context 'Foo' not found",
      fix: 'Check context names with: fspec show-foundation-event-storm --type bounded-context',
    },
    {
      error: "Command 'Bar' not found in bounded context 'Foo'",
      fix: 'Check command names with: fspec show-foundation-event-storm',
    },
    {
      error: 'spec/foundation.json not found',
      fix: 'Initialize foundation first: fspec discover-foundation',
    },
  ],
  commonPatterns: [
    'Requires both context name and command name (matching the add command signature)',
    'Uses soft-delete (sets deleted: true) — items are not permanently erased',
    'FOUNDATION.md is auto-regenerated after removal',
    'Only removes the command — other items in the context are unaffected',
  ],
  notes: [
    'Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern',
    'Both the bounded context and command must exist and not be deleted',
    'FOUNDATION.md is automatically regenerated after removal',
    'Uses fileManager.transaction() for atomic updates',
    'Commands are identified by name within their bounded context',
  ],
};

export default config;
