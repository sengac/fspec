import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'prioritize-work-unit',
  description: 'Change the priority order of a work unit in the backlog',
  usage: 'fspec prioritize-work-unit <workUnitId> [options]',
  whenToUse:
    'Use this command to reorder work units in the backlog to reflect changing priorities. Move critical work to the top, defer less important work to the bottom, or position work relative to other work units.',
  prerequisites: ['Work unit must exist and be in backlog status'],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to prioritize (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--position <position>',
      description:
        'Absolute position: "top" (highest priority), "bottom" (lowest priority), or numeric index (1-based)',
    },
    {
      flag: '--before <workUnitId>',
      description: 'Place before this work unit in the backlog',
    },
    {
      flag: '--after <workUnitId>',
      description: 'Place after this work unit in the backlog',
    },
  ],
  examples: [
    {
      command: 'fspec prioritize-work-unit AUTH-001 --position top',
      description: 'Move AUTH-001 to highest priority (top of backlog)',
      output: '✓ Work unit AUTH-001 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit PERF-003 --position bottom',
      description: 'Move PERF-003 to lowest priority (bottom of backlog)',
      output: '✓ Work unit PERF-003 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit API-002 --before AUTH-001',
      description: 'Place API-002 immediately before AUTH-001',
      output: '✓ Work unit API-002 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit UI-005 --after AUTH-001',
      description: 'Place UI-005 immediately after AUTH-001',
      output: '✓ Work unit UI-005 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit DB-001 --position 3',
      description: 'Move DB-001 to 3rd position in backlog (1-based index)',
      output: '✓ Work unit DB-001 prioritized successfully',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Work unit AUTH-001 does not exist',
      fix: 'Check work unit ID is correct. Run: fspec list-work-units',
    },
    {
      error:
        'Error: Can only prioritize work units in backlog state. AUTH-001 is in "implementing" state.',
      fix: 'Only backlog items can be reprioritized. Work units in other states have their order determined by workflow.',
    },
    {
      error:
        'Error: Work unit UI-002 does not exist (using --before or --after)',
      fix: 'Target work unit for relative positioning does not exist. Check ID is correct.',
    },
  ],
  typicalWorkflow:
    '1. Review backlog: fspec list-work-units --status backlog → 2. Identify critical work → 3. fspec prioritize-work-unit <id> --position top → 4. Verify order: fspec board',
  commonPatterns: [
    {
      pattern: 'Sprint Planning - Prioritize Critical Items',
      example:
        '# Move critical auth work to top\nfspec prioritize-work-unit AUTH-001 --position top\nfspec prioritize-work-unit AUTH-002 --after AUTH-001\n\n# Defer non-critical work\nfspec prioritize-work-unit REFAC-005 --position bottom',
    },
    {
      pattern: 'Dependency Ordering',
      example:
        '# DB schema must come before API endpoints\nfspec prioritize-work-unit DB-001 --position top\nfspec prioritize-work-unit API-001 --after DB-001\nfspec prioritize-work-unit API-002 --after API-001',
    },
  ],
  relatedCommands: [
    'list-work-units',
    'board',
    'create-work-unit',
    'update-work-unit-status',
  ],
  notes: [
    'Only works for work units in backlog status',
    'Numeric positions are 1-based (1 = first item in backlog)',
    'Relative positioning (--before/--after) is preferred for clarity',
    'Priority order affects display in board and list-work-units commands',
    'Work units in other states (specifying, testing, etc.) maintain workflow order',
  ],
};

export default config;
