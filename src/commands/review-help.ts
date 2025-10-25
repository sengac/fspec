import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'review',
  description:
    'Perform comprehensive review of work unit with ACDD compliance, quality checks, and coding standards validation',
  usage: 'fspec review <work-unit-id>',
  whenToUse:
    'Use when you need to validate a work unit against ACDD workflow, check test coverage, verify coding standards, and get actionable recommendations before marking work as complete.',
  arguments: [
    {
      name: 'work-unit-id',
      description:
        'ID of work unit to review (e.g., AUTH-001, UI-003, API-005)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec review AUTH-001',
      description: 'Review work unit for ACDD compliance and quality',
      output:
        '================================================================================\n' +
        'REVIEW: AUTH-001 - User Login\n' +
        '================================================================================\n\n' +
        '## Issues Found\n\n' +
        '### 🔴 Critical Issues\n' +
        'No critical issues detected.\n\n' +
        '### 🟡 Warnings\n' +
        'No warnings detected.\n\n' +
        '## ACDD Compliance\n\n' +
        '✅ **Passed:**\n' +
        '- Example Mapping completed (3 rules, 5 examples, 2 questions answered)\n' +
        '- Feature file created during specifying phase\n' +
        '- All scenarios have test coverage (100%)\n' +
        '- Temporal ordering verified (5 state transitions)\n\n' +
        '## Coverage Analysis\n\n' +
        '- **Total Scenarios:** 4\n' +
        '- **Covered Scenarios:** 4 (100%)\n\n' +
        '## Summary\n\n' +
        '**Overall Assessment:** PASS\n\n' +
        '**Priority Actions:**\n' +
        '1. Work unit review complete - no critical actions needed',
    },
    {
      command: 'fspec review API-005',
      description: 'Review work unit with issues detected',
      output:
        '================================================================================\n' +
        'REVIEW: API-005 - Data Validation\n' +
        '================================================================================\n\n' +
        '## Issues Found\n\n' +
        '### 🔴 Critical Issues\n' +
        '1. **Issue:** Use of `any` type detected\n' +
        '   - **Location:** src/__tests__/validation.test.ts\n' +
        '   - **Fix:** Replace `any` with proper TypeScript types\n' +
        '   - **Action:** Review file and add proper type annotations\n\n' +
        '### 🟡 Warnings\n' +
        'No warnings detected.\n\n' +
        '## Recommendations\n\n' +
        '1. **Recommendation:** Add tests for uncovered scenarios\n' +
        '   - **Rationale:** All acceptance criteria must have corresponding tests\n' +
        '   - **Action:** fspec show-coverage data-validation to see uncovered scenarios\n\n' +
        '## ACDD Compliance\n\n' +
        '✅ **Passed:**\n' +
        '- Example Mapping completed (2 rules, 3 examples, 1 questions answered)\n' +
        '- Feature file created during specifying phase\n' +
        '- Temporal ordering verified (4 state transitions)\n\n' +
        '❌ **Failed:**\n' +
        '- Incomplete test coverage (75%)\n\n' +
        '## Coverage Analysis\n\n' +
        '- **Total Scenarios:** 4\n' +
        '- **Covered Scenarios:** 3 (75%)\n\n' +
        '**Uncovered Scenarios:**\n' +
        '  - Validate nested object structure\n\n' +
        '## Summary\n\n' +
        '**Overall Assessment:** CRITICAL ISSUES\n\n' +
        '**Priority Actions:**\n' +
        '1. Fix 1 critical issue(s)\n' +
        '2. Address ACDD compliance violations\n' +
        '3. Complete test coverage for all scenarios',
    },
  ],
  prerequisites: [
    'Work unit must exist in spec/work-units.json',
    'For full analysis, work unit should have linked feature files and test coverage',
  ],
  typicalWorkflow:
    'implement code → fspec review <id> → fix issues → validate → mark done',
  relatedCommands: [
    'show-work-unit',
    'show-coverage',
    'validate',
    'update-work-unit-status',
  ],
  notes: [
    'Review checks: ACDD compliance, test coverage, coding standards, temporal ordering',
    'Output is agent-aware: <system-reminder> for Claude, **⚠️ IMPORTANT:** for IDE agents, **IMPORTANT:** for CLI agents',
    'Critical issues prevent work from being marked as done',
    'Review is non-destructive - only reports issues, does not modify files',
  ],
  commonErrors: [
    {
      error: 'Error: Work unit \'XYZ-999\' does not exist',
      solution:
        'Check work unit ID with: fspec list-work-units\nEnsure work unit was created with: fspec create-work-unit',
    },
  ],
  commonPatterns: [
    {
      pattern: 'Review before marking done',
      code:
        '# Complete implementation\n' +
        'fspec update-work-unit-status AUTH-001 validating\n\n' +
        '# Review for issues\n' +
        'fspec review AUTH-001\n\n' +
        '# If review passes\n' +
        'fspec update-work-unit-status AUTH-001 done',
    },
    {
      pattern: 'Review in-progress work',
      code:
        '# Check current state\n' +
        'fspec show-work-unit UI-003\n\n' +
        '# Review what\'s done so far\n' +
        'fspec review UI-003\n\n' +
        '# Address issues found\n' +
        '# Continue implementation',
    },
  ],
};

export default config;
