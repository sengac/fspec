import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'bootstrap',
  description:
    'Load complete fspec documentation and workflow guidance for AI agents. Internally calls all help section functions to provide comprehensive context.',
  usage: 'fspec bootstrap [options]',
  whenToUse:
    'Use this command at the start of EVERY fspec session to load complete workflow documentation into AI agent context. Required before using any fspec commands. This is typically called automatically by the /fspec slash command in Claude Code.',
  prerequisites: [
    'fspec must be installed and accessible',
    'Run from project root directory',
    'Foundation file (spec/foundation.json) should exist (will prompt to create if missing)',
    'Tool configuration recommended (fspec configure-tools)',
  ],
  arguments: [],
  options: [],
  examples: [
    {
      command: 'fspec bootstrap',
      description: 'Load complete fspec documentation',
      output:
        '# fspec Command - Kanban-Based Project Management\n\n[Complete workflow documentation output...]\n\nStep 1: Load fspec Context\n...',
    },
  ],
  commonErrors: [],
  typicalWorkflow:
    '1. Start session: fspec bootstrap\n2. Load context into AI agent memory\n3. Begin work: fspec board (view Kanban)\n4. Follow ACDD workflow: discovery → specifying → testing → implementing → validating → done',
  relatedCommands: ['configure-tools', 'discover-foundation', 'board', 'help'],
  notes: [
    'Outputs comprehensive documentation by calling: getSpecsHelpContent(), getWorkHelpContent(), getDiscoveryHelpContent(), getMetricsHelpContent(), getSetupHelpContent(), getHooksHelpContent()',
    'Typically invoked by /fspec slash command automatically',
    'Output is designed for AI agent context loading, not human reading',
    'Includes ACDD workflow, Example Mapping guidance, and all command documentation',
  ],
};

export default config;
