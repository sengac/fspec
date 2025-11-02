import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-persona',
  description:
    'Add a user persona to foundation.json or foundation.json.draft. Personas describe WHO uses the system, their characteristics, and their goals. Used during foundation discovery to define the problem space.',
  usage: 'fspec add-persona "<name>" "<description>" [options]',
  whenToUse:
    'Use during fspec discover-foundation workflow when adding personas to foundation.json.draft, or when adding new personas to an existing foundation.json after initial discovery.',
  arguments: [
    {
      name: 'name',
      description:
        'Persona name (e.g., "Developer", "End User", "Administrator"). Should represent a distinct user type.',
      required: true,
    },
    {
      name: 'description',
      description:
        'Description of the persona: who they are, their characteristics, and how they interact with the system.',
      required: true,
    },
  ],
  options: [
    {
      flag: '--goal <goal>',
      description:
        'Primary goal for this persona (can be specified multiple times for multiple goals)',
    },
  ],
  examples: [
    {
      command:
        'fspec add-persona "Developer" "Software developer using fspec for project management" --goal "Track work efficiently" --goal "Generate documentation"',
      description: 'Add persona with multiple goals',
      output:
        '✓ Added persona to foundation.json.draft\n  Name: Developer\n  Description: Software developer using fspec for project management\n  Goals: Track work efficiently, Generate documentation',
    },
    {
      command:
        'fspec add-persona "End User" "Person using the application features" --goal "Complete tasks quickly"',
      description: 'Add persona with single goal',
      output:
        '✓ Added persona to foundation.json.draft\n  Name: End User\n  Description: Person using the application features\n  Goals: Complete tasks quickly',
    },
  ],
  prerequisites: [
    'foundation.json or foundation.json.draft must exist',
    'Run fspec discover-foundation to create foundation.json.draft if needed',
  ],
  typicalWorkflow: [
    'Start foundation discovery: fspec discover-foundation',
    'AI analyzes codebase and identifies user types',
    'Add each persona: fspec add-persona "name" "description" --goal "goal1" --goal "goal2"',
    'Repeat for all personas',
    'Finalize foundation: fspec discover-foundation --finalize',
  ],
  commonErrors: [
    {
      error: 'foundation.json not found',
      solution:
        'Run: fspec discover-foundation to create foundation.json.draft',
    },
    {
      error: 'Persona already exists',
      solution:
        'Check existing personas in foundation.json or use a different name',
    },
  ],
  relatedCommands: [
    'fspec discover-foundation - Start foundation discovery process',
    'fspec remove-persona - Remove persona from foundation',
    'fspec add-capability - Add capability to foundation',
    'fspec update-foundation - Update foundation fields',
  ],
  notes: [
    'Draft file (foundation.json.draft) takes precedence over foundation.json',
    'Personas describe WHO uses the system (problem space)',
    'Goals should describe what the persona wants to achieve',
    'Personas help inform Example Mapping and acceptance criteria',
    'Multiple goals can be specified using multiple --goal flags',
  ],
};

export default config;
