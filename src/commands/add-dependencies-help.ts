import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-dependencies',
  description: 'Add multiple dependency relationships to a work unit at once',
  usage: 'fspec add-dependencies <workUnitId> [options]',
  whenToUse:
    'Use this command to add multiple dependency relationships to a work unit in a single command, instead of calling add-dependency multiple times. Efficient for setting up complex dependency graphs.',
  prerequisites: ['Work unit must exist in spec/work-units.json', 'Target work units must exist'],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to add dependencies to (e.g., AUTH-001)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--blocks <ids...>',
      description: 'Space-separated list of work unit IDs that this work unit blocks',
    },
    {
      flag: '--blocked-by <ids...>',
      description: 'Space-separated list of work unit IDs that block this work unit',
    },
    {
      flag: '--depends-on <ids...>',
      description: 'Space-separated list of work unit IDs this work unit depends on',
    },
    {
      flag: '--relates-to <ids...>',
      description: 'Space-separated list of related work unit IDs (non-blocking association)',
    },
  ],
  examples: [
    {
      command: 'fspec add-dependencies AUTH-001 --depends-on AUTH-002 AUTH-003 --relates-to UI-001',
      description: 'Add multiple dependencies and relations',
      output: '✓ Added 3 dependencies successfully',
    },
    {
      command: 'fspec add-dependencies API-001 --blocked-by DB-001 DB-002',
      description: 'Mark work unit as blocked by multiple dependencies',
      output: '✓ Added 2 dependencies successfully',
    },
    {
      command: 'fspec add-dependencies UI-001 --blocks UI-002 UI-003 UI-004',
      description: 'Mark work unit as blocking multiple other units',
      output: '✓ Added 3 dependencies successfully',
    },
    {
      command: 'fspec add-dependencies REFAC-001 --relates-to AUTH-001 API-001 DB-001',
      description: 'Add related work units (non-blocking relationships)',
      output: '✓ Added 3 dependencies successfully',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Work unit AUTH-001 does not exist',
      fix: 'Ensure work unit exists. Run: fspec list-work-units',
    },
    {
      error: 'Error: Target work unit DB-001 does not exist',
      fix: 'All target work units must exist before adding dependencies',
    },
  ],
  typicalWorkflow:
    '1. Create work units → 2. Identify dependencies → 3. fspec add-dependencies <id> --depends-on <deps> → 4. Verify: fspec dependencies <id>',
  commonPatterns: [
    {
      pattern: 'Setting Up Dependency Chain',
      example:
        '# API depends on DB schema and auth\nfspec add-dependencies API-001 --depends-on DB-001 AUTH-001\n\n# UI depends on API\nfspec add-dependencies UI-001 --depends-on API-001',
    },
    {
      pattern: 'Marking Blockers',
      example:
        '# Critical bug blocks multiple features\nfspec add-dependencies BUG-001 --blocks FEAT-001 FEAT-002 FEAT-003',
    },
    {
      pattern: 'Related Work (Non-Blocking)',
      example:
        '# Documentation relates to multiple features\nfspec add-dependencies DOC-001 --relates-to AUTH-001 API-001 UI-001',
    },
  ],
  relatedCommands: [
    'add-dependency',
    'remove-dependency',
    'dependencies',
    'export-dependencies',
    'suggest-dependencies',
  ],
  notes: [
    'All options accept space-separated lists of work unit IDs',
    'Each relationship is validated before being added',
    'Automatically creates bidirectional relationships where appropriate',
    'More efficient than multiple add-dependency calls',
    'Use --relates-to for associations that do not imply blocking or dependency',
  ],
};

export default config;
