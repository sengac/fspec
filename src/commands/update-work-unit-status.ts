import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import { glob } from 'tinyglobby';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

type WorkUnitStatus = 'backlog' | 'specifying' | 'testing' | 'implementing' | 'validating' | 'done' | 'blocked';

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
  done: [], // Done is final
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
  warnings?: string[];
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

  // Prevent moving from done
  if (currentStatus === 'done' && newStatus !== 'done') {
    throw new Error(
      `Cannot change status of completed work unit. Create a new work unit for additional work.`
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
  if (currentStatus !== newStatus && !isValidTransition(currentStatus, newStatus)) {
    const errorMessages = [];
    errorMessages.push(`Invalid state transition from '${currentStatus}' to '${newStatus}'.`);

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

  // Validate prerequisites for testing state
  if (newStatus === 'testing' && currentStatus === 'specifying') {
    // Check for unanswered questions (Example Mapping integration)
    if (workUnit.questions && workUnit.questions.length > 0) {
      const questionsList = workUnit.questions.map(q => `  - ${q}`).join('\n');
      throw new Error(
        `Unanswered questions prevent state transition from '${currentStatus}' to '${newStatus}':\n${questionsList}\n\nAnswer questions with 'fspec answer-question ${options.workUnitId} <index>' before moving to testing.`
      );
    }

    // Warn if no examples (Example Mapping integration)
    if (!workUnit.examples || workUnit.examples.length === 0) {
      warnings.push(
        'No examples captured in Example Mapping. Consider adding examples with \'fspec add-example\' before testing.'
      );
    }

    // Check if scenarios exist
    const hasScenarios = await checkScenariosExist(options.workUnitId, cwd);
    if (!hasScenarios) {
      throw new Error(
        `No Gherkin scenarios found for work unit ${options.workUnitId}. At least one scenario must be tagged with @${options.workUnitId}. Use 'fspec generate-scenarios ${options.workUnitId}' or manually tag scenarios.`
      );
    }

    // Warn if no estimate
    if (!workUnit.estimate) {
      warnings.push('No estimate assigned. Consider adding estimate with --estimate=<points>');
    }

    // Warn about soft dependencies (dependsOn) that aren't done
    if (workUnit.dependsOn && workUnit.dependsOn.length > 0) {
      const incompleteDeps = workUnit.dependsOn.filter(depId => {
        const dep = workUnitsData.workUnits[depId];
        return dep && dep.status !== 'done';
      });

      if (incompleteDeps.length > 0) {
        const depDetails = incompleteDeps
          .map(id => `${id} (status: ${workUnitsData.workUnits[id]?.status || 'unknown'})`)
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
  }

  // Remove from current state array
  if (workUnitsData.states[currentStatus]) {
    workUnitsData.states[currentStatus] = workUnitsData.states[currentStatus].filter(
      id => id !== options.workUnitId
    );
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
    ...(newStatus === 'blocked' && options.blockedReason && { reason: options.blockedReason }),
  });

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  return {
    success: true,
    ...(warnings.length > 0 && { warnings }),
  };
}

function isValidTransition(from: WorkUnitStatus, to: WorkUnitStatus): boolean {
  if (from === to) {
    return true; // Same state is valid
  }
  return STATE_TRANSITIONS[from].includes(to);
}

async function checkScenariosExist(workUnitId: string, cwd: string): Promise<boolean> {
  try {
    const featureFiles = await glob(['spec/features/**/*.feature'], { cwd, absolute: true });

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
