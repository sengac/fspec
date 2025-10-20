import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'workflow-automation',
  description:
    'Workflow automation utilities for work units (record iterations, track tokens, auto-advance state, validate spec alignment)',
  usage:
    'fspec workflow-automation <action> [work-unit-id] [options]\n\nActions:\n  record-iteration     - Increment iteration counter for work unit\n  record-tokens        - Record token usage for work unit\n  auto-advance         - Auto-advance state after event (tests-pass, validation-pass, specs-complete)\n  validate-alignment   - Check if work unit has corresponding Gherkin scenarios',
  whenToUse:
    'Use this command for workflow automation and metrics tracking. Essential for AI agents to track progress, measure efficiency (iterations, tokens), auto-advance work unit states after successful events, and validate alignment between work units and Gherkin specs.',
  prerequisites: [
    'spec/work-units.json exists with work units',
    'Work unit is in appropriate state for action',
  ],
  arguments: [
    {
      name: 'action',
      description:
        'Action to perform: record-iteration, record-tokens, auto-advance, validate-alignment',
      required: true,
    },
    {
      name: 'work-unit-id',
      description: 'Work unit ID (required for all actions)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--tokens <count>',
      description: 'Number of tokens to record (used with record-tokens)',
    },
    {
      flag: '--event <event>',
      description:
        'Event triggering state change: tests-pass, validation-pass, specs-complete (used with auto-advance)',
    },
    {
      flag: '--from-state <state>',
      description:
        'Current state before transition (used with auto-advance for safety check)',
    },
  ],
  examples: [
    {
      command: 'fspec workflow-automation record-iteration AUTH-001',
      description: 'Increment iteration counter (tracks red→green cycles)',
      output: '✓ Recorded iteration for AUTH-001\n  Total iterations: 3',
    },
    {
      command:
        'fspec workflow-automation record-tokens AUTH-001 --tokens 15000',
      description: 'Record token usage for work unit',
      output: '✓ Recorded 15000 tokens for AUTH-001\n  Total tokens: 42000',
    },
    {
      command:
        'fspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing',
      description: 'Auto-advance from testing → implementing after tests pass',
      output:
        '✓ Advanced AUTH-001 from testing → implementing\n  Event: tests-pass\n  Updated: 2025-01-15T10:30:00Z',
    },
    {
      command:
        'fspec workflow-automation auto-advance AUTH-001 --event validation-pass --from-state validating',
      description:
        'Auto-advance from validating → done after validation passes',
      output:
        '✓ Advanced AUTH-001 from validating → done\n  Event: validation-pass',
    },
    {
      command: 'fspec workflow-automation validate-alignment AUTH-001',
      description: 'Check if work unit has corresponding Gherkin scenarios',
      output:
        '✓ Work unit AUTH-001 is aligned with specifications\n  Scenarios found: 5\n  Features:\n    - spec/features/user-authentication.feature\n    - spec/features/session-management.feature',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Work unit does not exist',
      fix: 'Verify work unit ID. Run: fspec list-work-units',
    },
    {
      error: 'Error: Invalid transition: tests-pass from implementing',
      fix: 'tests-pass event only valid from "testing" state. Check current state with: fspec show-work-unit <id>',
    },
    {
      error: 'Error: Work unit is in state "implementing", expected "testing"',
      fix: 'State mismatch. Verify current state before auto-advance. Use --from-state to ensure safety.',
    },
    {
      error: 'Error: No scenarios found for work unit AUTH-001',
      fix: 'Work unit has no @AUTH-001 tagged scenarios. Add tag to scenarios or create feature file.',
    },
  ],
  typicalWorkflow:
    '1. Start work: fspec update-work-unit-status <id> testing → 2. Write tests (red) → 3. Record iteration: fspec workflow-automation record-iteration <id> → 4. Implement (green) → 5. Tests pass → 6. Auto-advance: fspec workflow-automation auto-advance <id> --event tests-pass --from-state testing → 7. Validate alignment: fspec workflow-automation validate-alignment <id>',
  commonPatterns: [
    {
      pattern: 'TDD Cycle Tracking',
      example:
        '# Start testing phase\nfspec update-work-unit-status AUTH-001 testing\n\n# Red: write failing test\nnpm test  # Tests fail\nfspec workflow-automation record-iteration AUTH-001\n\n# Green: implement code\nnpm test  # Tests pass\nfspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing\n\n# Now in implementing state, ready for next feature',
    },
    {
      pattern: 'Token Usage Tracking',
      example:
        '# AI agent tracks token usage per work unit\nfspec workflow-automation record-tokens AUTH-001 --tokens 12500\n\n# After multiple iterations\nfspec workflow-automation record-tokens AUTH-001 --tokens 8300\nfspec workflow-automation record-tokens AUTH-001 --tokens 15700\n\n# Total tokens: 36500 (tracked in metrics)',
    },
    {
      pattern: 'ACDD State Automation',
      example:
        '# Specifying → Testing (after specs complete)\nfspec workflow-automation auto-advance AUTH-001 --event specs-complete --from-state specifying\n\n# Testing → Implementing (after tests pass)\nfspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing\n\n# Validating → Done (after validation passes)\nfspec workflow-automation auto-advance AUTH-001 --event validation-pass --from-state validating',
    },
    {
      pattern: 'Spec Alignment Validation',
      example:
        '# After creating feature files, validate alignment\nfspec workflow-automation validate-alignment AUTH-001\n\n# If not aligned, add @AUTH-001 tag to scenarios\nfspec add-scenario user-authentication "Login flow" --tags @AUTH-001\n\n# Re-validate\nfspec workflow-automation validate-alignment AUTH-001',
    },
  ],
  relatedCommands: [
    'update-work-unit-status',
    'show-work-unit',
    'create-work-unit',
    'list-features',
    'add-scenario',
  ],
  notes: [
    'Metrics tracked:',
    '  - iterations: Number of red→green cycles (TDD iterations)',
    '  - actualTokens: Cumulative token usage across all iterations',
    'State transitions (auto-advance):',
    '  - specs-complete: specifying → testing',
    '  - tests-pass: testing → implementing',
    '  - validation-pass: validating → done',
    'Auto-advance safety:',
    '  - Requires --from-state to verify current state before transition',
    '  - Prevents invalid state transitions',
    '  - Updates stateHistory with timestamp',
    'Spec alignment:',
    '  - Searches for @WORK-UNIT-ID tags in feature files',
    '  - Returns count of matching scenarios and feature files',
    '  - Essential for reverse ACDD to track specification coverage',
    'Iteration tracking:',
    '  - Increments on each red→green cycle',
    '  - Useful metric for complexity/difficulty assessment',
    '  - High iteration count may indicate unclear requirements',
  ],
};

export default config;
