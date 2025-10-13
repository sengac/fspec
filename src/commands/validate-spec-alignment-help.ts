import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'validate-spec-alignment',
  description: 'Validate alignment between specs, tests, and implementation',
  usage: 'fspec validate-spec-alignment [options]',
  whenToUse:
    'Use in validating phase to ensure specifications, tests, and implementation are aligned.',
  options: [
    {
      flag: '--work-unit <id>',
      description: 'Validate specific work unit',
    },
  ],
  examples: [
    {
      command: 'fspec validate-spec-alignment',
      description: 'Validate all alignment',
      output: '✓ Spec-test alignment: 95%\n✓ Test-impl alignment: 98%\n⚠ 2 scenarios missing tests',
    },
  ],
  relatedCommands: ['validate', 'check'],
  notes: [
    'Checks that feature scenarios have corresponding tests',
    'Verifies test coverage',
  ],
};

export default config;
