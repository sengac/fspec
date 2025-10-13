import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'record-tokens',
  description: 'Record AI token usage for a work unit',
  usage: 'fspec record-tokens <workUnitId> <tokens> [options]',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'tokens',
      description: 'Number of tokens used',
      required: true,
    },
  ],
  options: [
    {
      flag: '--model <model>',
      description: 'AI model name',
    },
  ],
  examples: [
    {
      command: 'fspec record-tokens AUTH-001 15000',
      description: 'Record token usage',
      output: '✓ Recorded 15000 tokens for AUTH-001',
    },
  ],
  relatedCommands: ['show-work-unit', 'query-metrics'],
};

export default config;
