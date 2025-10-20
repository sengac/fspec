import type { CommandHelpConfig } from '../utils/help-formatter';

const updateWorkUnitStatusHelp: CommandHelpConfig = {
  name: 'update-work-unit-status',
  description:
    'Move a work unit through the ACDD workflow by updating its status',
  usage: 'fspec update-work-unit-status <id> <status>',
  whenToUse:
    'Use this command when moving work through ACDD workflow stages: from backlog to specifying (writing feature files), to testing (writing tests), to implementing (writing code), to validating (quality checks), and finally to done.',
  prerequisites: [
    'Work unit must exist (create with fspec create-work-unit)',
    'For transition to testing: All Example Mapping questions must be answered',
    'For transition to implementing: Tests must be written',
    'For transition to active states: All blocking dependencies must be completed (status: done)',
  ],
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID (e.g., AUTH-001)',
      required: true,
    },
    {
      name: 'status',
      description:
        'New status: backlog, specifying, testing, implementing, validating, done, blocked',
      required: true,
    },
  ],
  options: [
    {
      name: '--reason <text>',
      description:
        'Reason for status change (optional, added to state history)',
    },
    {
      name: '--blocked-reason <text>',
      description:
        'Reason for blocked status (required when status is "blocked")',
    },
    {
      name: '--skip-temporal-validation',
      description:
        'Skip temporal ordering validation (for reverse ACDD or importing existing work)',
    },
  ],
  examples: [
    {
      command: 'fspec update-work-unit-status AUTH-001 specifying',
      description:
        'Move to specifying (start Example Mapping and write feature files)',
      output: '✓ Work unit AUTH-001 status updated to specifying',
    },
    {
      command:
        'fspec update-work-unit-status AUTH-001 blocked --blocked-reason="Waiting for API design"',
      description: 'Mark work unit as blocked with reason',
      output: '✓ Work unit AUTH-001 status updated to blocked',
    },
    {
      command:
        'fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation',
      description: 'Import existing work and skip temporal validation',
      output: '✓ Work unit LEGACY-001 status updated to testing',
    },
    {
      command: 'fspec update-work-unit-status UI-001 implementing',
      description: 'Attempt to start work on blocked work unit (will fail)',
      output:
        '✗ Cannot start work on UI-001: work unit is blocked by incomplete dependencies.\n\nActive blockers:\n  - AUTH-001 (status: implementing)\n\nComplete blocking work units or remove dependencies before starting work.',
    },
  ],
  typicalWorkflow:
    'backlog → specifying (Example Mapping + feature file) → testing (write failing tests) → implementing (make tests pass) → validating (run all tests + quality checks) → done',
  relatedCommands: [
    'show-work-unit',
    'list-work-units',
    'board',
    'auto-advance',
  ],
  notes: [
    'ACDD enforces strict workflow: you cannot skip states',
    'FEAT-011: Temporal ordering is enforced - files must be created AFTER entering their required state',
    'Moving to testing: feature files must be created AFTER entering specifying state',
    'Moving to implementing: test files must be created AFTER entering testing state',
    'Use --skip-temporal-validation for reverse ACDD or importing existing work',
    'Status "blocked" can be used from any state when progress is prevented',
    'Moving to testing requires all Example Mapping questions to be answered first',
    'Cannot move to active states (specifying, testing, implementing, validating) if work unit has incomplete blocking dependencies',
    'Use "fspec remove-dependency" to remove blockers or complete blocking work units first',
    'Backward movement is allowed: can move from implementing → testing → specifying when mistakes discovered',
  ],
};

export default updateWorkUnitStatusHelp;
