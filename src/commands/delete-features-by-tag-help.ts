import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'delete-features',
  description: 'Bulk delete feature files by tag using AND logic (all tags must match)',
  usage: 'fspec delete-features --tag <tag> [--tag <tag>] [options]',
  whenToUse:
    'Use this command to bulk delete feature files by tag (e.g., removing all @deprecated, @phase1, or @spike features). Essential for cleanup after major refactoring or phase completion. ALWAYS use --dry-run first to preview deletions.',
  prerequisites: [
    'spec/features/ directory exists with feature files',
    'Feature files have tags at feature level',
  ],
  arguments: [],
  options: [
    {
      flag: '--tag <tag>',
      description:
        'Filter by tag (can specify multiple times for AND logic). Example: @phase1, @deprecated',
    },
    {
      flag: '--dry-run',
      description: 'Preview deletions without making changes (ALWAYS use first)',
    },
  ],
  examples: [
    {
      command: 'fspec delete-features --tag @deprecated --dry-run',
      description: 'Preview deletion of deprecated features (safe preview)',
      output:
        'Dry run mode - no files modified\n\nWould delete 3 feature file(s):\n\n  - spec/features/legacy-auth.feature\n  - spec/features/old-api.feature\n  - spec/features/prototype-ui.feature',
    },
    {
      command: 'fspec delete-features --tag @deprecated',
      description: 'Delete all features tagged @deprecated',
      output:
        '✓ Deleted 3 feature file(s)\n\nDeleted files:\n  - spec/features/legacy-auth.feature\n  - spec/features/old-api.feature\n  - spec/features/prototype-ui.feature',
    },
    {
      command: 'fspec delete-features --tag @phase1 --tag @spike --dry-run',
      description: 'Preview deletion with AND logic (both tags required)',
      output:
        'Dry run mode - no files modified\n\nWould delete 2 feature file(s):\n\n  - spec/features/phase1-exploration.feature\n  - spec/features/phase1-prototype.feature',
    },
    {
      command: 'fspec delete-features --tag @phase1 --tag @complete',
      description: 'Delete completed phase1 features',
      output: '✓ Deleted 5 feature file(s)\n\nDeleted files:\n  - spec/features/phase1-auth.feature\n  ...',
    },
  ],
  commonErrors: [
    {
      error: 'Error: At least one --tag is required',
      fix: 'Provide at least one --tag option: fspec delete-features --tag @deprecated',
    },
    {
      error: 'Error: No feature files found',
      fix: 'Ensure spec/features/ directory exists with .feature files',
    },
    {
      error: 'Error: No feature files found matching tags',
      fix: 'No features have all specified tags (AND logic). Check tag names with: fspec list-features --tag @yourtag',
    },
  ],
  typicalWorkflow:
    '1. List features to delete: fspec list-features --tag @deprecated → 2. Preview deletion: fspec delete-features --tag @deprecated --dry-run → 3. Review output carefully → 4. Execute deletion: fspec delete-features --tag @deprecated → 5. Commit changes: git add . && git commit -m "Remove deprecated features"',
  commonPatterns: [
    {
      pattern: 'Phase Cleanup',
      example:
        '# After completing a phase, remove phase1 features\nfspec list-features --tag @phase1\nfspec delete-features --tag @phase1 --tag @complete --dry-run\nfspec delete-features --tag @phase1 --tag @complete\n\n# Commit cleanup\ngit add . && git commit -m "Remove completed phase1 features"',
    },
    {
      pattern: 'Deprecated Feature Removal',
      example:
        '# Mark features as deprecated first\nfspec add-tag-to-feature old-api @deprecated\n\n# Preview deletion\nfspec delete-features --tag @deprecated --dry-run\n\n# Delete deprecated features\nfspec delete-features --tag @deprecated\n\n# Verify cleanup\nfspec list-features --tag @deprecated  # Should return empty',
    },
    {
      pattern: 'Spike/Prototype Cleanup',
      example:
        '# Remove experimental spike features\nfspec delete-features --tag @spike --dry-run\nfspec delete-features --tag @spike\n\n# Remove prototype features\nfspec delete-features --tag @prototype --dry-run\nfspec delete-features --tag @prototype',
    },
  ],
  relatedCommands: [
    'delete-scenarios',
    'list-features',
    'add-tag-to-feature',
    'remove-tag-from-feature',
    'list-tags',
  ],
  notes: [
    'ALWAYS use --dry-run first to preview deletions',
    'Uses AND logic: ALL specified tags must be present on feature',
    'Only deletes features with tags at FEATURE level (not scenario tags)',
    'Files are permanently deleted (ensure version control is up to date)',
    'Feature files with invalid Gherkin syntax are skipped',
    'No undo operation - commit work before deleting',
    'Coverage files (.feature.coverage) are NOT automatically deleted',
    'Consider archiving important features instead of deleting',
  ],
};

export default config;
