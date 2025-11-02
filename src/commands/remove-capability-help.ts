import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-capability',
  description:
    'Remove a capability from foundation.json or foundation.json.draft by exact name match (case-sensitive). Used to correct mistakes during foundation discovery or remove obsolete capabilities.',
  usage: 'fspec remove-capability "<name>"',
  whenToUse:
    'Use during foundation discovery to correct mistakes, or when updating an existing foundation to remove capabilities that are no longer part of the system.',
  arguments: [
    {
      name: 'name',
      description:
        'Exact name of the capability to remove (case-sensitive). Must match existing capability name exactly.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec remove-capability "User Authentication"',
      description: 'Remove capability by exact name',
      output:
        '✓ Removed capability "User Authentication" from foundation.json.draft',
    },
    {
      command: 'fspec remove-capability "Data Export"',
      description: 'Remove capability from finalized foundation',
      output: '✓ Removed capability "Data Export" from foundation.json',
    },
  ],
  prerequisites: [
    'foundation.json or foundation.json.draft must exist',
    'Capability must exist with exact name match',
  ],
  typicalWorkflow: [
    'List current capabilities: Check foundation.json or foundation.json.draft',
    'Remove capability: fspec remove-capability "exact name"',
    'Verify removal: Check updated foundation file',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      solution:
        'Run: fspec discover-foundation to create foundation.json.draft',
    },
    {
      error: 'Capability "Name" not found',
      solution:
        'Check exact name (case-sensitive) in foundation.json. Available capabilities are listed in the error message.',
    },
    {
      error: 'No capabilities exist in foundation',
      solution:
        'Cannot remove capabilities from empty foundation. Add capabilities first with fspec add-capability.',
    },
  ],
  relatedCommands: [
    'fspec add-capability - Add capability to foundation',
    'fspec discover-foundation - Foundation discovery process',
    'fspec remove-persona - Remove persona from foundation',
    'fspec update-foundation - Update foundation fields',
  ],
  notes: [
    'Draft file (foundation.json.draft) takes precedence over foundation.json',
    'Name matching is case-sensitive and must be exact',
    'Removal is permanent - no undo functionality',
    'Error message shows available capabilities if name not found',
  ],
};

export default config;
