import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'generate-coverage',
  description:
    'Generate .feature.coverage files for existing .feature files that do not have coverage tracking yet',
  usage: 'fspec generate-coverage [options]',
  whenToUse:
    'Use this command when transitioning an existing fspec project to coverage tracking, or when .feature.coverage files were accidentally deleted. Creates empty coverage structures for all unmapped features. Essential for reverse ACDD setup.',
  whenNotToUse:
    'Do not use if .feature.coverage files already exist (they will be skipped). Do not use for creating new features (use fspec create-feature instead, which auto-creates coverage files).',
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
        'Generate coverage files for all .feature files without existing coverage',
      output:
        '✓ Generated coverage for: user-authentication.feature\n✓ Generated coverage for: user-registration.feature\n✓ Skipped (already exists): gherkin-validation.feature\n\nCreated 2 coverage files',
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
    'This command only creates coverage files, it does not link tests or implementation',
    'Existing .feature.coverage files are skipped to prevent data loss',
    'Coverage files contain empty testMappings arrays until you run fspec link-coverage',
    'Use --dry-run to preview before creating files',
  ],
  prerequisites: [
    'Valid .feature files must exist in spec/features/',
    'Feature files must have valid Gherkin syntax (run fspec validate first)',
  ],
};

export default config;
