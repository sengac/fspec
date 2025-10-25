import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-virtual-hook',
  description: 'Add a work unit-scoped virtual hook for dynamic validation',
  usage: 'fspec add-virtual-hook <workUnitId> <event> <command> [options]',
  whenToUse:
    'Use this command to attach ephemeral validation hooks to specific work units. Virtual hooks are scoped to a single work unit and run BEFORE global hooks. Perfect for one-off quality checks (linting, type checking, security scans) that apply only to the current story/bug/task.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID to attach hook to (e.g., AUTH-001, BUG-042)',
      required: true,
    },
    {
      name: 'event',
      description:
        'Hook event when command should run (e.g., post-implementing, pre-validating)',
      required: true,
    },
    {
      name: 'command',
      description: 'Command to execute (e.g., "npm run lint", "eslint src/")',
      required: true,
    },
  ],
  options: [
    {
      flag: '--blocking',
      description:
        'If set, hook failure prevents workflow transition. AI sees failure in <system-reminder> tags.',
    },
    {
      flag: '--git-context',
      description:
        'Provide git context (staged/unstaged files). Generates script in spec/hooks/.virtual/ that processes changed files only.',
    },
  ],
  examples: [
    {
      command:
        'fspec add-virtual-hook AUTH-001 post-implementing "npm run lint" --blocking',
      description:
        'Add blocking lint check after implementing (prevents validating if lint fails)',
      output: '✓ Virtual hook added to AUTH-001\n  Total virtual hooks: 1',
    },
    {
      command:
        'fspec add-virtual-hook BUG-042 pre-validating "npm run typecheck" --blocking',
      description: 'Add type check before validation phase',
      output: '✓ Virtual hook added to BUG-042\n  Total virtual hooks: 1',
    },
    {
      command:
        'fspec add-virtual-hook FEAT-123 post-implementing "eslint src/" --git-context --blocking',
      description: 'Lint only changed files using git context (more efficient)',
      output: '✓ Virtual hook added to FEAT-123\n  Total virtual hooks: 1',
    },
    {
      command: 'fspec add-virtual-hook AUTH-001 post-validating "npm audit"',
      description:
        'Run security audit (non-blocking - shows output but allows completion)',
      output: '✓ Virtual hook added to AUTH-001\n  Total virtual hooks: 2',
    },
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      fix: 'Create the work unit first: fspec create-story AUTH "Title" (or create-bug/create-task)',
    },
    {
      error: 'Hook command not found: eslint',
      fix: 'Install the tool first (npm install -D eslint) or use a different command.',
    },
  ],
  typicalWorkflow:
    'Move to specifying → AI asks about quality checks → Add virtual hooks → Move to testing → Hooks run automatically at specified events → Move to done → AI asks to keep or remove hooks',
  relatedCommands: [
    'list-virtual-hooks',
    'remove-virtual-hook',
    'clear-virtual-hooks',
    'copy-virtual-hooks',
    'update-work-unit-status',
  ],
  notes: [
    'Virtual hooks are stored in spec/work-units.json under workUnit.virtualHooks array',
    'Virtual hooks run BEFORE global hooks (from spec/fspec-hooks.json)',
    'Multiple hooks can be added to the same event - they execute sequentially',
    'Hook name is auto-generated from command (e.g., "eslint src/" → "eslint")',
    'Git context hooks generate script files in spec/hooks/.virtual/',
    'Scripts are automatically cleaned up when hooks are removed',
    'Blocking hooks emit <system-reminder> tags on failure for AI agents',
    'Non-blocking hooks show output but do not prevent workflow transitions',
  ],
  prerequisites: [
    'Work unit must exist (fspec create-story, create-bug, or create-task)',
    'Command/tool must be installed and available in PATH',
  ],
  commonPatterns: [
    {
      pattern: 'Linting Before Implementation',
      example:
        'fspec add-virtual-hook AUTH-001 pre-implementing "npm run lint" --blocking',
      description:
        'Ensures code is clean before starting implementation. Prevents messy code from being committed.',
    },
    {
      pattern: 'Type Checking Before Validation',
      example:
        'fspec add-virtual-hook AUTH-001 pre-validating "npm run typecheck" --blocking',
      description:
        'Catches type errors before moving to validation phase. Strict quality gate.',
    },
    {
      pattern: 'Security Scan on Changed Files',
      example:
        'fspec add-virtual-hook FEAT-123 post-implementing "npm audit" --git-context',
      description:
        'Runs security audit only on changed files for efficiency. Uses git context.',
    },
    {
      pattern: 'Multiple Quality Checks',
      example:
        'fspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --blocking\nfspec add-virtual-hook AUTH-001 post-implementing "prettier --check ." --blocking',
      description:
        'Adds multiple hooks to same event. Both must pass to proceed.',
    },
  ],
};

export default config;
