import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-hook',
  description: 'Add a lifecycle hook to the configuration',
  usage: 'fspec add-hook <event> <name> --command <path> [options]',
  whenToUse:
    'Use this command to add a new lifecycle hook to your project. Hooks execute custom scripts at specific command events (pre-/post- pattern) and enable workflow automation like quality gates, testing, and notifications.',
  arguments: [
    {
      name: 'event',
      description:
        'Event name when hook should run (e.g., pre-update-work-unit-status, post-implementing)',
      required: true,
    },
    {
      name: 'name',
      description: 'Unique name for this hook (e.g., validate-feature, run-tests)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--command <path>',
      description:
        'Path to hook script, relative to project root (e.g., spec/hooks/validate.sh)',
    },
    {
      flag: '--blocking',
      description:
        'If set, hook failure prevents command execution (pre-hooks) or sets exit code to 1 (post-hooks)',
    },
    {
      flag: '--timeout <seconds>',
      description: 'Timeout in seconds (default: 60). Hook is killed if it exceeds this time.',
    },
  ],
  examples: [
    {
      command:
        'fspec add-hook pre-implementing validate-code --command spec/hooks/lint.sh --blocking',
      description: 'Add blocking pre-hook to validate code before implementing',
      output: '✓ Hook added: pre-implementing/validate-code',
    },
    {
      command:
        'fspec add-hook post-implementing run-tests --command spec/hooks/test.sh --timeout 300',
      description: 'Add post-hook with custom timeout',
      output: '✓ Hook added: post-implementing/run-tests',
    },
    {
      command:
        'fspec add-hook pre-update-work-unit-status check-ready --command spec/hooks/check.sh --blocking',
      description: 'Add quality gate before status changes',
      output: '✓ Hook added: pre-update-work-unit-status/check-ready',
    },
  ],
  commonErrors: [
    {
      error: 'Hook script does not exist',
      fix: 'Create the hook script first, then add the hook. Make sure the script is executable (chmod +x).',
    },
    {
      error: 'Hook name already exists for this event',
      fix: 'Choose a unique name or remove the existing hook first with fspec remove-hook.',
    },
  ],
  typicalWorkflow:
    'Create hook script → Make executable (chmod +x) → fspec add-hook → fspec validate-hooks → Test execution',
  relatedCommands: ['remove-hook', 'list-hooks', 'validate-hooks'],
  notes: [
    'Creates spec/fspec-hooks.json if it does not exist',
    'Hook scripts must be executable (chmod +x script-path)',
    'Paths must be relative to project root, not to spec/ directory',
    'Event names follow pre-<command> and post-<command> pattern',
    'Blocking hooks emit <system-reminder> tags on failure (for AI agents)',
    'Use --blocking for quality gates, omit for notifications',
  ],
  prerequisites: [
    'Hook script exists at the specified path',
    'Hook script is executable (chmod +x)',
  ],
  commonPatterns: [
    {
      pattern: 'Quality Gate (Blocking Pre-Hook)',
      example:
        'fspec add-hook pre-implementing validate --command spec/hooks/lint.sh --blocking',
      description:
        'Prevents implementing phase if linting fails. AI agents see failure in <system-reminder>.',
    },
    {
      pattern: 'Automated Testing (Post-Hook)',
      example:
        'fspec add-hook post-implementing test --command spec/hooks/test.sh --timeout 300',
      description: 'Runs tests after implementation completes. Failure sets exit code to 1.',
    },
    {
      pattern: 'Notification (Non-Blocking Post-Hook)',
      example:
        'fspec add-hook post-validating notify --command spec/hooks/notify-slack.sh',
      description:
        'Sends notifications after validation. Failure does not prevent completion.',
    },
  ],
};

export default config;
