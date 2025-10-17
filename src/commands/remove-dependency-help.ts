import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-dependency',
  description:
    'Remove dependency relationships between work units to clean up outdated blockers and dependencies',
  usage: 'fspec remove-dependency <workUnitId> [dependsOnId] [options]',
  whenToUse:
    'Use this command when a dependency relationship is no longer valid (task completed, blocker removed, or relationship was added by mistake). This is the inverse of add-dependency and supports bidirectional cleanup for blocks/blocked-by and relates-to relationships.',
  prerequisites: [
    'Work units must exist in spec/work-units.json',
    'At least one dependency relationship must exist to remove',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to remove dependency from',
      required: true,
    },
    {
      name: 'dependsOnId',
      description:
        'Work unit ID to remove from dependsOn (shorthand for --depends-on)',
      required: false,
    },
  ],
  options: [
    {
      flag: '--blocks <targetId>',
      description:
        'Remove blocks relationship (also removes reverse blockedBy from target)',
    },
    {
      flag: '--blocked-by <targetId>',
      description:
        'Remove blockedBy relationship (also removes reverse blocks from target)',
    },
    {
      flag: '--depends-on <targetId>',
      description: 'Remove dependsOn relationship (unidirectional)',
    },
    {
      flag: '--relates-to <targetId>',
      description:
        'Remove relatesTo relationship (also removes reverse relatesTo from target)',
    },
  ],
  examples: [
    {
      command: 'fspec remove-dependency AUTH-002 AUTH-001',
      description: 'Shorthand: Remove AUTH-001 from AUTH-002 dependsOn list',
      output: '✓ Dependency removed successfully',
    },
    {
      command: 'fspec remove-dependency AUTH-002 --blocks API-001',
      description:
        'Remove blocks relationship (AUTH-002 no longer blocks API-001)',
      output: '✓ Dependency removed successfully',
    },
    {
      command: 'fspec remove-dependency UI-001 --blocked-by API-001',
      description: 'Remove blockedBy relationship (UI-001 no longer blocked)',
      output: '✓ Dependency removed successfully',
    },
    {
      command: 'fspec remove-dependency DASH-001 --depends-on AUTH-001',
      description: 'Explicit: Remove AUTH-001 from DASH-001 dependsOn list',
      output: '✓ Dependency removed successfully',
    },
    {
      command: 'fspec remove-dependency UI-005 --relates-to UI-004',
      description: 'Remove relatesTo relationship (bidirectional cleanup)',
      output: '✓ Dependency removed successfully',
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-999' does not exist",
      fix: 'Verify the work unit ID exists with: fspec list-work-units',
    },
    {
      error:
        'Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to',
      fix: 'Provide either a second argument (dependsOnId) or one of the relationship flags',
    },
    {
      error:
        'Cannot specify dependency both as argument and --depends-on option',
      fix: 'Use either shorthand (two arguments) OR --depends-on flag, not both',
    },
  ],
  typicalWorkflow:
    '1. Verify dependency exists: fspec dependencies <workUnitId> → 2. Remove dependency: fspec remove-dependency <workUnitId> --blocks <targetId> → 3. Verify removal: fspec dependencies <workUnitId>',
  commonPatterns: [
    {
      pattern: 'Clean up completed blocker',
      example:
        '# After AUTH-001 is done, unblock dependent tasks\nfspec remove-dependency AUTH-001 --blocks API-001\nfspec remove-dependency AUTH-001 --blocks UI-001',
    },
    {
      pattern: 'Remove all dependencies for a work unit',
      example:
        '# List current dependencies\nfspec dependencies AUTH-002\n\n# Remove each dependency\nfspec remove-dependency AUTH-002 AUTH-001\nfspec remove-dependency AUTH-002 --relates-to UI-001',
    },
  ],
  relatedCommands: [
    'add-dependency',
    'dependencies',
    'clear-dependencies',
    'export-dependencies',
  ],
  notes: [
    'Use shorthand syntax (two arguments) for simple dependsOn removals',
    'Blocks/blocked-by and relates-to relationships are bidirectional - removing from one side removes from both',
    'DependsOn relationships are unidirectional - only removed from the specified work unit',
    'The command will succeed even if the relationship does not exist (idempotent)',
  ],
};

export default config;
