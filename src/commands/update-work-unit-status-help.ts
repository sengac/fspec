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
  examples: [
    {
      command: 'fspec update-work-unit-status AUTH-001 specifying',
      description: 'Move to specifying (start Example Mapping and write feature files)',
      output: '✓ Work unit AUTH-001 status updated to specifying',
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
    'Status "blocked" can be used from any state when progress is prevented',
    'Moving to testing requires all Example Mapping questions to be answered first',
  ],
};

export default updateWorkUnitStatusHelp;
