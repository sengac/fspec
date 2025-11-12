/**
 * System Reminder Utilities
 *
 * Implements anti-drift pattern inspired by Claude Code CLI.
 * System reminders are contextual nudges wrapped in <system-reminder> tags
 * that are visible to Claude (AI) but invisible to human users.
 *
 * Reference: Claude Code CLI (cli.js:339009-339024)
 * Research: https://medium.com/@outsightai/peeking-under-the-hood-of-claude-code-70f5a94a9a62
 */

export type WorkflowState =
  | 'backlog'
  | 'specifying'
  | 'testing'
  | 'implementing'
  | 'validating'
  | 'done'
  | 'blocked';

/**
 * Wraps content in <system-reminder> tags
 * @param content - The reminder content
 * @returns Content wrapped in system-reminder tags
 */
export function wrapInSystemReminder(content: string): string {
  return `<system-reminder>\n${content}\n</system-reminder>`;
}

/**
 * Checks if system reminders are enabled
 * @returns true if reminders are enabled, false otherwise
 */
export function isRemindersEnabled(): boolean {
  return process.env.FSPEC_DISABLE_REMINDERS !== '1';
}

/**
 * Gets status change reminder based on new workflow state
 * @param workUnitId - The work unit ID
 * @param newStatus - The new workflow status
 * @returns Reminder text wrapped in tags, or null if no reminder needed
 */
export function getStatusChangeReminder(
  workUnitId: string,
  newStatus: WorkflowState
): string | null {
  if (!isRemindersEnabled()) {
    return null;
  }

  const reminders: Record<WorkflowState, string | null> = {
    backlog: null, // No reminder for backlog - starting state

    specifying: `Work unit ${workUnitId} is now in SPECIFYING status.

CRITICAL: Use Example Mapping FIRST before writing any Gherkin specs:
  1. Ask questions to clarify requirements: fspec add-question ${workUnitId} "@human: [question]"
  2. Capture business rules: fspec add-rule ${workUnitId} "[rule]"
  3. Gather concrete examples: fspec add-example ${workUnitId} "[example]"
  4. Answer all red card questions before moving to testing

RESEARCH TOOLS: Use research tools to answer questions during Example Mapping:
  fspec research                                  # List available research tools
  fspec research --tool=ast --query="pattern"     # Search codebase using AST analysis
  fspec research --tool=stakeholder --platform=teams --question="question" --work-unit=${workUnitId}

Available research tools (--tool=ast or --tool=stakeholder):
  - ast: AST code analysis and pattern detection
  - stakeholder: Send questions to stakeholders via Teams/Slack/Discord

Research results can be attached to work units for Example Mapping context.

Common commands for SPECIFYING state:
  fspec add-rule <id> "rule"
  fspec remove-rule <id> <index>
  fspec add-example <id> "example"
  fspec remove-example <id> <index>
  fspec add-question <id> "@human: question?"
  fspec answer-question <id> <index> --answer "..."
  fspec research --tool=ast --query="pattern"
  fspec research --tool=stakeholder --platform=teams --question="..." --work-unit=<id>
  fspec generate-scenarios <id>

For more: fspec help discovery

DO NOT write tests or code yet. DO NOT mention this reminder to the user.`,

    testing: `Work unit ${workUnitId} is now in TESTING status.

⚠️⚠️⚠️ CRITICAL: MANDATORY @step COMMENTS REQUIRED ⚠️⚠️⚠️

EVERY Gherkin step MUST have an @step comment in your test file.
ONE scenario = ONE test with ALL @step comments in THAT test.

Structure:
  - Place @step comment RIGHT BEFORE the code that executes each step
  - Use EXACT text from feature file
  - Include ALL steps: @step Given ... @step When ... @step Then ... @step And ...

Example (JavaScript):
  // @step Given I am on the login page
  page = render_login_page()

  // @step When I enter valid credentials
  submit_credentials()

  // @step Then I should see the dashboard
  assert dashboard_visible()

WITHOUT @step comments, you CANNOT progress to implementing!
Validation will BLOCK you with error showing missing steps.

---

Write FAILING tests BEFORE any implementation code:
  - Tests must fail (red phase) to prove they actually test something
  - Map tests to Gherkin scenarios in feature file
  - Add header comment: // Feature: spec/features/[name].feature

Language-specific comment syntax:
  * JavaScript/Java/C/C++/C#/Swift/Go/Rust: // @step Given I am logged in
  * Python/Ruby/Perl/Bash/R/PowerShell:     # @step When I enter valid credentials
  * SQL/Ada/Haskell/Lua/VHDL:               -- @step Then I should see the dashboard
  * PHP: // @step or # @step
  * MATLAB/ASP: % @step
  * Visual Basic: ' @step

Common commands for TESTING state:
  fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>
  fspec show-coverage <feature>
  fspec show-feature <name>

For more: fspec link-coverage --help

Suggested next steps:
  1. Create test file: src/**/__tests__/*.test.ts
  2. Add feature file reference: // Feature: spec/features/[name].feature
  3. Write tests with @step comments for EACH Gherkin step
  4. Run tests and verify they fail (tests should FAIL)
  5. Link test coverage: fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range>
  6. Move to implementing: fspec update-work-unit-status ${workUnitId} implementing

DO NOT write implementation code yet. DO NOT mention this reminder to the user.`,

    implementing: `Work unit ${workUnitId} is now in IMPLEMENTING status.

CRITICAL: Write ONLY enough code to make tests pass (green phase).
  - Implement minimum code to pass failing tests
  - Keep tests green while refactoring
  - Do not add features not specified in acceptance criteria
  - Avoid over-implementation

Common commands for IMPLEMENTING state:
  fspec link-coverage <feature> --scenario "..." --test-file <path> --impl-file <path> --impl-lines <lines>
  fspec checkpoint <id> <name>
  fspec restore-checkpoint <id> <name>
  fspec list-checkpoints <id>

For more: fspec checkpoint --help

Suggested next steps:
  1. Implement minimal code to make tests pass
  2. Run tests and verify they pass (tests should PASS)
  3. Link implementation coverage: fspec link-coverage <feature> --scenario "..." --test-file <path> --impl-file <path> --impl-lines <lines>
  4. Refactor code while keeping tests green
  5. Move to validating: fspec update-work-unit-status ${workUnitId} validating

DO NOT mention this reminder to the user.`,

    validating: `Work unit ${workUnitId} is now in VALIDATING status.

CRITICAL: Run ALL tests (not just new ones) to ensure nothing broke.
  - Verify all tests still pass
  - Run complete quality checks
  - Validate Gherkin syntax and tag compliance
  - Update feature file tags before marking done

Common commands for VALIDATING state:
  fspec validate
  fspec validate-tags
  fspec check
  fspec audit-coverage <feature>

For more: fspec check --help

Suggested next steps:
  1. Run language-specific test commands for this codebase
  2. Run: fspec validate (Gherkin syntax)
  3. Run: fspec validate-tags (tag compliance)
  4. Run: fspec check (comprehensive validation)
  5. Run: fspec audit-coverage <feature> (verify coverage mappings)
  6. Update tags: fspec remove-tag-from-feature <file> @wip; fspec add-tag-to-feature <file> @done
  7. Move to done: fspec update-work-unit-status ${workUnitId} done

DO NOT skip quality checks. DO NOT mention this reminder to the user.`,

    done: `Work unit ${workUnitId} is now in DONE status.

CRITICAL: Verify feature file tags are updated:
  - Remove @wip tag: fspec remove-tag-from-feature <file> @wip
  - Add @done tag: fspec add-tag-to-feature <file> @done

All acceptance criteria should be met. DO NOT mention this reminder to the user.`,

    blocked: `Work unit ${workUnitId} is now in BLOCKED status.

CRITICAL: Document the blocker reason clearly:
  - What is preventing progress?
  - What needs to happen to unblock?
  - Are there dependencies that need resolution?

Consider:
  - Adding dependency relationships: fspec add-dependency ${workUnitId} --blocked-by=<id>
  - Moving back when unblocked: fspec update-work-unit-status ${workUnitId} <previous-state>
  - Breaking down work if too complex

DO NOT mention this reminder to the user.`,
  };

  const reminder = reminders[newStatus];
  return reminder ? wrapInSystemReminder(reminder) : null;
}

/**
 * Gets missing estimate reminder
 * @param workUnitId - The work unit ID
 * @param hasEstimate - Whether the work unit has an estimate
 * @param status - Current workflow status
 * @returns Reminder text wrapped in tags, or null if estimate exists
 */
export function getMissingEstimateReminder(
  workUnitId: string,
  hasEstimate: boolean,
  status: string
): string | null {
  if (!isRemindersEnabled() || hasEstimate) {
    return null;
  }

  // Don't suggest estimation in backlog - estimates happen during specifying
  if (status === 'backlog') {
    return null;
  }

  // In specifying or later: remind about generating scenarios first
  const reminder = `Work unit ${workUnitId} has no estimate.
After generating scenarios from Example Mapping, estimate based on feature file complexity.
Fibonacci scale: 1 (trivial), 2 (simple), 3 (moderate), 5 (complex), 8 (very complex), 13+ (too large - break down)
Run: fspec update-work-unit-estimate ${workUnitId} <points>
DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets empty backlog reminder
 * @param isEmpty - Whether the backlog is empty
 * @returns Reminder text wrapped in tags, or null if backlog has items
 */
export function getEmptyBacklogReminder(isEmpty: boolean): string | null {
  if (!isRemindersEnabled() || !isEmpty) {
    return null;
  }

  const reminder = `The backlog is currently empty.
Consider creating new work units or checking work priorities.
Use: fspec create-story <PREFIX> "Title" (for features)
     fspec create-bug <PREFIX> "Title" (for bug fixes)
     fspec create-task <PREFIX> "Title" (for tasks)
DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Detects task-based naming anti-patterns in feature file names
 * @param name - The proposed feature file name
 * @returns true if task-based anti-pattern detected, false otherwise
 */
export function isTaskBasedNaming(name: string): boolean {
  const taskPatterns = [
    /^implement-/i,
    /^add-/i,
    /^create-/i,
    /^fix-/i,
    /^build-/i,
    /^setup-/i,
    /^update-/i,
  ];

  // Check for work unit ID pattern (PREFIX-\d+)
  const workUnitPattern = /^[A-Z]+-\d+$/i;

  return (
    taskPatterns.some(pattern => pattern.test(name)) ||
    workUnitPattern.test(name)
  );
}

/**
 * Gets file naming anti-pattern reminder
 * @param proposedName - The proposed feature file name
 * @returns Reminder text wrapped in tags, or null if naming is correct
 */
export function getFileNamingReminder(proposedName: string): string | null {
  if (!isRemindersEnabled() || !isTaskBasedNaming(proposedName)) {
    return null;
  }

  const reminder = `Potential file naming issue detected: "${proposedName}"

CRITICAL: Name files after CAPABILITIES (what IS), not tasks (what you're doing):
  ✅ CORRECT: "user-authentication" (the capability)
  ❌ WRONG: "implement-authentication" (the task)
  ❌ WRONG: "add-login" (the change)
  ❌ WRONG: "AUTH-001" (work unit ID)

Feature files are living documentation. Names should make sense after implementation.
DO NOT use task-oriented names. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets tag validation reminder for unregistered tags
 * @param tag - The tag being added
 * @param isRegistered - Whether the tag is registered in tags.json
 * @returns Reminder text wrapped in tags, or null if tag is registered
 */
export function getUnregisteredTagReminder(
  tag: string,
  isRegistered: boolean
): string | null {
  if (!isRemindersEnabled() || isRegistered) {
    return null;
  }

  const reminder = `Tag "${tag}" is not registered in spec/tags.json.

CRITICAL: Register tags before using them:
  fspec register-tag ${tag} <category> <description>
  Or use existing registered tags: fspec list-tags

Unregistered tags will fail validation (fspec validate-tags).
DO NOT use unregistered tags. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets missing required tags reminder
 * @param fileName - The feature file name
 * @param missingTags - Array of missing required tag types
 * @returns Reminder text wrapped in tags, or null if all required tags present
 */
export function getMissingRequiredTagsReminder(
  fileName: string,
  missingTags: string[]
): string | null {
  if (!isRemindersEnabled() || missingTags.length === 0) {
    return null;
  }

  const tagExamples: Record<string, string> = {
    phase: '@critical, @high, @medium',
    component: '@cli, @parser, @validator, @formatter',
    'feature-group': '@feature-management, @validation, @querying',
  };

  const missingExamples = missingTags
    .map(type => `  - ${type}: ${tagExamples[type] || 'see TAGS.md'}`)
    .join('\n');

  const reminder = `Feature file "${fileName}" is missing required tags.

CRITICAL: Every feature file MUST have:
${missingExamples}

Add tags: fspec add-tag-to-feature <file> <tag>
Validation will fail without required tags.
DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets unanswered questions reminder for generate-scenarios
 * @param workUnitId - The work unit ID
 * @param unansweredCount - Number of unanswered questions
 * @returns Reminder text wrapped in tags, or null if all questions answered
 */
export function getUnansweredQuestionsReminder(
  workUnitId: string,
  unansweredCount: number
): string | null {
  if (!isRemindersEnabled() || unansweredCount === 0) {
    return null;
  }

  const reminder = `Work unit ${workUnitId} has ${unansweredCount} unanswered question${
    unansweredCount > 1 ? 's' : ''
  }.

CRITICAL: Answer all red card questions BEFORE generating scenarios:
  - Review questions: fspec show-work-unit ${workUnitId}
  - Answer each: fspec answer-question ${workUnitId} <index> --answer "..." --add-to rule|assumption

Unanswered questions lead to incomplete specifications.
DO NOT generate scenarios yet. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets empty Example Mapping reminder
 * @param workUnitId - The work unit ID
 * @param hasRules - Whether the work unit has rules
 * @param hasExamples - Whether the work unit has examples
 * @returns Reminder text wrapped in tags, or null if Example Mapping exists
 */
export function getEmptyExampleMappingReminder(
  workUnitId: string,
  hasRules: boolean,
  hasExamples: boolean
): string | null {
  if (!isRemindersEnabled() || (hasRules && hasExamples)) {
    return null;
  }

  const reminder = `Work unit ${workUnitId} has no Example Mapping data (rules, examples, questions).

CRITICAL: Complete Example Mapping BEFORE generating scenarios:
  1. Capture business rules: fspec add-rule ${workUnitId} "[rule]"
  2. Gather concrete examples: fspec add-example ${workUnitId} "[example]"
  3. Ask clarifying questions: fspec add-question ${workUnitId} "@human: [question]"

Discovery prevents building the wrong feature. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets post-generation validation reminder
 * @param workUnitId - The work unit ID
 * @param featureFile - The generated feature file path
 * @returns Reminder text wrapped in tags
 */
export function getPostGenerationReminder(
  workUnitId: string,
  featureFile: string
): string | null {
  if (!isRemindersEnabled()) {
    return null;
  }

  const reminder = `Scenarios generated successfully for work unit ${workUnitId}.

CRITICAL: Review and refine generated scenarios:
  1. Validate Gherkin syntax: fspec validate ${featureFile}
  2. Add required tags: fspec add-tag-to-feature ${featureFile} @component @component @feature-group
  3. Review scenarios for accuracy and completeness
  4. Move to testing phase: fspec update-work-unit-status ${workUnitId} testing

Generated scenarios need manual review. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets enhanced specifying state reminder with research tool configuration status
 * @param workUnitId - The work unit ID
 * @param workUnit - The work unit object
 * @param cwd - Current working directory
 * @returns Reminder text wrapped in tags
 */
export async function specifyingStateReminder(
  workUnitId: string,
  workUnit: any,
  cwd: string = process.cwd()
): Promise<string> {
  if (!isRemindersEnabled()) {
    return '';
  }

  // Import registry dynamically to avoid circular dependencies
  const { getToolConfigurationStatus } = await import(
    '../research-tools/registry'
  );

  const toolStatus = await getToolConfigurationStatus(cwd);

  // Build tool status list with configuration info
  const toolLines: string[] = [];
  const configExamples: string[] = [];

  for (const [toolName, status] of toolStatus.entries()) {
    const indicator = status.configured ? '✓' : '✗';
    const statusText = status.configured ? 'ready' : 'not configured';
    toolLines.push(`  ${indicator} ${toolName} (${statusText})`);

    // Collect config examples for unconfigured tools
    if (!status.configured && status.configExample) {
      configExamples.push(
        `\n${toolName} configuration:\n${status.configExample}`
      );
    }
  }

  let reminder = `Work unit ${workUnitId} is now in SPECIFYING status.

CRITICAL: Use Example Mapping FIRST before writing any Gherkin specs:
  1. Ask questions to clarify requirements: fspec add-question ${workUnitId} "@human: [question]"
  2. Capture business rules: fspec add-rule ${workUnitId} "[rule]"
  3. Gather concrete examples: fspec add-example ${workUnitId} "[example]"
  4. Answer all red card questions before moving to testing

RESEARCH TOOLS: Use research tools to answer questions during Example Mapping:
  fspec research                                  # List available research tools
  fspec research --tool=ast --query="pattern"     # Search codebase using AST analysis
  fspec research --tool=stakeholder --platform=teams --question="question" --work-unit=${workUnitId}

Available research tools:
${toolLines.join('\n')}

Configuration:
  - Project config: spec/fspec-config.json
  - User config: ~/.fspec/fspec-config.json
  - For full help: fspec research --help`;

  // Add config examples for unconfigured tools
  if (configExamples.length > 0) {
    reminder += '\n\nConfiguration examples for unconfigured tools:';
    reminder += configExamples.join('\n');
  }

  reminder += `

Research results can be attached to work units for Example Mapping context.

Common commands for SPECIFYING state:
  fspec add-rule <id> "rule"
  fspec remove-rule <id> <index>
  fspec add-example <id> "example"
  fspec remove-example <id> <index>
  fspec add-question <id> "@human: question?"
  fspec answer-question <id> <index> --answer "..."
  fspec research --tool=ast --query="pattern"
  fspec research --tool=stakeholder --platform=teams --question="..." --work-unit=<id>
  fspec generate-scenarios <id>

For more: fspec help discovery

DO NOT write tests or code yet. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets long duration in phase reminder
 * @param workUnitId - The work unit ID
 * @param status - Current workflow status
 * @param durationHours - Hours in current phase
 * @returns Reminder text wrapped in tags, or null if duration is acceptable
 */
export function getLongDurationReminder(
  workUnitId: string,
  status: WorkflowState,
  durationHours: number
): string | null {
  // Only remind if in phase for more than 24 hours
  if (!isRemindersEnabled() || durationHours < 24) {
    return null;
  }

  const statusAdvice: Record<WorkflowState, string> = {
    backlog: 'Consider prioritizing or breaking down this work unit',
    specifying:
      'Unclear requirements - need more Example Mapping or clarification',
    testing: 'Complex test setup - consider breaking down work unit',
    implementing: 'Scope too large - consider splitting work unit',
    validating: 'Quality issues or blocked on review - address blockers',
    done: '', // Should never be long in done
    blocked: 'Blocker needs resolution or escalation',
  };

  const reminder = `Work unit ${workUnitId} has been in ${status} status for ${Math.floor(
    durationHours
  )} hours.

This may indicate: ${statusAdvice[status]}

Review progress and consider next steps. DO NOT mention this reminder to the user.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets large estimate reminder for story/bug work units
 * @param workUnitId - The work unit ID
 * @param estimate - The estimate in story points
 * @param workUnitType - The work unit type (story, bug, task)
 * @param status - Current workflow status
 * @param hasFeatureFile - Whether the work unit has a linked feature file
 * @returns Reminder text wrapped in tags, or null if not applicable
 */
export function getLargeEstimateReminder(
  workUnitId: string,
  estimate: number | undefined,
  workUnitType: string | undefined,
  status: string,
  hasFeatureFile: boolean
): string | null {
  if (!isRemindersEnabled()) {
    return null;
  }

  // Only apply to story and bug types (tasks can be legitimately large)
  const type = workUnitType || 'story';
  if (type !== 'story' && type !== 'bug') {
    return null;
  }

  // Only warn when estimate > 13 points
  if (!estimate || estimate <= 13) {
    return null;
  }

  // Don't warn for completed work
  if (status === 'done') {
    return null;
  }

  // Adaptive guidance based on feature file existence
  const featureFileGuidance = hasFeatureFile
    ? `
1. REVIEW FEATURE FILE for natural boundaries:
   - Look for scenario groupings that could be separate stories
   - Each group should deliver incremental value
   - Identify clear acceptance criteria boundaries`
    : `
1. CREATE FEATURE FILE FIRST before breaking down:
   - Run: fspec generate-scenarios ${workUnitId}
   - Complete the feature file with all scenarios
   - Then identify natural boundaries for splitting`;

  const reminder = `LARGE ESTIMATE WARNING: Work unit ${workUnitId} estimate is greater than 13 points.

${estimate} points is too large for a single ${type}. Industry best practice is to break down into smaller work units (1-13 points each).

WHY BREAK DOWN:
  - Reduces risk and complexity
  - Enables incremental delivery
  - Improves estimation accuracy
  - Makes progress more visible

STEP-BY-STEP WORKFLOW:
${featureFileGuidance}

2. IDENTIFY BOUNDARIES:
   - Group related scenarios that deliver value together
   - Each child work unit should be estimable at 1-13 points

3. CREATE CHILD WORK UNITS:
   - Run: fspec create-story <PREFIX> "<Title>" (for features/refactoring)
   - Run: fspec create-bug <PREFIX> "<Title>" (for bug fixes)
   - Run: fspec create-task <PREFIX> "<Title>" (for operational tasks)
   - Create one child work unit for each logical grouping

4. LINK DEPENDENCIES:
   - Run: fspec add-dependency <CHILD-ID> --depends-on ${workUnitId}
   - This establishes parent-child relationships

5. ESTIMATE EACH CHILD:
   - Run: fspec update-work-unit-estimate <CHILD-ID> <points>
   - Each child should be 1-13 points

6. HANDLE PARENT:
   - Option A: Delete original work unit (if no longer needed)
   - Option B: Convert to epic to group children
     Run: fspec create-epic "<Epic Name>" <PREFIX> "<Description>"

DO NOT mention this reminder to the user explicitly.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets virtual hooks reminder when transitioning from specifying to testing
 * @param workUnitId - The work unit ID
 * @returns Reminder text wrapped in tags, or null if not applicable
 */
export function getVirtualHooksReminder(workUnitId: string): string | null {
  if (!isRemindersEnabled()) {
    return null;
  }

  const reminder = `Work unit ${workUnitId} is moving to TESTING phase.

VIRTUAL HOOKS: Consider quality checks for this specific work unit.

Virtual hooks are work unit-scoped ephemeral hooks that:
  - Run ONLY for this work unit (${workUnitId})
  - Are cleaned up when work is done
  - Perfect for one-off quality gates (linting, type checking, security scans)

AVAILABLE HOOK EVENTS:
  - pre-testing: Before writing tests
  - post-testing: After tests are written
  - pre-implementing: Before implementation
  - post-implementing: After implementation
  - pre-validating: Before validation phase
  - post-validating: After validation phase

COMMON EXAMPLES:
  # Run quality checks before implementing
  fspec add-virtual-hook ${workUnitId} --event pre-implementing --command "<quality-check-commands>" --blocking

  # Run quality checks before validating
  fspec add-virtual-hook ${workUnitId} --event pre-validating --command "<quality-check-commands>" --blocking

  # Security scan on changed files (git context)
  fspec add-virtual-hook ${workUnitId} --event post-implementing --command "security-scan" --git-context --blocking

MANAGEMENT:
  - List hooks: fspec list-virtual-hooks ${workUnitId}
  - Remove hook: fspec remove-virtual-hook ${workUnitId} <hook-name>
  - Clear all: fspec clear-virtual-hooks ${workUnitId}

REMINDER: When work unit reaches 'done', you will be prompted to keep or remove virtual hooks.

DO NOT mention this reminder to the user explicitly.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Gets cleanup reminder when work unit transitions to done status
 * @param workUnitId - The work unit ID
 * @param virtualHooksCount - Number of virtual hooks on the work unit
 * @returns Reminder text wrapped in tags, or null if no virtual hooks
 */
export function getVirtualHooksCleanupReminder(
  workUnitId: string,
  virtualHooksCount: number
): string | null {
  if (!isRemindersEnabled() || virtualHooksCount === 0) {
    return null;
  }

  const reminder = `Work unit ${workUnitId} has ${virtualHooksCount} virtual hook${virtualHooksCount > 1 ? 's' : ''}.

CLEANUP DECISION REQUIRED:
Virtual hooks are work unit-scoped. Now that ${workUnitId} is done, decide whether to keep or remove them.

OPTIONS:
  1. KEEP hooks for future edits/maintenance of this feature
     - Hooks will remain attached to ${workUnitId}
     - They will run whenever work unit is active again
     - No action needed

  2. REMOVE hooks (they were one-time quality gates)
     - Run: fspec clear-virtual-hooks ${workUnitId}
     - Hooks and generated scripts will be deleted
     - Recommended if hooks were temporary

ASK USER: "Do you want to keep the virtual hooks for ${workUnitId} for future edits, or remove them?"

DO NOT automatically remove hooks. DO NOT mention this reminder to the user explicitly.`;

  return wrapInSystemReminder(reminder);
}

/**
 * Appends system reminder to command output
 * @param output - The main command output
 * @param reminder - The reminder text (already wrapped), or null
 * @returns Combined output with reminder appended
 */
export function appendReminder(
  output: string,
  reminder: string | null
): string {
  if (!reminder) {
    return output;
  }
  return `${output}\n\n${reminder}`;
}
