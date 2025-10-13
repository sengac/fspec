import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'set-user-story',
  description:
    'Set the user story for a work unit, capturing the role, action, and benefit for Example Mapping',
  usage: 'fspec set-user-story <work-unit-id> [options]',
  whenToUse:
    'Use this command during Example Mapping (Discovery phase) to capture the user story BEFORE generating scenarios. This is the FIRST step in defining acceptance criteria. Essential for ensuring generated scenarios align with user needs.',
  whenNotToUse:
    'Do not use after generating scenarios (use update-work-unit to change fields). Do not use for technical implementation details (those go in architecture notes).',
  prerequisites: [
    'Work unit must exist (create with fspec create-work-unit)',
    'Work unit should be in "specifying" status',
  ],
  arguments: [
    {
      name: 'work-unit-id',
      description:
        'Work unit ID (e.g., AUTH-001, DASH-002). Must match existing work unit.',
      required: true,
    },
  ],
  options: [
    {
      flag: '--role <role>',
      description: 'User role or persona (e.g., "developer", "system admin")',
    },
    {
      flag: '--action <action>',
      description: 'What the user wants to do (e.g., "validate feature files")',
    },
    {
      flag: '--benefit <benefit>',
      description: 'Why the user wants it (e.g., "catch syntax errors early")',
    },
  ],
  examples: [
    {
      command:
        'fspec set-user-story AUTH-001 --role "developer" --action "validate feature files automatically" --benefit "I catch syntax errors before committing"',
      description: 'Set user story for authentication work unit',
      output:
        '✓ User story set for AUTH-001\n  As a developer\n  I want to validate feature files automatically\n  So that I catch syntax errors before committing',
    },
    {
      command:
        'fspec set-user-story DASH-002 --role "system admin" --action "view all work unit statuses in one place" --benefit "I can track team progress at a glance"',
      description: 'Set user story for dashboard work unit',
      output:
        '✓ User story set for DASH-002\n  As a system admin\n  I want to view all work unit statuses in one place\n  So that I can track team progress at a glance',
    },
  ],
  commonErrors: [
    {
      error: "Error: Work unit 'AUTH-001' does not exist",
      fix: 'Create work unit first: fspec create-work-unit AUTH "User Authentication"',
    },
    {
      error: 'Error: Missing required option --role',
      fix: 'Provide all three flags: --role, --action, and --benefit',
    },
  ],
  typicalWorkflow:
    '1. Create work unit → 2. Move to specifying status → 3. fspec set-user-story (this command) → 4. Add rules/examples → 5. fspec generate-scenarios',
  commonPatterns: [
    'Example Mapping: Set user story first, then add rules and examples before generating scenarios',
    'Feature Discovery: Capture user story during stakeholder conversations to ensure alignment',
    'ACDD Workflow: User story defines WHAT and WHY, scenarios define HOW',
  ],
  relatedCommands: [
    'create-work-unit',
    'generate-scenarios',
    'add-rule',
    'add-example',
    'show-work-unit',
  ],
  notes: [
    'User story format: "As a [role] I want to [action] So that [benefit]"',
    'ALWAYS set user story BEFORE generating scenarios (prevents prefill placeholders)',
    'Role should be a persona or user type, not a system component',
    'Action should describe capability, not implementation',
    'Benefit should explain business value or user outcome',
    'User story is stored in spec/work-units.json and used by generate-scenarios',
  ],
};

export default config;
