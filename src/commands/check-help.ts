import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'check',
  description: 'Run all validation checks: Gherkin syntax, tag compliance, and formatting',
  usage: 'fspec check',
  whenToUse:
    'Use before committing or as part of CI/CD pipeline to ensure all specifications meet quality standards. Combines validate, validate-tags, and format checks.',
  examples: [
    {
      command: 'fspec check',
      description: 'Run all validation checks',
      output: '✓ Gherkin syntax validation passed\n✓ Tag validation passed\n✓ All files properly formatted\n\n✓ All checks passed',
    },
  ],
  prerequisites: [
    'Feature files must exist in spec/features/',
    'Tags must be registered in spec/tags.json',
  ],
  typicalWorkflow: 'Edit feature files → fspec check → Fix any issues → Commit',
  relatedCommands: ['validate', 'validate-tags', 'format'],
  notes: [
    'Runs validate, validate-tags, and format verification',
    'Exit code 0 = all checks pass, non-zero = failures',
    'Recommended to run before committing',
    'Can be used in pre-commit hooks',
  ],
};

export default config;
