import type { CommandHelpConfig } from '../utils/help-formatter';

const addDependencyHelp: CommandHelpConfig = {
  name: 'add-dependency',
  description:
    'Add dependency relationships between work units to track blockers and dependencies',
  usage: 'fspec add-dependency <id> [dependsOnId] [options]',
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID to add dependency to',
      required: true,
    },
    {
      name: 'dependsOnId',
      description:
        'Work unit ID that this depends on (shorthand for --depends-on)',
      required: false,
    },
  ],
  options: [
    {
      flag: '--blocks <id>',
      description: 'This work unit blocks the specified work unit',
    },
    {
      flag: '--blocked-by <id>',
      description: 'This work unit is blocked by the specified work unit',
    },
    {
      flag: '--depends-on <id>',
      description: 'This work unit depends on the specified work unit',
    },
    {
      flag: '--relates-to <id>',
      description:
        'This work unit is related to the specified work unit (no blocking)',
    },
  ],
  examples: [
    {
      command: 'fspec add-dependency AUTH-002 AUTH-001',
      description: 'Shorthand: AUTH-002 depends on AUTH-001',
      output: '✓ Added dependency: AUTH-002 depends on AUTH-001',
    },
    {
      command: 'fspec add-dependency AUTH-002 --blocks API-001',
      description: 'AUTH-002 blocks API-001 from starting',
      output: '✓ Added dependency: AUTH-002 blocks API-001',
    },
    {
      command: 'fspec add-dependency UI-001 --blocked-by API-001',
      description: 'UI-001 is blocked by API-001',
      output: '✓ Added dependency: UI-001 blocked by API-001',
    },
    {
      command: 'fspec add-dependency DASH-001 --depends-on AUTH-001',
      description: 'Explicit: DASH-001 depends on AUTH-001',
      output: '✓ Added dependency: DASH-001 depends on AUTH-001',
    },
    {
      command: 'fspec add-dependency UI-005 --relates-to UI-004',
      description: 'UI-005 is related to UI-004 (no blocking)',
      output: '✓ Added dependency: UI-005 relates to UI-004',
    },
  ],
  commonErrors: [
    {
      error: 'Work unit AUTH-999 not found',
      fix: 'Verify the work unit ID exists with: fspec list-work-units',
    },
    {
      error: 'Circular dependency detected',
      fix: 'Remove the circular dependency chain before adding this relationship',
    },
  ],
  relatedCommands: [
    'remove-dependency',
    'dependencies',
    'export-dependencies',
    'clear-dependencies',
  ],
  notes: [
    'Use shorthand syntax (two arguments) for simple depends-on relationships',
    'Use explicit flags (--blocks, --depends-on) for clarity in complex dependency graphs',
    'Circular dependencies are not allowed and will be rejected',
  ],
};

export default addDependencyHelp;
