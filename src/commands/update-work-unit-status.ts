import { readFile } from 'fs/promises';
import { fileManager } from '../utils/file-manager';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import { glob } from 'tinyglobby';
import type { WorkUnitsData, QuestionItem, WorkUnitType } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import {
  getStatusChangeReminder,
  getVirtualHooksReminder,
  getVirtualHooksCleanupReminder,
  type WorkflowState,
} from '../utils/system-reminder';
import { checkWorkUnitFeatureForPrefill } from '../utils/prefill-detection';
import {
  findStateHistoryEntry,
  checkFileCreatedAfter,
} from '../utils/temporal-validation';
import * as gitCheckpoint from '../utils/git-checkpoint';
import { existsSync } from 'fs';
import { checkTestCommand, checkQualityCommands } from './configure-tools.js';
import {
  insertWorkUnitSorted,
  compareByUpdatedDescending,
} from '../utils/states-array';
import { compactWorkUnit } from './compact-work-unit';
import { validateSteps, formatValidationError } from '../utils/step-validation';
import { parseAllFeatures } from '../utils/feature-parser';

type WorkUnitStatus =
  | 'backlog'
  | 'specifying'
  | 'testing'
  | 'implementing'
  | 'validating'
  | 'done'
  | 'blocked';

const ALLOWED_STATES: WorkUnitStatus[] = [
  'backlog',
  'specifying',
  'testing',
  'implementing',
  'validating',
  'done',
  'blocked',
];

const STATE_TRANSITIONS: Record<WorkUnitStatus, WorkUnitStatus[]> = {
  backlog: ['specifying', 'blocked'],
  specifying: ['testing', 'blocked'], // Cannot go back to backlog (prevented by separate check)
  testing: ['implementing', 'specifying', 'blocked'], // Can go back to specifying
  implementing: ['validating', 'testing', 'specifying', 'blocked'], // Can go back to testing or specifying
  validating: ['done', 'implementing', 'testing', 'specifying', 'blocked'], // Can go back for fixes
  done: ['specifying', 'testing', 'implementing', 'validating', 'blocked'], // Can move backward when mistakes discovered
  blocked: ['backlog', 'specifying', 'testing', 'implementing', 'validating'], // Can return to previous state
};

interface UpdateWorkUnitStatusOptions {
  workUnitId: string;
  status: WorkUnitStatus;
  blockedReason?: string;
  reason?: string;
  skipTemporalValidation?: boolean;
  cwd?: string;
}

interface UpdateWorkUnitStatusResult {
  success: boolean;
  message?: string;
  warnings?: string[];
  systemReminder?: string;
  output?: string;
  checkpointCreated?: boolean;
  checkpointName?: string;
  newStatus?: string;
  stashRef?: string;
  capturedFiles?: string[];
}

export async function updateWorkUnitStatus(
  options: UpdateWorkUnitStatusOptions
): Promise<UpdateWorkUnitStatusResult> {
  const cwd = options.cwd || process.cwd();
  const warnings: string[] = [];

  // Read work units
  let workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  let workUnit = workUnitsData.workUnits[options.workUnitId];
  const currentStatus = workUnit.status;
  const newStatus = options.status;
  const workUnitType: WorkUnitType = workUnit.type || 'story'; // Default to 'story' for backward compatibility

  // Validate state is allowed
  if (!ALLOWED_STATES.includes(newStatus)) {
    throw new Error(
      `Invalid status value: ${newStatus}. Allowed values: ${ALLOWED_STATES.join(', ')}`
    );
  }

  // Validate type-specific workflow constraints
  if (workUnitType === 'task' && newStatus === 'testing') {
    throw new Error(
      `Tasks do not have a testing phase. task workflow: backlog → specifying → implementing → validating → done.\n` +
        `Tasks are for operational work without testable acceptance criteria.\n` +
        `Use stories for user-facing features that require tests.`
    );
  }

  // Prevent moving to backlog
  if (newStatus === 'backlog' && currentStatus !== 'backlog') {
    throw new Error(
      `Cannot move work back to backlog. Use 'blocked' state if work cannot progress.`
    );
  }

  // Validate blocked state requires reason
  if (newStatus === 'blocked' && !options.blockedReason) {
    throw new Error(
      `Blocked reason is required when moving to blocked state. Use --blocked-reason='description of blocker'`
    );
  }

  // Validate state transitions (ACDD enforcement with type-specific rules)
  if (currentStatus !== newStatus) {
    // Special case for tasks: allow specifying → implementing (skip testing)
    const isTaskSkippingTest =
      workUnitType === 'task' &&
      currentStatus === 'specifying' &&
      newStatus === 'implementing';

    if (!isTaskSkippingTest && !isValidTransition(currentStatus, newStatus)) {
      const errorMessages = [];
      errorMessages.push(
        `Invalid state transition from '${currentStatus}' to '${newStatus}'.`
      );

      // Specific ACDD violation messages
      if (currentStatus === 'backlog' && newStatus === 'testing') {
        errorMessages.push(`Must move to 'specifying' state first.`);
        errorMessages.push(`ACDD requires specification before testing.`);
      } else if (
        currentStatus === 'specifying' &&
        newStatus === 'implementing' &&
        workUnitType !== 'task'
      ) {
        errorMessages.push(`Must move to 'testing' state first.`);
        errorMessages.push(`ACDD requires tests before implementation.`);
        errorMessages.push(
          `Note: Only tasks can skip testing. Use --type=task for operational work.`
        );
      }

      throw new Error(errorMessages.join(' '));
    }
  }

  // Prevent starting work that is blocked by incomplete dependencies
  const activeStates = ['specifying', 'testing', 'implementing', 'validating'];
  if (newStatus !== 'blocked' && activeStates.includes(newStatus)) {
    const blockedBy = workUnit.blockedBy || [];
    const activeBlockers: string[] = [];

    for (const blockerId of blockedBy) {
      const blockerUnit = workUnitsData.workUnits[blockerId];
      if (blockerUnit && blockerUnit.status !== 'done') {
        activeBlockers.push(
          `${blockerId} (status: ${blockerUnit.status || 'unknown'})`
        );
      }
    }

    if (activeBlockers.length > 0) {
      throw new Error(
        `Cannot start work on ${options.workUnitId}: work unit is blocked by incomplete dependencies.\n\n` +
          `Active blockers:\n  - ${activeBlockers.join('\n  - ')}\n\n` +
          `Complete blocking work units or remove dependencies before starting work.`
      );
    }
  }

  // Check for prefill in linked feature files (blocks ALL forward transitions except to blocked)
  if (newStatus !== 'blocked' && currentStatus !== newStatus) {
    const prefillResult = await checkWorkUnitFeatureForPrefill(
      options.workUnitId,
      cwd
    );

    if (prefillResult && prefillResult.hasPrefill) {
      const matchDetails = prefillResult.matches
        .slice(0, 3)
        .map(m => `  Line ${m.line}: ${m.pattern}`)
        .join('\n');

      throw new Error(
        `Cannot advance work unit status: linked feature file contains prefill placeholders.\n\n` +
          `Found ${prefillResult.matches.length} placeholder(s):\n${matchDetails}\n` +
          (prefillResult.matches.length > 3
            ? `  ... and ${prefillResult.matches.length - 3} more\n\n`
            : '\n') +
          `Fix prefill using CLI commands:\n` +
          `  - fspec set-user-story ${options.workUnitId} --role='...' --action='...' --benefit='...'\n` +
          `  - fspec add-step <feature> <scenario> <keyword> <text>\n` +
          `  - fspec add-tag-to-feature <file> <tag>\n` +
          `  - fspec add-architecture <feature> <text>\n\n` +
          `DO NOT use Write or Edit tools to replace prefill directly.`
      );
    }
  }

  // Validate prerequisites for testing state
  if (newStatus === 'testing' && currentStatus === 'specifying') {
    // Type-specific validation for bugs: must link to existing feature file
    // Bugs can use either linkedFeatures array or @WORK-UNIT-ID tags in scenarios
    if (workUnitType === 'bug') {
      const linkedFeatures = workUnit.linkedFeatures || [];
      const hasLinkedFeatures = linkedFeatures.length > 0;

      // Check for scenarios if no linkedFeatures
      let hasScenarios = false;
      if (!hasLinkedFeatures) {
        hasScenarios = await checkScenariosExist(options.workUnitId, cwd);
      }

      if (!hasLinkedFeatures && !hasScenarios) {
        throw new Error(
          `Bugs must link to existing feature file before moving to testing.\n\n` +
            `Use: fspec link-feature ${options.workUnitId} <feature-name>\n` +
            `Or tag scenarios in feature files with @${options.workUnitId}\n\n` +
            `If the feature has no spec, create a story instead of a bug.`
        );
      }
    }

    // Check for unanswered questions (Example Mapping integration)
    if (workUnit.questions && workUnit.questions.length > 0) {
      // Filter for unselected questions (BUG-060: exclude deleted questions)
      const unansweredQuestions = workUnit.questions.filter(q => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        const questionItem = q as QuestionItem;
        return !questionItem.deleted && !questionItem.selected;
      });

      if (unansweredQuestions.length > 0) {
        const questionsList = unansweredQuestions
          .map(q => {
            const questionItem = q as QuestionItem;
            // Find the original index in the full array
            const originalIndex = workUnit.questions!.indexOf(q);
            return `  - [${originalIndex}] ${questionItem.text}`;
          })
          .join('\n');

        throw new Error(
          `Unanswered questions prevent state transition from '${currentStatus}' to '${newStatus}':\n${questionsList}\n\nAnswer questions with 'fspec answer-question ${options.workUnitId} <index>' before moving to testing.`
        );
      }
    }

    // Warn if no examples (Example Mapping integration)
    if (!workUnit.examples || workUnit.examples.length === 0) {
      warnings.push(
        "No examples captured in Example Mapping. Consider adding examples with 'fspec add-example' before testing."
      );
    }

    // Check if scenarios exist (skip for parent work units with children)
    const isParentWorkUnit = workUnit.children && workUnit.children.length > 0;
    if (!isParentWorkUnit) {
      const hasScenarios = await checkScenariosExist(options.workUnitId, cwd);
      if (!hasScenarios) {
        throw new Error(
          `No Gherkin scenarios found for work unit ${options.workUnitId}. At least one scenario must be tagged with @${options.workUnitId}. Use 'fspec generate-scenarios ${options.workUnitId}' or manually tag scenarios.`
        );
      }
    }

    // FEAT-011: Temporal validation - ensure feature file was created AFTER entering specifying
    if (!options.skipTemporalValidation) {
      const specifyingEntry = findStateHistoryEntry(workUnit, 'specifying');
      if (specifyingEntry) {
        await checkFileCreatedAfter(
          options.workUnitId,
          specifyingEntry.timestamp,
          'feature',
          cwd
        );
      }
    }

    // Warn if no estimate
    if (!workUnit.estimate) {
      warnings.push(
        'No estimate assigned. Consider adding estimate with --estimate=<points>'
      );
    }

    // Warn about soft dependencies (dependsOn) that aren't done
    if (workUnit.dependsOn && workUnit.dependsOn.length > 0) {
      const incompleteDeps = workUnit.dependsOn.filter(depId => {
        const dep = workUnitsData.workUnits[depId];
        return dep && dep.status !== 'done';
      });

      if (incompleteDeps.length > 0) {
        const depDetails = incompleteDeps
          .map(
            id =>
              `${id} (status: ${workUnitsData.workUnits[id]?.status || 'unknown'})`
          )
          .join(', ');
        warnings.push(
          `Work unit has soft dependencies that are not complete: ${depDetails}. Consider completing dependencies first for better workflow.`
        );
      }
    }
  }

  // FEAT-011: Temporal validation for implementing state
  // Ensure tests were created AFTER entering testing state
  if (
    newStatus === 'implementing' &&
    currentStatus === 'testing' &&
    !options.skipTemporalValidation &&
    workUnitType !== 'task' // Tasks don't have tests
  ) {
    const testingEntry = findStateHistoryEntry(workUnit, 'testing');
    if (testingEntry) {
      await checkFileCreatedAfter(
        options.workUnitId,
        testingEntry.timestamp,
        'test',
        cwd
      );
    }
  }

  // BUG-061: Step validation for implementing and validating states
  // Ensure test files have complete Given/When/Then @step docstrings
  if (
    (newStatus === 'implementing' || newStatus === 'validating') &&
    workUnitType !== 'task' // Tasks are exempt from step validation
  ) {
    await validateTestStepDocstrings(options.workUnitId, workUnitType, cwd);
  }

  // Validate parent/child constraints for done state
  if (newStatus === 'done') {
    if (workUnit.children && workUnit.children.length > 0) {
      const incompleteChildren = workUnit.children.filter(childId => {
        const child = workUnitsData.workUnits[childId];
        return child && child.status !== 'done';
      });

      if (incompleteChildren.length > 0) {
        const childDetails = incompleteChildren
          .map(id => `${id} (status: ${workUnitsData.workUnits[id].status})`)
          .join(', ');
        throw new Error(
          `Cannot mark parent as done while children are incomplete: ${childDetails}. Complete all children first.`
        );
      }
    }

    // Check coverage completeness when moving to done (COV-006)
    const coverageCheck = await checkCoverageCompleteness(workUnit, cwd);
    if (!coverageCheck.complete) {
      return {
        success: false,
        message: coverageCheck.message,
        systemReminder: coverageCheck.systemReminder,
      };
    }

    // Add warnings if coverage file doesn't exist
    if (coverageCheck.warning) {
      warnings.push(coverageCheck.warning);
    }
  }

  // Create automatic checkpoint before state transition (GIT-002)
  let checkpointCreated = false;
  let checkpointName = '';

  try {
    const isDirty = await gitCheckpoint.isWorkingDirectoryDirty(cwd);

    // Explicit boolean assignment prevents Vite optimizer from breaking execution flow
    // Without this, optimizer transforms: 'const isDirty = await ...; if (isDirty && ...)'
    // Into broken code: 'await Zn(t) && i !== "backlog" && (...)'
    const shouldCreateCheckpoint = isDirty && currentStatus !== 'backlog';

    if (shouldCreateCheckpoint) {
      checkpointName = gitCheckpoint.createAutomaticCheckpointName(
        options.workUnitId,
        currentStatus
      );
      await gitCheckpoint.createCheckpoint({
        workUnitId: options.workUnitId,
        checkpointName,
        cwd,
        includeUntracked: true,
      });
      checkpointCreated = true;
      console.log(
        chalk.gray(
          `🤖 Auto-checkpoint: "${checkpointName}" created before transition`
        )
      );
    }
  } catch (error: unknown) {
    // Silently skip checkpoint creation if git operations fail
    // This allows commands to work even without git repository
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (process.env.DEBUG) {
      console.warn(chalk.yellow(`⚠️  Checkpoint skipped: ${errorMessage}`));
    }
  }

  // BOARD-016: Use shared utility for states array manipulation
  // Done column uses sorted insertion (by 'updated' timestamp, most recent first)
  // IMPORTANT: Must do sorting BEFORE setting new 'updated' timestamp
  // If moving to done, use existing 'updated' field for sort position (or current time if not set)
  // Then update the timestamp after sorting

  // Set temporary 'updated' field if moving to done and not already set (for sorting)
  const hadUpdatedField = !!workUnit.updated;
  if (newStatus === 'done' && !workUnit.updated) {
    workUnit.updated = new Date().toISOString();
  }

  const updatedWorkUnitsData =
    newStatus === 'done'
      ? insertWorkUnitSorted(
          workUnitsData,
          options.workUnitId,
          currentStatus,
          newStatus,
          compareByUpdatedDescending
        )
      : insertWorkUnitSorted(
          workUnitsData,
          options.workUnitId,
          currentStatus,
          newStatus
          // No comparator - appends to end
        );

  // BUG FIX (IDX-002): Auto-compact when moving to done status
  // CRITICAL TEMPORAL COUPLING: This must happen BEFORE applying state sorting
  // because compactWorkUnit() saves to disk and we re-read from disk, which would
  // lose the sorted state if we sorted first.
  //
  // Sequence:
  // 1. Calculate sorted states (insertWorkUnitSorted above)
  // 2. Auto-compact (saves to disk)
  // 3. Re-read from disk (loses sorted states)
  // 4. Apply sorted states (line 480) ← MUST be after re-read!
  if (newStatus === 'done') {
    await compactWorkUnit({
      workUnitId: options.workUnitId,
      force: true,
      cwd,
    });

    // Re-read work units data after compaction (compactWorkUnit saves to disk)
    workUnitsData = await ensureWorkUnitsFile(cwd);
    workUnit = workUnitsData.workUnits[options.workUnitId];

    if (!workUnit) {
      throw new Error(`Work unit '${options.workUnitId}' does not exist`);
    }
  }

  // BUG FIX (IDX-002): Update workUnitsData reference to use sorted result
  // CRITICAL: This MUST happen AFTER auto-compact (not before)
  // Reason: compactWorkUnit() re-reads from disk, which would overwrite sorted states
  // if we applied them before compaction. By applying AFTER, we preserve user-defined
  // sort orders through the auto-compact operation.
  workUnitsData.states = updatedWorkUnitsData.states;

  // Update work unit status
  workUnit.status = newStatus;
  workUnit.updatedAt = new Date().toISOString();

  // BOARD-016: Set 'updated' field to current timestamp when moving TO done
  // This happens AFTER sorting (which used the old 'updated' value if it existed)
  if (newStatus === 'done') {
    const currentTimestamp = new Date().toISOString();
    workUnit.updated = currentTimestamp;

    // BOARD-016: Sort ENTIRE done array by 'updated' timestamp (most recent first)
    // This fixes any existing mis-ordering in the done column
    const doneArray = workUnitsData.states.done || [];

    const sortedDoneArray = [...doneArray].sort((aId, bId) => {
      return compareByUpdatedDescending(aId, bId, workUnitsData.workUnits);
    });

    workUnitsData.states.done = sortedDoneArray;
  }

  // Update blocked reason
  if (newStatus === 'blocked' && options.blockedReason) {
    workUnit.blockedReason = options.blockedReason;
  } else if (newStatus !== 'blocked' && workUnit.blockedReason) {
    delete workUnit.blockedReason;
  }

  // Update state history
  if (!workUnit.stateHistory) {
    workUnit.stateHistory = [];
  }
  workUnit.stateHistory.push({
    state: newStatus,
    timestamp: new Date().toISOString(),
    ...(options.reason && { reason: options.reason }),
    ...(newStatus === 'blocked' &&
      options.blockedReason && { reason: options.blockedReason }),
  });

  // LOCK-002: Use fileManager.transaction() for atomic write
  // Note: All modifications above already happened to workUnitsData in memory
  // Now we need to write them atomically
  await fileManager.transaction(workUnitsFile, async data => {
    // Copy all modifications into the locked data
    Object.assign(data, workUnitsData);
  });

  // Collect all system reminders
  const reminders: string[] = [];

  // Get status change reminder
  const statusReminder = getStatusChangeReminder(
    options.workUnitId,
    newStatus as WorkflowState
  );
  if (statusReminder) {
    reminders.push(statusReminder);
  }

  // Get virtual hooks reminder when transitioning from specifying → testing
  if (currentStatus === 'specifying' && newStatus === 'testing') {
    const virtualHooksReminder = getVirtualHooksReminder(options.workUnitId);
    if (virtualHooksReminder) {
      reminders.push(virtualHooksReminder);
    }
  }

  // Get cleanup reminder when transitioning to done
  if (newStatus === 'done') {
    const virtualHooksCount = workUnit.virtualHooks?.length || 0;
    const cleanupReminder = getVirtualHooksCleanupReminder(
      options.workUnitId,
      virtualHooksCount
    );
    if (cleanupReminder) {
      reminders.push(cleanupReminder);
    }

    // Add review suggestion reminder (only for story and bug work units)
    if (workUnit.type === 'story' || workUnit.type === 'bug') {
      const reviewReminder = `<system-reminder>
QUALITY CHECK OPPORTUNITY

Work unit ${options.workUnitId} is being marked as done.

Would you like me to run fspec review ${options.workUnitId} for a quality review before finalizing?

Suggested workflow:
  1. Run: fspec review ${options.workUnitId}
  2. If findings exist: address findings and fix any issues
  3. If no findings (or all fixed): then mark done

If yes: Run the quality review and address findings before marking done
If no: Proceed with marking done

This is optional but recommended to catch issues early.
</system-reminder>`;
      reminders.push(reviewReminder);
    }
  }

  // Combine all reminders
  const systemReminder =
    reminders.length > 0 ? reminders.join('\n\n') : undefined;

  // Check for tool configuration when moving to validating state
  if (newStatus === 'validating') {
    const testCheck = await checkTestCommand(cwd);
    console.log(testCheck.message);

    const qualityCheck = await checkQualityCommands(cwd);
    console.log(qualityCheck.message);
  }

  // Build output string
  const outputParts: string[] = [];
  outputParts.push(
    `✓ Work unit ${options.workUnitId} status updated to ${newStatus}`
  );
  if (systemReminder) {
    outputParts.push(systemReminder);
  }
  const output = outputParts.join('\n\n');

  return {
    success: true,
    message: `✓ Work unit ${options.workUnitId} status updated to ${newStatus}`,
    output,
    ...(warnings.length > 0 && { warnings }),
    ...(systemReminder && { systemReminder }),
    checkpointCreated,
    ...(checkpointCreated && { checkpointName }),
    newStatus,
    ...(checkpointCreated && { stashRef: `stash@{0}` }),
  };
}

function isValidTransition(from: WorkUnitStatus, to: WorkUnitStatus): boolean {
  if (from === to) {
    return true; // Same state is valid
  }
  return STATE_TRANSITIONS[from].includes(to);
}

/**
 * Validate that test files have complete Given/When/Then @step docstrings
 *
 * BUG-061: Ensures test-to-scenario traceability through step comments
 *
 * @param workUnitId - Work unit ID
 * @param workUnitType - Work unit type (story, bug, task)
 * @param cwd - Working directory
 * @throws Error if step validation fails
 */
async function validateTestStepDocstrings(
  workUnitId: string,
  workUnitType: WorkUnitType,
  cwd: string
): Promise<void> {
  // Find feature files tagged with this work unit ID
  const parsedFeatures = await parseAllFeatures(cwd);
  const matchingFeatures = parsedFeatures.filter(
    f => f.workUnitId === workUnitId
  );

  if (matchingFeatures.length === 0) {
    // No feature files found - this is OK, might be using linkedFeatures array
    return;
  }

  // BUG-061: Read coverage files to find test files (language-agnostic)
  const workUnitTestFiles = new Set<string>();

  for (const feature of matchingFeatures) {
    // Coverage file path: <feature-file-path>.coverage
    const coverageFilePath = join(cwd, feature.filePath + '.coverage');

    if (!existsSync(coverageFilePath)) {
      throw new Error(
        `No coverage file found for feature ${feature.filePath}.\n\n` +
          `Coverage file expected at: ${feature.filePath}.coverage\n\n` +
          `Test files must be linked using coverage files.\n` +
          `Use: fspec link-coverage <feature> --scenario "<name>" --test-file <path> --test-lines <range>\n\n` +
          `Before moving to implementing or validating, tests must be written and linked.`
      );
    }

    // Read coverage file and extract test files
    const coverageContent = await readFile(coverageFilePath, 'utf-8');
    let coverage;
    try {
      coverage = JSON.parse(coverageContent);
    } catch (error: unknown) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      throw new Error(
        `Malformed coverage file: ${coverageFilePath}\n` +
          `JSON parse error: ${errorMessage}\n\n` +
          `Please check the coverage file for syntax errors.`
      );
    }

    if (coverage.scenarios && Array.isArray(coverage.scenarios)) {
      for (const scenario of coverage.scenarios) {
        if (scenario.testMappings && Array.isArray(scenario.testMappings)) {
          for (const mapping of scenario.testMappings) {
            if (mapping.file) {
              workUnitTestFiles.add(mapping.file);
            }
          }
        }
      }
    }
  }

  if (workUnitTestFiles.size === 0) {
    throw new Error(
      `No test files found in coverage files for work unit ${workUnitId}.\n\n` +
        `Coverage files exist but contain no test mappings.\n` +
        `Use: fspec link-coverage <feature> --scenario "<name>" --test-file <path> --test-lines <range>\n\n` +
        `Before moving to implementing or validating, tests must be written and linked.`
    );
  }

  // Validate each test file
  const validationErrors: string[] = [];

  for (const testFilePath of workUnitTestFiles) {
    const absoluteTestPath = join(cwd, testFilePath);

    try {
      const testContent = await readFile(absoluteTestPath, 'utf-8');

      // Validate test file is not empty
      if (!testContent || testContent.trim().length === 0) {
        throw new Error(
          `Test file is empty: ${testFilePath}\n\n` +
            `Test files must contain step docstrings matching feature scenarios.`
        );
      }

      // For each feature, validate all scenario steps
      for (const feature of matchingFeatures) {
        for (const scenario of feature.scenarios) {
          const featureSteps = scenario.steps.map(
            s => `${s.keyword} ${s.text}`
          );

          const validationResult = validateSteps(featureSteps, testContent);

          if (!validationResult.valid) {
            const errorMessage = formatValidationError(
              validationResult,
              workUnitType
            );
            validationErrors.push(
              `Test file: ${testFilePath}\n` +
                `Scenario: ${scenario.name}\n` +
                errorMessage
            );
          }
        }
      }
    } catch (error: unknown) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      throw new Error(
        `Failed to read test file ${testFilePath}: ${errorMessage}`
      );
    }
  }

  if (validationErrors.length > 0) {
    // Show all validation errors, not just the first one
    const errorSummary =
      validationErrors.length === 1
        ? validationErrors[0]
        : `Multiple validation errors found (${validationErrors.length} scenarios):\n\n` +
          validationErrors.join('\n\n---\n\n');
    throw new Error(errorSummary);
  }
}

async function checkScenariosExist(
  workUnitId: string,
  cwd: string
): Promise<boolean> {
  try {
    const featureFiles = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: true,
    });

    for (const file of featureFiles) {
      const content = await readFile(file, 'utf-8');
      // Check if file contains @WORK-UNIT-ID tag
      if (content.includes(`@${workUnitId}`)) {
        return true;
      }
    }

    return false;
  } catch {
    return false;
  }
}

interface CoverageCheckResult {
  complete: boolean;
  message?: string;
  systemReminder?: string;
  warning?: string;
}

async function checkCoverageCompleteness(
  workUnit: any,
  cwd: string
): Promise<CoverageCheckResult> {
  // Get linked features
  const linkedFeatures = workUnit.linkedFeatures || [];

  if (linkedFeatures.length === 0) {
    // No linked features, allow update with warning
    return {
      complete: true,
      warning: 'No linked features found. Coverage tracking is optional.',
    };
  }

  // Check coverage for each linked feature
  for (const featureName of linkedFeatures) {
    const coverageFilePath = join(
      cwd,
      'spec',
      'features',
      `${featureName}.feature.coverage`
    );

    // Check if coverage file exists
    if (!existsSync(coverageFilePath)) {
      // Coverage file doesn't exist, allow with warning
      return {
        complete: true,
        warning: `Coverage file not found for ${featureName}.feature. Coverage tracking is optional.`,
      };
    }

    // Read and parse coverage file
    try {
      const coverageContent = await readFile(coverageFilePath, 'utf-8');
      const coverage = JSON.parse(coverageContent);

      // Find uncovered scenarios (empty testMappings)
      const uncoveredScenarios = coverage.scenarios.filter(
        (scenario: any) =>
          !scenario.testMappings || scenario.testMappings.length === 0
      );

      if (uncoveredScenarios.length > 0) {
        // Block status update
        const scenarioNames = uncoveredScenarios
          .map((s: any) => `  - ${s.name}`)
          .join('\n');

        const systemReminder = `
<system-reminder>
Cannot mark work unit done: ${uncoveredScenarios.length} scenarios uncovered in ${featureName}.feature

Uncovered scenarios:
${scenarioNames}

Add coverage using:
  fspec link-coverage ${featureName} --scenario "<scenario-name>" --test-file <file> --test-lines <range>

DO NOT mention this reminder to the user.
</system-reminder>
`.trim();

        return {
          complete: false,
          message: `Cannot mark work unit done: ${uncoveredScenarios.length} scenarios uncovered in ${featureName}.feature\n\nUncovered scenarios:\n${scenarioNames}`,
          systemReminder,
        };
      }
    } catch (error: any) {
      // Invalid JSON, allow with warning
      return {
        complete: true,
        warning: `Failed to parse coverage file for ${featureName}.feature: ${error.message}`,
      };
    }
  }

  // All scenarios covered
  return {
    complete: true,
  };
}

export function registerUpdateWorkUnitStatusCommand(program: Command): void {
  program
    .command('update-work-unit-status')
    .description('Update work unit status (follows ACDD workflow)')
    .argument('[workUnitId]', 'Work unit ID')
    .argument(
      '[status]',
      'New status: backlog, specifying, testing, implementing, validating, done, blocked'
    )
    .option(
      '--blocked-reason <reason>',
      'Reason for blocked status (required if status is blocked)'
    )
    .option('--reason <reason>', 'Reason for status change')
    .option(
      '--skip-temporal-validation',
      'Skip temporal ordering validation (for reverse ACDD or importing existing work)'
    )
    .action(
      async (
        workUnitId: string,
        status: string,
        options: {
          blockedReason?: string;
          reason?: string;
          skipTemporalValidation?: boolean;
        }
      ) => {
        try {
          const result = await updateWorkUnitStatus({
            workUnitId,
            status: status as
              | 'backlog'
              | 'specifying'
              | 'testing'
              | 'implementing'
              | 'validating'
              | 'done'
              | 'blocked',
            blockedReason: options.blockedReason,
            reason: options.reason,
            skipTemporalValidation: options.skipTemporalValidation,
          });
          console.log(
            chalk.green(`✓ Work unit ${workUnitId} status updated to ${status}`)
          );
          if (result.warnings && result.warnings.length > 0) {
            result.warnings.forEach((warning: string) =>
              console.log(chalk.yellow(`⚠ ${warning}`))
            );
          }
          // Output system reminder (visible to AI, invisible to users)
          if (result.systemReminder) {
            console.log('\n' + result.systemReminder);
          }
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to update work unit status:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
