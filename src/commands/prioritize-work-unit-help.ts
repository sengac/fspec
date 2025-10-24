import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'prioritize-work-unit',
  description: 'Reorder work units in any Kanban column except done',
  usage: 'fspec prioritize-work-unit <workUnitId> [options]',
  whenToUse:
    'Use this command to reorder work units within their current column (backlog, specifying, testing, implementing, validating, blocked). Move critical work to the top, defer less important work to the bottom, or position work relative to other work units in the same column. CRITICAL: Can only reorder within the same column - cannot move work units between columns.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must NOT be in done status',
  ],
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
      description:
        'Place before this work unit (must be in same column as target)',
    },
    {
      flag: '--after <workUnitId>',
      description:
        'Place after this work unit (must be in same column as target)',
    },
  ],
  examples: [
    {
      command: 'fspec prioritize-work-unit AUTH-001 --position top',
      description:
        'Move AUTH-001 to top of its current column (e.g., backlog)',
      output: '✓ Work unit AUTH-001 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit FEAT-017 --position top',
      description:
        'Move FEAT-017 to top of specifying column (if in specifying status)',
      output: '✓ Work unit FEAT-017 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit API-002 --before AUTH-001',
      description:
        'Place API-002 before AUTH-001 (both must be in same column)',
      output: '✓ Work unit API-002 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit BUG-003 --position bottom',
      description: 'Move BUG-003 to bottom of implementing column',
      output: '✓ Work unit BUG-003 prioritized successfully',
    },
    {
      command: 'fspec prioritize-work-unit DB-001 --position 3',
      description:
        'Move DB-001 to 3rd position in its current column (1-based index)',
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
        'Error: Cannot prioritize work units in done column. Done items are ordered by completion time...',
      fix: 'Cannot reorder done items. Only backlog, specifying, testing, implementing, validating, blocked can be prioritized.',
    },
    {
      error:
        'Error: Cannot prioritize across columns. FEAT-017 (specifying) and AUTH-001 (testing) are in different columns.',
      fix: 'Work units must be in the same column. Remove --before/--after or choose work unit in same column.',
    },
    {
      error:
        'Error: Work unit UI-002 does not exist (using --before or --after)',
      fix: 'Target work unit for relative positioning does not exist. Check ID is correct.',
    },
  ],
  typicalWorkflow:
    '1. Review column: fspec list-work-units --status <column> or fspec board → 2. Identify critical work → 3. fspec prioritize-work-unit <id> --position top → 4. Verify order: fspec board',
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
    'Works for all columns EXCEPT done (backlog, specifying, testing, implementing, validating, blocked)',
    'CRITICAL: Can only reorder within same column - cannot move between columns',
    'Done items are ordered by completion time and cannot be manually reordered',
    'Numeric positions are 1-based (1 = first item in column)',
    'Relative positioning (--before/--after) is preferred for clarity',
    'Priority order affects display in board and list-work-units commands',
  ],
};

export default config;
