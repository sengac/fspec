import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-coverage',
  description:
    'Generate or update .feature.coverage files for existing .feature files. Creates new coverage files or updates existing ones with missing scenarios.',
  usage: 'fspec generate-coverage [options]',
  whenToUse:
    'Use this command when: 1) Setting up coverage tracking for existing features, 2) You added new scenarios to existing .feature files and need to update .coverage files, 3) .feature.coverage files were accidentally deleted. Essential for reverse ACDD setup and maintaining coverage sync.',
  whenNotToUse:
    'Do not use for creating new features (use fspec create-feature instead, which auto-creates coverage files). This command is safe to run anytime - it preserves existing test mappings.',
  options: [
    {
      flag: '--dry-run',
      description:
        'Preview what coverage files would be created without actually creating them',
    },
  ],
  examples: [
    {
      command: 'fspec generate-coverage',
      description:
        'Generate or update coverage files for all .feature files',
      output:
        '✓ Created 2, Updated 1, Skipped 3\n\nCreated: user-authentication.feature.coverage, user-registration.feature.coverage\nUpdated: existing-feature.feature.coverage (added 2 missing scenarios)\nSkipped: already-up-to-date.feature.coverage',
    },
    {
      command: 'fspec generate-coverage --dry-run',
      description: 'Preview what would be created without creating files',
      output:
        '[DRY RUN] Would generate coverage for:\n  - user-authentication.feature (3 scenarios)\n  - user-registration.feature (2 scenarios)\n\nWould create 2 coverage files',
    },
  ],
  commonErrors: [
    {
      error: 'Error: No feature files found in spec/features/',
      fix: 'Ensure you have .feature files in spec/features/ directory. Create features with: fspec create-feature',
    },
    {
      error: 'Error: Invalid Gherkin syntax in feature file',
      fix: 'Fix Gherkin syntax errors first. Run: fspec validate to identify issues.',
    },
  ],
  typicalWorkflow:
    '1. Run fspec generate-coverage --dry-run to preview → 2. Review what will be created → 3. Run fspec generate-coverage to create files → 4. Verify with fspec show-coverage',
  commonPatterns: [
    'Reverse ACDD Setup: Run generate-coverage to create empty coverage structures, then use link-coverage to map scenarios to tests',
    'Post-Recovery: After accidentally deleting coverage files, run generate-coverage to recreate empty structures',
    'Project Migration: When adding coverage tracking to existing fspec project, use --dry-run first to preview',
  ],
  relatedCommands: [
    'create-feature',
    'link-coverage',
    'show-coverage',
    'audit-coverage',
  ],
  notes: [
    'Coverage files are automatically created by fspec create-feature for new features',
    'This command creates new coverage files AND updates existing ones with missing scenarios',
    'Existing test mappings are preserved when updating coverage files',
    'New scenarios are added with empty testMappings arrays',
    'Safe to run anytime - idempotent and preserves existing data',
    'Use --dry-run to preview before making changes',
    'Returns status: created, updated, skipped, or recreated (if invalid JSON)',
  ],
  prerequisites: [
    'Valid .feature files must exist in spec/features/',
    'Feature files must have valid Gherkin syntax (run fspec validate first)',
  ],
};

export default config;
