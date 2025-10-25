import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'show-work-unit',
  description:
    'Display detailed information about a work unit including Example Mapping data, dependencies, and linked feature files',
  usage: 'fspec show-work-unit <workUnitId> [options]',
  whenToUse:
    'Use to view complete details of a work unit: status, type, description, Example Mapping (rules, examples, questions, assumptions), dependencies (blocks/blockedBy/dependsOn/relatesTo), linked feature files, attachments, and system reminders. Supports both human-readable text and JSON output.',
  prerequisites: ['Work unit must exist in spec/work-units.json'],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [
    {
      flag: '-f, --format <format>',
      description: 'Output format: text (default) or json',
    },
  ],
  examples: [
    {
      command: 'fspec show-work-unit AUTH-001',
      description: 'Show work unit details in text format',
      output:
        'AUTH-001\nType: story\nStatus: specifying\n\nUser login feature\nImplement user authentication\n\nRules:\n  1. Must validate email format\n  2. Password must be 8+ characters\n\nQuestions:\n  [0] Should we support OAuth?\n\nLinked Features:\n  spec/features/auth/login.feature\n    spec/features/auth/login.feature:10 - Valid user login\n\nCreated: 13/10/2025 14:30:00\nUpdated: 13/10/2025 15:45:00',
    },
    {
      command: 'fspec show-work-unit AUTH-001 --format json',
      description: 'Show work unit details as JSON',
      output:
        '{\n  "id": "AUTH-001",\n  "title": "User login feature",\n  "type": "story",\n  "status": "specifying",\n  "description": "Implement user authentication",\n  "rules": ["Must validate email format"],\n  "questions": ["[0] Should we support OAuth?"],\n  "linkedFeatures": [...],\n  "createdAt": "2025-10-13T14:30:00Z",\n  "updatedAt": "2025-10-13T15:45:00Z"\n}',
    },
    {
      command: 'fspec show-work-unit EPIC-001',
      description: 'Show epic details with children',
      output:
        'EPIC-001\nType: epic\nStatus: implementing\n\nUser Management\n\nChildren: AUTH-001, AUTH-002, AUTH-003\n\nCreated: 01/10/2025 10:00:00',
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-999' does not exist",
      fix: 'Verify the work unit ID exists with: fspec list-work-units',
    },
    {
      error: 'ENOENT: no such file or directory, spec/work-units.json',
      fix: 'Initialize work units with: fspec create-story PREFIX "title" (or create-bug/create-task)',
    },
  ],
  typicalWorkflow:
    '1. List work units: fspec list-work-units → 2. Show details: fspec show-work-unit <workUnitId> → 3. Update fields: fspec update-work-unit <workUnitId> --title "New title" → 4. Verify changes: fspec show-work-unit <workUnitId>',
  commonPatterns: [
    {
      pattern: 'View work unit before Example Mapping session',
      example:
        '# Check current state\nfspec show-work-unit AUTH-001\n\n# Add rules/examples/questions\nfspec add-rule AUTH-001 "Must validate email"\nfspec add-example AUTH-001 "User with valid credentials"\nfspec add-question AUTH-001 "Should we support OAuth?" --human',
    },
    {
      pattern: 'Export work unit data for processing',
      example:
        '# Export as JSON for scripts/tools\nfspec show-work-unit AUTH-001 --format json > auth-001.json\n\n# Process with jq\nfspec show-work-unit AUTH-001 --format json | jq .rules',
    },
    {
      pattern: 'Check system reminders',
      example:
        '# View work unit to see reminders\nfspec show-work-unit AUTH-001\n\n# Output includes:\n# <system-reminder>\n# Missing estimate for work unit AUTH-001\n# </system-reminder>',
    },
  ],
  relatedCommands: [
    'list-work-units',
    'update-work-unit',
    'update-work-unit-status',
    'add-rule',
    'add-example',
    'add-question',
    'dependencies',
  ],
  notes: [
    'Text format shows all Example Mapping data (rules, examples, questions, assumptions, architecture notes)',
    'Text format shows all dependency relationships (blocks, blockedBy, dependsOn, relatesTo)',
    'Text format shows linked feature files with scenario names and line numbers',
    'JSON format includes all fields, suitable for scripting and automation',
    'System reminders are displayed for missing estimates, empty Example Mapping, long phase duration, or large estimates (> 13 points for story/bug)',
    'Questions are filtered to show only unselected questions (answered questions are hidden)',
    'Linked features are automatically discovered by scanning feature files for work unit tags',
  ],
};

export default config;
