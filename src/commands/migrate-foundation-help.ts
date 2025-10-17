import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'migrate-foundation',
  description: 'Migrate legacy foundation.json (v1.x) to generic v2.0.0 schema format',
  usage: 'fspec migrate-foundation [options]',
  whenToUse:
    'Use this command when upgrading existing projects with v1.x foundation.json files to the new v2.0.0 generic foundation schema. Essential for projects created before the v2.0.0 schema release to maintain compatibility.',
  prerequisites: [
    'spec/foundation.json exists with legacy v1.x format',
    'Project is under version control (migration creates backup)',
  ],
  arguments: [],
  options: [
    {
      flag: '--output <path>',
      description: 'Output path for migrated foundation.json',
      defaultValue: 'spec/foundation.json',
    },
    {
      flag: '--backup',
      description: 'Create backup of original file before migration',
    },
    {
      flag: '--dry-run',
      description: 'Preview migration without writing files',
    },
  ],
  examples: [
    {
      command: 'fspec migrate-foundation',
      description: 'Migrate foundation.json to v2.0.0 format',
      output:
        '✓ Migrated foundation.json to v2.0.0 schema\n  Backup: spec/foundation.json.backup\n  Updated: spec/foundation.json\n  Version: 2.0.0',
    },
    {
      command: 'fspec migrate-foundation --dry-run',
      description: 'Preview migration changes without modifying files',
      output:
        'Dry run mode - no files modified\n\nMigration Preview:\n  version: "2.0.0"\n  project.name: "fspec"\n  project.vision: "CLI tool for managing Gherkin specifications"\n  project.projectType: "cli-tool"',
    },
    {
      command: 'fspec migrate-foundation --output foundation-v2.json',
      description: 'Migrate to custom output path',
      output:
        '✓ Migrated foundation.json to v2.0.0 schema\n  Output: foundation-v2.json\n  Original: spec/foundation.json (unchanged)',
    },
  ],
  commonErrors: [
    {
      error: 'Error: foundation.json not found',
      fix: 'Ensure spec/foundation.json exists. Run: fspec init or fspec discover-foundation',
    },
    {
      error: 'Error: foundation.json is already v2.0.0 format',
      fix: 'File is already migrated. No action needed. Check version field in foundation.json.',
    },
    {
      error: 'Error: Invalid legacy foundation.json format',
      fix: 'Legacy file must have valid v1.x structure. Check for JSON syntax errors.',
    },
    {
      error: 'Error: Migration validation failed',
      fix: 'Migrated JSON does not pass v2.0.0 schema validation. Check error details for specific issue.',
    },
  ],
  typicalWorkflow:
    '1. Backup project (git commit) → 2. Run migration: fspec migrate-foundation --dry-run → 3. Review changes → 4. Migrate: fspec migrate-foundation → 5. Validate: fspec validate-foundation-schema → 6. Regenerate docs: fspec generate-foundation-md',
  commonPatterns: [
    {
      pattern: 'Safe Migration Workflow',
      example:
        '# Commit current state\ngit add . && git commit -m "Pre-migration snapshot"\n\n# Preview migration\nfspec migrate-foundation --dry-run\n\n# Perform migration with backup\nfspec migrate-foundation --backup\n\n# Validate migrated schema\nfspec validate-foundation-schema\n\n# Regenerate documentation\nfspec generate-foundation-md',
    },
    {
      pattern: 'Side-by-Side Comparison',
      example:
        '# Migrate to separate file for comparison\nfspec migrate-foundation --output foundation-v2.json\n\n# Compare old vs new\ndiff spec/foundation.json foundation-v2.json\n\n# Replace when satisfied\nmv foundation-v2.json spec/foundation.json',
    },
  ],
  relatedCommands: [
    'validate-foundation-schema',
    'generate-foundation-md',
    'discover-foundation',
    'show-foundation',
    'update-foundation',
  ],
  notes: [
    'Migration maps v1.x fields to v2.0.0 generic schema (WHY/WHAT focus)',
    'HOW implementation details are extracted to documentation during migration',
    'Original file is backed up if --backup flag is used',
    'v2.0.0 schema is foundation-agnostic (no fspec-specific fields)',
    'projectType defaults to "other" if not inferrable from legacy data',
    'Architecture diagrams are preserved as they are compatible with v2.0.0',
    'After migration, run fspec generate-foundation-md to create FOUNDATION.md',
  ],
};

export default config;
