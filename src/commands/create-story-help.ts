import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-story',
  description:
    'Create a new story with Example Mapping guidance for defining acceptance criteria',
  usage: 'fspec create-story <prefix> <title> [options]',
  whenToUse:
    'Use when creating user-facing features that require Example Mapping to clarify acceptance criteria through collaborative conversation (rules, examples, questions).',
  arguments: [
    {
      name: 'prefix',
      description:
        'Story prefix (e.g., AUTH, UI, API). Must be registered with create-prefix first.',
      required: true,
    },
    {
      name: 'title',
      description: 'Brief description of the story',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <description>',
      description: 'Detailed description of the story',
    },
    {
      flag: '-e, --epic <epic>',
      description: 'Epic ID to associate with this story',
    },
    {
      flag: '-p, --parent <parent>',
      description: 'Parent story ID for hierarchical relationships',
    },
  ],
  examples: [
    {
      command: 'fspec create-story AUTH "User login feature"',
      description: 'Create simple story with Example Mapping guidance',
      output:
        '✓ Created story AUTH-001\n  Title: User login feature\n\n<system-reminder>\nStory AUTH-001 created successfully.\n\nNext steps - Example Mapping:\n  fspec add-rule AUTH-001 "Rule text"\n  fspec add-example AUTH-001 "Example text"\n  fspec add-question AUTH-001 "@human: Question?"\n  fspec set-user-story AUTH-001 --role "..." --action "..." --benefit "..."\n</system-reminder>',
    },
    {
      command:
        'fspec create-story AUTH "OAuth integration" --epic=user-management',
      description: 'Create story with epic',
      output:
        '✓ Created story AUTH-002\n  Title: OAuth integration\n  Epic: user-management',
    },
    {
      command:
        'fspec create-story AUTH "Google login" --parent=AUTH-001 --description="Support Google OAuth"',
      description: 'Create child story with description',
      output:
        '✓ Created story AUTH-003\n  Title: Google login\n  Description: Support Google OAuth\n  Parent: AUTH-001',
    },
  ],
  prerequisites: [
    'Prefix must be registered: fspec create-prefix PREFIX "Description"',
    'Epic must exist if using --epic: fspec create-epic EPIC "Title"',
    'Parent story must exist if using --parent',
  ],
  typicalWorkflow: [
    'Create story: fspec create-story PREFIX "Title"',
    'Follow Example Mapping guidance from system-reminder',
    'Set user story: fspec set-user-story STORY-001 --role "..." --action "..." --benefit "..."',
    'Add rules: fspec add-rule STORY-001 "Rule text"',
    'Add examples: fspec add-example STORY-001 "Example text"',
    'Add questions: fspec add-question STORY-001 "@human: Question?"',
    'Generate scenarios: fspec generate-scenarios STORY-001',
    'Move to specifying: fspec update-work-unit-status STORY-001 specifying',
  ],
  commonErrors: [
    {
      error: "Prefix 'PREFIX' is not registered",
      solution: 'Run: fspec create-prefix PREFIX "Description"',
    },
    {
      error: "Parent story 'PARENT-001' does not exist",
      solution: 'Create parent first or remove --parent option',
    },
    {
      error: "Epic 'epic-name' does not exist",
      solution: 'Run: fspec create-epic epic-name "Title"',
    },
  ],
  relatedCommands: [
    'fspec add-rule - Add business rule to story',
    'fspec add-example - Add concrete example to story',
    'fspec add-question - Add question for human clarification',
    'fspec set-user-story - Set user story fields (role, action, benefit)',
    'fspec generate-scenarios - Generate Gherkin scenarios from example map',
    'fspec update-work-unit-status - Move story through ACDD workflow',
  ],
  notes: [
    'Stories use Example Mapping for collaborative requirement discovery',
    'System-reminder guides AI agents to use Example Mapping commands',
    'Stories require feature files with acceptance criteria',
    'Stories require tests before implementation (ACDD)',
  ],
};

export default config;
