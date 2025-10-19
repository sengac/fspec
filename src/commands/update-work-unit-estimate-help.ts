import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-work-unit-estimate',
  description: 'Set story point estimate for a work unit using Fibonacci scale',
  usage: 'fspec update-work-unit-estimate <id> <points>',
  whenToUse:
    'Use after generating scenarios from Example Mapping when you have enough information to estimate complexity. IMPORTANT: Story and Bug work units require a completed feature file before estimation. Task work units can be estimated at any stage.',
  prerequisites: [
    'For story/bug work units: Feature file must exist and be complete (no prefill placeholders)',
    'For task work units: No prerequisites (tasks do not require feature files)',
    'Work unit must be in specifying phase or later',
  ],
  arguments: [
    {
      name: 'id',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'points',
      description: 'Story points (Fibonacci: 1, 2, 3, 5, 8, 13, 21)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec update-work-unit-estimate AUTH-001 5',
      description: 'Set estimate for story work unit (requires completed feature file)',
      output: '✓ Work unit AUTH-001 estimate set to 5',
    },
    {
      command: 'fspec update-work-unit-estimate TASK-001 3',
      description: 'Set estimate for task work unit (no feature file required)',
      output: '✓ Work unit TASK-001 estimate set to 3',
    },
  ],
  commonErrors: [
    {
      error: 'ACDD requires feature file completion before estimation',
      solution:
        'Complete the specifying phase first: Use Example Mapping, generate scenarios, and ensure feature file has no prefill placeholders',
    },
    {
      error: 'Feature file has prefill placeholders must be removed first',
      solution:
        'Remove all placeholders like [role], [action], [benefit] using fspec CLI commands (NOT Write/Edit tools)',
    },
    {
      error: 'Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21',
      solution: 'Use only Fibonacci numbers for estimates',
    },
  ],
  relatedCommands: [
    'show-work-unit',
    'query-estimate-accuracy',
    'generate-scenarios',
    'set-user-story',
  ],
  notes: [
    'Use Fibonacci sequence for estimates: 1, 2, 3, 5, 8, 13, 21',
    'Estimate AFTER generating scenarios from Example Mapping and completing feature file',
    '1=trivial, 3=small, 5=medium, 8=large, 13+=very large',
    'Story/Bug types: MUST have completed feature file with @WORK-UNIT-ID tag',
    'Task types: Can be estimated without feature files',
    'Prefill placeholders ([role], [action], etc.) block estimation',
    'IMPORTANT: Estimates > 13 points trigger a warning in show-work-unit recommending breakdown into smaller work units (1-13 points each)',
    'Large estimate warnings persist until estimate ≤ 13 or status = done (tasks exempt from warnings)',
  ],
};

export default config;
