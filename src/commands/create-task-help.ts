import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-task',
  description:
    'Create a new task with minimal requirements for operational work (setup, config, infrastructure)',
  usage: 'fspec create-task <prefix> <title> [options]',
  whenToUse:
    'Use when tracking operational work that does not require user-facing features, tests, or acceptance criteria (e.g., setup CI/CD, update dependencies, refactor code).',
  arguments: [
    {
      name: 'prefix',
      description:
        'Task prefix (e.g., TASK, INFRA, DEVOPS). Must be registered with create-prefix first.',
      required: true,
    },
    {
      name: 'title',
      description: 'Brief description of the task',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <description>',
      description: 'Detailed description of the task',
    },
    {
      flag: '-e, --epic <epic>',
      description: 'Epic ID to associate with this task',
    },
    {
      flag: '-p, --parent <parent>',
      description: 'Parent task ID for hierarchical relationships',
    },
  ],
  examples: [
    {
      command: 'fspec create-task TASK "Setup CI/CD pipeline"',
      description: 'Create simple task with minimal requirements',
      output:
        '✓ Created task TASK-001\n  Title: Setup CI/CD pipeline\n\n<system-reminder>\nTask TASK-001 created successfully.\n\nTasks are for operational work.\n  - Tasks have optional feature file\n  - Tasks have optional tests\n  - Tasks can skip Example Mapping\n</system-reminder>',
    },
    {
      command:
        'fspec create-task INFRA "Configure monitoring" --epic=infrastructure',
      description: 'Create task with epic',
      output:
        '✓ Created task INFRA-001\n  Title: Configure monitoring\n  Epic: infrastructure',
    },
    {
      command:
        'fspec create-task DEVOPS "Update dependencies" --description="Security patches"',
      description: 'Create task with description',
      output:
        '✓ Created task DEVOPS-001\n  Title: Update dependencies\n  Description: Security patches',
    },
  ],
  prerequisites: [
    'Prefix must be registered: fspec create-prefix PREFIX "Description"',
    'Epic must exist if using --epic: fspec create-epic EPIC "Title"',
    'Parent task must exist if using --parent',
  ],
  typicalWorkflow: [
    'Create task: fspec create-task PREFIX "Title"',
    'Tasks can skip specifying phase (optional feature file)',
    'Move directly to implementing: fspec update-work-unit-status TASK-001 implementing',
    'Complete work without tests (for operational tasks)',
    'Move to done: fspec update-work-unit-status TASK-001 done',
  ],
  commonErrors: [
    {
      error: 'Prefix \'PREFIX\' is not registered',
      solution: 'Run: fspec create-prefix PREFIX "Description"',
    },
    {
      error: 'Parent task \'PARENT-001\' does not exist',
      solution: 'Create parent first or remove --parent option',
    },
    {
      error: 'Epic \'epic-name\' does not exist',
      solution: 'Run: fspec create-epic epic-name "Title"',
    },
  ],
  relatedCommands: [
    'fspec update-work-unit-status - Move task through workflow',
    'fspec list-work-units - List all tasks',
    'fspec show-work-unit - Show task details',
    'fspec update-work-unit - Update task title/description',
  ],
  notes: [
    'Tasks have optional feature files (not required for operational work)',
    'Tasks have optional tests (not required for infrastructure work)',
    'Tasks can skip Example Mapping (no need for acceptance criteria)',
    'Tasks can move directly to implementing without specifying phase',
    'Examples: Setup CI/CD, configure monitoring, update dependencies, refactor code, write docs',
  ],
};

export default config;
