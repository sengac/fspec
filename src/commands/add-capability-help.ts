import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-capability',
  description:
    'Add a capability to foundation.json or foundation.json.draft. Capabilities describe WHAT the system CAN DO (from the user perspective), not HOW it works internally. Used during foundation discovery to incrementally build the solution space.',
  usage: 'fspec add-capability "<name>" "<description>"',
  whenToUse:
    'Use during fspec discover-foundation workflow when adding capabilities to foundation.json.draft, or when adding new capabilities to an existing foundation.json after initial discovery.',
  arguments: [
    {
      name: 'name',
      description:
        'Capability name (e.g., "User Authentication", "Data Export"). Should describe what users can do, not implementation details.',
      required: true,
    },
    {
      name: 'description',
      description:
        'Description of the capability from user perspective. Focus on WHAT users can achieve, not HOW it works.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec add-capability "User Authentication" "Users can register, login, and manage their accounts"',
      description: 'Add capability during foundation discovery',
      output:
        '✓ Added capability to foundation.json.draft\n  Name: User Authentication\n  Description: Users can register, login, and manage their accounts',
    },
    {
      command:
        'fspec add-capability "Data Export" "Users can export their data in multiple formats (CSV, JSON, PDF)"',
      description: 'Add capability to existing foundation',
      output:
        '✓ Added capability to foundation.json\n  Name: Data Export\n  Description: Users can export their data in multiple formats (CSV, JSON, PDF)',
    },
  ],
  prerequisites: [
    'foundation.json or foundation.json.draft must exist',
    'Run fspec discover-foundation to create foundation.json.draft if needed',
  ],
  typicalWorkflow: [
    'Start foundation discovery: fspec discover-foundation',
    'AI analyzes codebase and identifies capabilities',
    'Add each capability: fspec add-capability "name" "description"',
    'Repeat for all capabilities',
    'Finalize foundation: fspec discover-foundation --finalize',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      solution:
        'Run: fspec discover-foundation to create foundation.json.draft',
    },
    {
      error: 'Capability already exists',
      solution:
        'Check existing capabilities in foundation.json or use a different name',
    },
  ],
  relatedCommands: [
    'fspec discover-foundation - Start foundation discovery process',
    'fspec remove-capability - Remove capability from foundation',
    'fspec add-persona - Add user persona to foundation',
    'fspec update-foundation - Update foundation fields',
  ],
  notes: [
    'Draft file (foundation.json.draft) takes precedence over foundation.json',
    'Capabilities describe WHAT users can do, not HOW system works',
    'Focus on user-facing functionality, not implementation details',
    'Capabilities are part of the solution space (what system provides)',
    'Use present tense and active voice (e.g., "Users can..." not "User will...")',
  ],
};

export default config;
