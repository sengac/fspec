import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate',
  description:
    'Validate Gherkin syntax in feature files using @cucumber/gherkin parser',
  usage: 'fspec validate [file] [options]',
  whenToUse:
    'Use this command to verify that your feature files have correct Gherkin syntax before committing. Essential step in ACDD workflow before moving from specifying to testing phase.',
  arguments: [
    {
      name: 'file',
      description:
        'Specific feature file to validate (e.g., spec/features/login.feature). If omitted, validates all .feature files in spec/features/',
      required: false,
    },
  ],
  options: [
    {
      flag: '-v, --verbose',
      description:
        'Show detailed validation output including line numbers and suggestions',
    },
  ],
  examples: [
    {
      command: 'fspec validate',
      description: 'Validate all feature files',
      output:
        '✓ spec/features/login.feature is valid\n✓ spec/features/signup.feature is valid\n\nValidated 2 files: 2 valid, 0 invalid',
    },
    {
      command: 'fspec validate spec/features/login.feature',
      description: 'Validate specific feature file',
      output: '✓ spec/features/login.feature is valid',
    },
    {
      command: 'fspec validate --verbose',
      description: 'Validate with detailed output',
      output:
        '✓ spec/features/login.feature is valid\n  - 3 scenarios, 12 steps\n  - Tags: @phase1, @authentication',
    },
  ],
  commonErrors: [
    {
      error: "expected: #EOF, #StepLine, got 'And user should see dashboard'",
      fix: 'Check indentation - steps must be indented (usually 4 spaces)',
    },
    {
      error: "expected: #Feature, got 'Scenario'",
      fix: 'Feature keyword must come before Scenario. Add Feature: line at top.',
    },
  ],
  typicalWorkflow:
    'Write feature file → fspec validate → Fix errors → Repeat until valid → Move to testing phase',
  relatedCommands: ['format', 'create-feature', 'check', 'validate-tags'],
  notes: [
    'Uses official @cucumber/gherkin parser for validation',
    'Exit code 0 = all valid, non-zero = errors found',
    'Run before committing feature files',
    'Part of fspec check command',
  ],
};

export default config;
