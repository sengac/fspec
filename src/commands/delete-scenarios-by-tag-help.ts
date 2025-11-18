import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-scenarios',
  description:
    'Bulk delete scenarios by tag across multiple feature files using AND logic (all tags must match)',
  usage: 'fspec delete-scenarios --tag <tag> [--tag <tag>] [options]',
  whenToUse:
    'Use this command to bulk delete scenarios by tag (e.g., removing @deprecated, @spike, @todo scenarios) across multiple feature files. Essential for cleanup after prototyping or removing obsolete scenarios. ALWAYS use --dry-run first to preview deletions.',
  prerequisites: [
    'spec/features/ directory exists with feature files',
    'Scenarios have tags at scenario level',
  ],
  arguments: [],
  options: [
    {
      flag: '--tag <tag>',
      description:
        'Filter by tag (can specify multiple times for AND logic). Example: @spike, @deprecated',
    },
    {
      flag: '--dry-run',
      description:
        'Preview deletions without making changes (ALWAYS use first)',
    },
  ],
  examples: [
    {
      command: 'fspec delete-scenarios --tag @spike --dry-run',
      description: 'Preview deletion of spike scenarios (safe preview)',
      output:
        'Dry run mode - no files modified\n\nWould delete 4 scenario(s) from 2 file(s):\n\nspec/features/authentication.feature:\n  - Prototype JWT flow (@spike)\n  - Test OAuth integration (@spike)\n\nspec/features/user-management.feature:\n  - Experiment with user roles (@spike)\n  - Explore admin permissions (@spike)',
    },
    {
      command: 'fspec delete-scenarios --tag @spike',
      description: 'Delete all scenarios tagged @spike',
      output:
        '✓ Deleted 4 scenario(s) from 2 file(s). All modified files validated successfully.',
    },
    {
      command:
        'fspec delete-scenarios --tag @deprecated --tag @critical --dry-run',
      description: 'Preview deletion with AND logic (both tags required)',
      output:
        'Dry run mode - no files modified\n\nWould delete 2 scenario(s) from 1 file(s):\n\nspec/features/legacy-api.feature:\n  - Old login flow (@deprecated @critical)\n  - Legacy token refresh (@deprecated @critical)',
    },
    {
      command: 'fspec delete-scenarios --tag @todo',
      description: 'Delete unimplemented placeholder scenarios',
      output:
        '✓ Deleted 8 scenario(s) from 5 file(s). All modified files validated successfully.',
    },
  ],
  commonErrors: [
    {
      error: 'Error: At least one --tag is required',
      fix: 'Provide at least one --tag option: fspec delete-scenarios --tag @spike',
    },
    {
      error: 'Error: No scenarios found matching tags',
      fix: 'No scenarios have all specified tags (AND logic). Check scenario tags with: fspec list-features',
    },
    {
      error: 'Error: Validation failed after deleting scenarios from <file>',
      fix: 'Scenario deletion resulted in invalid Gherkin. Feature file not modified. Manually review file structure.',
    },
  ],
  typicalWorkflow:
    '1. Preview deletion: fspec delete-scenarios --tag @spike --dry-run → 2. Review scenarios to be deleted → 3. Execute deletion: fspec delete-scenarios --tag @spike → 4. Validate files: fspec validate → 5. Commit changes: git add . && git commit -m "Remove spike scenarios"',
  commonPatterns: [
    {
      pattern: 'Prototype Cleanup',
      example:
        '# After prototyping phase, remove spike scenarios\nfspec delete-scenarios --tag @spike --dry-run\nfspec delete-scenarios --tag @spike\n\n# Validate feature files\nfspec validate\n\n# Commit cleanup\ngit add . && git commit -m "Remove spike scenarios from prototyping"',
    },
    {
      pattern: 'Deprecated Scenario Removal',
      example:
        '# Remove deprecated scenarios (marked with @deprecated tag)\nfspec delete-scenarios --tag @deprecated --dry-run\nfspec delete-scenarios --tag @deprecated\n\n# Verify no deprecated scenarios remain\nfspec list-features | grep @deprecated  # Should be empty',
    },
    {
      pattern: 'Placeholder Cleanup',
      example:
        '# Remove TODO/placeholder scenarios\nfspec delete-scenarios --tag @todo --dry-run\nfspec delete-scenarios --tag @todo\n\n# Or remove unimplemented scenarios\nfspec delete-scenarios --tag @unimplemented --dry-run\nfspec delete-scenarios --tag @unimplemented',
    },
    {
      pattern: 'Phase-Specific Cleanup',
      example:
        '# Remove completed phase1 spike scenarios\nfspec delete-scenarios --tag @critical --tag @spike --dry-run\nfspec delete-scenarios --tag @critical --tag @spike\n\n# Commit changes\ngit add . && git commit -m "Remove phase1 spike scenarios"',
    },
  ],
  relatedCommands: [
    'delete-features',
    'list-features',
    'add-scenario',
    'remove-scenario',
    'validate',
    'list-tags',
  ],
  notes: [
    'ALWAYS use --dry-run first to preview deletions',
    'Uses AND logic: ALL specified tags must be present on scenario',
    'Only deletes scenarios with tags at SCENARIO level (not feature tags)',
    'Scenarios are deleted from feature files (files remain, scenarios removed)',
    'Modified files are validated after deletion to ensure valid Gherkin',
    'If validation fails, file is NOT modified (rollback)',
    'Excessive blank lines (4+) are cleaned up after deletion',
    'No undo operation - commit work before deleting',
    'Coverage mappings in .feature.coverage files are automatically updated',
    'Deleted scenarios are removed from coverage files and statistics recalculated',
    'Consider using @deprecated tag for soft deletion instead of immediate removal',
  ],
};

export default config;
