import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-work-unit',
  description:
    'Update work unit metadata including title, description, epic association, and parent relationship',
  usage: 'fspec update-work-unit <workUnitId> [options]',
  whenToUse:
    'Use this command to modify work unit metadata after creation. Common use cases: refining title/description, changing epic association, or restructuring parent-child relationships. NOTE: Work unit type (story/bug/task) is immutable and cannot be changed.',
  prerequisites: ['Work unit must exist in spec/work-units.json'],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to update (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [
    {
      flag: '-t, --title <title>',
      description: 'New title for the work unit',
    },
    {
      flag: '-d, --description <description>',
      description: 'New description for the work unit',
    },
    {
      flag: '-e, --epic <epic>',
      description: 'Epic ID to associate with (moves work unit to new epic)',
    },
    {
      flag: '-p, --parent <parent>',
      description:
        'Parent work unit ID (for hierarchical relationships, prevents circular references)',
    },
  ],
  examples: [
    {
      command:
        'fspec update-work-unit AUTH-001 --title "OAuth 2.0 integration"',
      description: 'Update work unit title',
      output: '✓ Work unit AUTH-001 updated successfully',
    },
    {
      command:
        'fspec update-work-unit AUTH-001 --description "Implement OAuth 2.0 authentication flow"',
      description: 'Update work unit description',
      output: '✓ Work unit AUTH-001 updated successfully',
    },
    {
      command: 'fspec update-work-unit AUTH-001 --epic AUTH',
      description: 'Move work unit to different epic',
      output: '✓ Work unit AUTH-001 updated successfully',
    },
    {
      command: 'fspec update-work-unit AUTH-002 --parent AUTH-001',
      description: 'Set parent-child relationship',
      output: '✓ Work unit AUTH-002 updated successfully',
    },
    {
      command:
        'fspec update-work-unit AUTH-001 --title "New title" --description "New description" --epic API',
      description: 'Update multiple fields at once',
      output: '✓ Work unit AUTH-001 updated successfully',
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-999' does not exist",
      fix: 'Verify the work unit ID exists with: fspec list-work-units',
    },
    {
      error: "Epic 'NONEXISTENT' does not exist",
      fix: 'Check available epics with: fspec list-epics or create epic with: fspec create-epic',
    },
    {
      error: "Parent work unit 'AUTH-999' does not exist",
      fix: 'Verify parent ID exists with: fspec list-work-units',
    },
    {
      error: 'Circular parent relationship detected',
      fix: 'Cannot set parent to a work unit that is already a descendant. Check hierarchy with: fspec show-work-unit <id>',
    },
    {
      error: 'Work unit type is immutable and cannot be changed after creation',
      fix: 'Delete this work unit and create a new one with the correct type (story/bug/task)',
    },
  ],
  typicalWorkflow:
    '1. View current metadata: fspec show-work-unit <workUnitId> → 2. Update fields: fspec update-work-unit <workUnitId> --title "New title" → 3. Verify changes: fspec show-work-unit <workUnitId>',
  commonPatterns: [
    {
      pattern: 'Refine work unit details',
      example:
        '# Initial creation\nfspec create-story AUTH "User auth"\n\n# Refine after Example Mapping\nfspec update-work-unit AUTH-001 --title "OAuth 2.0 authentication" --description "Implement OAuth 2.0 flow with Google"',
    },
    {
      pattern: 'Reorganize epic structure',
      example:
        '# Move work units to different epic\nfspec update-work-unit AUTH-001 --epic SECURITY\nfspec update-work-unit AUTH-002 --epic SECURITY\n\n# Verify epic structure\nfspec show-epic SECURITY',
    },
    {
      pattern: 'Create parent-child relationships',
      example:
        '# Break large work unit into subtasks\nfspec create-story AUTH "OAuth implementation" --parent AUTH-001\nfspec create-task AUTH "OAuth testing" --parent AUTH-001\n\n# View hierarchy\nfspec show-work-unit AUTH-001',
    },
  ],
  relatedCommands: [
    'show-work-unit',
    'update-work-unit-status',
    'update-work-unit-estimate',
    'create-story',
    'create-bug',
    'create-task',
    'list-work-units',
  ],
  notes: [
    'Provide at least one option (--title, --description, --epic, or --parent)',
    'Work unit type is immutable - delete and recreate to change type',
    'Changing epic moves work unit from old epic to new epic automatically',
    'Parent-child relationships are validated for circular references',
    'Epic references are automatically updated in spec/epics.json',
    'Parent references update children arrays in both old and new parent work units',
    'UpdatedAt timestamp is automatically set to current time',
  ],
};

export default config;
