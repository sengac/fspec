import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-work-unit',
  description:
    'Create a new work unit for tracking a discrete piece of work through ACDD workflow',
  usage: 'fspec create-work-unit <prefix> <title> [options]',
  whenToUse:
    'Use when breaking down work into manageable units that will move through the ACDD workflow (backlog → specifying → testing → implementing → validating → done).',
  arguments: [
    {
      name: 'prefix',
      description:
        'Work unit prefix (e.g., AUTH, UI, API). Must be registered with create-prefix first.',
      required: true,
    },
    {
      name: 'title',
      description: 'Brief description of the work unit',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <description>',
      description: 'Detailed description of the work',
    },
    {
      flag: '-e, --epic <epic>',
      description: 'Epic ID to associate with this work unit',
    },
    {
      flag: '-p, --parent <parent>',
      description: 'Parent work unit ID for hierarchical relationships',
    },
  ],
  examples: [
    {
      command: 'fspec create-work-unit AUTH "User login feature"',
      description: 'Create simple work unit',
      output: '✓ Created work unit AUTH-001\n  Title: User login feature',
    },
    {
      command:
        'fspec create-work-unit AUTH "OAuth integration" --epic=user-management',
      description: 'Create work unit with epic',
      output:
        '✓ Created work unit AUTH-002\n  Title: OAuth integration\n  Epic: user-management',
    },
  ],
  prerequisites: [
    'Prefix must be registered first: fspec create-prefix AUTH "Authentication features"',
  ],
  typicalWorkflow:
    'create-work-unit → update-work-unit-status (move through workflow) → mark done',
  relatedCommands: [
    'list-work-units',
    'show-work-unit',
    'update-work-unit-status',
    'create-prefix',
    'create-epic',
  ],
  notes: [
    'Work unit IDs are auto-generated (PREFIX-###)',
    'New work units start in "backlog" status',
    'Title should be concise (1-2 sentences)',
  ],
};

export default config;
