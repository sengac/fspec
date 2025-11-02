import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-persona',
  description:
    'Remove a persona from foundation.json or foundation.json.draft by exact name match (case-sensitive). Used to correct mistakes during foundation discovery or remove obsolete personas.',
  usage: 'fspec remove-persona "<name>"',
  whenToUse:
    'Use during foundation discovery to correct mistakes, or when updating an existing foundation to remove personas that are no longer relevant to the system.',
  arguments: [
    {
      name: 'name',
      description:
        'Exact name of the persona to remove (case-sensitive). Must match existing persona name exactly.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec remove-persona "Developer"',
      description: 'Remove persona by exact name',
      output: '✓ Removed persona "Developer" from foundation.json.draft',
    },
    {
      command: 'fspec remove-persona "End User"',
      description: 'Remove persona from finalized foundation',
      output: '✓ Removed persona "End User" from foundation.json',
    },
  ],
  prerequisites: [
    'foundation.json or foundation.json.draft must exist',
    'Persona must exist with exact name match',
  ],
  typicalWorkflow: [
    'List current personas: Check foundation.json or foundation.json.draft',
    'Remove persona: fspec remove-persona "exact name"',
    'Verify removal: Check updated foundation file',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      solution:
        'Run: fspec discover-foundation to create foundation.json.draft',
    },
    {
      error: 'Persona "Name" not found',
      solution:
        'Check exact name (case-sensitive) in foundation.json. Available personas are listed in the error message.',
    },
    {
      error: 'No personas exist in foundation',
      solution:
        'Cannot remove personas from empty foundation. Add personas first with fspec add-persona.',
    },
  ],
  relatedCommands: [
    'fspec add-persona - Add persona to foundation',
    'fspec discover-foundation - Foundation discovery process',
    'fspec remove-capability - Remove capability from foundation',
    'fspec update-foundation - Update foundation fields',
  ],
  notes: [
    'Draft file (foundation.json.draft) takes precedence over foundation.json',
    'Name matching is case-sensitive and must be exact',
    'Removal is permanent - no undo functionality',
    'Error message shows available personas if name not found',
  ],
};

export default config;
