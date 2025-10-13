import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { glob } from 'tinyglobby';
import type { WorkUnitsData, QuestionItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import {
  getStatusChangeReminder,
  type WorkflowState,
} from '../utils/system-reminder';

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
  specifying: ['testing', 'blocked'],
  testing: ['implementing', 'blocked'],
  implementing: ['validating', 'blocked'],
  validating: ['done', 'implementing', 'specifying', 'blocked'], // Can go back for fixes
  done: ['specifying', 'testing', 'implementing', 'validating', 'blocked'], // Can move backward when mistakes discovered
  blocked: ['backlog', 'specifying', 'testing', 'implementing', 'validating'], // Can return to previous state
};

interface UpdateWorkUnitStatusOptions {
  workUnitId: string;
  status: WorkUnitStatus;
  blockedReason?: string;
  reason?: string;
  cwd?: string;
}

interface UpdateWorkUnitStatusResult {
  success: boolean;
  message?: string;
  warnings?: string[];
  systemReminder?: string;
}

export async function updateWorkUnitStatus(
  options: UpdateWorkUnitStatusOptions
): Promise<UpdateWorkUnitStatusResult> {
  const cwd = options.cwd || process.cwd();
  const warnings: string[] = [];

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];
  const currentStatus = workUnit.status;
  const newStatus = options.status;

  // Validate state is allowed
  if (!ALLOWED_STATES.includes(newStatus)) {
    throw new Error(
      `Invalid status value: ${newStatus}. Allowed values: ${ALLOWED_STATES.join(', ')}`
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

  // Validate state transitions (ACDD enforcement)
  if (
    currentStatus !== newStatus &&
    !isValidTransition(currentStatus, newStatus)
  ) {
    const errorMessages = [];
    errorMessages.push(
      `Invalid state transition from '${currentStatus}' to '${newStatus}'.`
    );

    // Specific ACDD violation messages
    if (currentStatus === 'backlog' && newStatus === 'testing') {
      errorMessages.push(`Must move to 'specifying' state first.`);
      errorMessages.push(`ACDD requires specification before testing.`);
    } else if (currentStatus === 'specifying' && newStatus === 'implementing') {
      errorMessages.push(`Must move to 'testing' state first.`);
      errorMessages.push(`ACDD requires tests before implementation.`);
    }

    throw new Error(errorMessages.join(' '));
  }

  // Check for prefill in linked feature files (blocks ALL forward transitions except to blocked)
  if (newStatus !== 'blocked' && currentStatus !== newStatus) {
    const { checkWorkUnitFeatureForPrefill } = await import(
      '../utils/prefill-detection'
    );
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
    // Check for unanswered questions (Example Mapping integration)
    if (workUnit.questions && workUnit.questions.length > 0) {
      // Filter for unselected questions
      const unansweredQuestions = workUnit.questions.filter(q => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        return !q.selected;
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

  // Remove from current state array
  if (workUnitsData.states[currentStatus]) {
    workUnitsData.states[currentStatus] = workUnitsData.states[
      currentStatus
    ].filter(id => id !== options.workUnitId);
  }

  // Add to new state array
  if (!workUnitsData.states[newStatus]) {
    workUnitsData.states[newStatus] = [];
  }
  if (!workUnitsData.states[newStatus].includes(options.workUnitId)) {
    workUnitsData.states[newStatus].push(options.workUnitId);
  }

  // Update work unit status
  workUnit.status = newStatus;
  workUnit.updatedAt = new Date().toISOString();

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

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  // Get system reminder for the new status
  const systemReminder = getStatusChangeReminder(
    options.workUnitId,
    newStatus as WorkflowState
  );

  return {
    success: true,
    message: `✓ Work unit ${options.workUnitId} status updated to ${newStatus}`,
    ...(warnings.length > 0 && { warnings }),
    ...(systemReminder && { systemReminder }),
  };
}

function isValidTransition(from: WorkUnitStatus, to: WorkUnitStatus): boolean {
  if (from === to) {
    return true; // Same state is valid
  }
  return STATE_TRANSITIONS[from].includes(to);
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
    const { existsSync } = await import('fs');
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
