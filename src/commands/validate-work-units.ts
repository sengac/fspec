import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface ValidateWorkUnitsOptions {
  cwd?: string;
}

interface ValidateWorkUnitsResult {
  valid: boolean;
  checks: string[];
  errors?: string[];
}

export async function validateWorkUnits(options: ValidateWorkUnitsOptions = {}): Promise<ValidateWorkUnitsResult> {
  const cwd = options.cwd || process.cwd();
  const errors: string[] = [];
  const checks: string[] = [];

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Check 1: JSON schema compliance
  checks.push('schema');
  if (!workUnitsData.workUnits || typeof workUnitsData.workUnits !== 'object') {
    errors.push('Invalid work units data structure: missing or invalid workUnits field');
  }
  if (!workUnitsData.states || typeof workUnitsData.states !== 'object') {
    errors.push('Invalid work units data structure: missing or invalid states field');
  }

  // Check 2: Unique IDs
  checks.push('uniqueIds');
  const ids = Object.keys(workUnitsData.workUnits);
  const uniqueIds = new Set(ids);
  if (ids.length !== uniqueIds.size) {
    errors.push('Duplicate work unit IDs detected');
  }

  // Check 3: Parent/child consistency
  checks.push('parentChild');
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    // Check if parent exists
    if (workUnit.parent && !workUnitsData.workUnits[workUnit.parent]) {
      errors.push(`Work unit ${id} references non-existent parent: ${workUnit.parent}`);
    }

    // Check if parent has this work unit as a child
    if (workUnit.parent) {
      const parent = workUnitsData.workUnits[workUnit.parent];
      if (!parent.children || !parent.children.includes(id)) {
        errors.push(`Work unit ${id} has parent ${workUnit.parent}, but parent doesn't list it as a child`);
      }
    }

    // Check if children exist
    if (workUnit.children) {
      for (const childId of workUnit.children) {
        if (!workUnitsData.workUnits[childId]) {
          errors.push(`Work unit ${id} references non-existent child: ${childId}`);
        } else {
          // Check if child has this work unit as parent
          const child = workUnitsData.workUnits[childId];
          if (child.parent !== id) {
            errors.push(`Work unit ${id} lists ${childId} as child, but child doesn't have it as parent`);
          }
        }
      }
    }
  }

  // Check 4: Valid state values
  const allowedStates = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'];
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    if (!allowedStates.includes(workUnit.status)) {
      errors.push(
        `Invalid status value for ${id}: ${workUnit.status}. Allowed values: ${allowedStates.join(', ')}`
      );
    }
  }

  // Check 5: State index consistency
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    const status = workUnit.status;
    const stateArray = workUnitsData.states[status];

    if (!stateArray || !stateArray.includes(id)) {
      errors.push(
        `State consistency error: Work unit ${id} has status '${status}' but is not in states.${status} array. Run 'fspec repair-work-units' to fix inconsistencies.`
      );
    }

    // Check if work unit is in wrong state array
    for (const [stateName, ids] of Object.entries(workUnitsData.states)) {
      if (stateName !== status && ids.includes(id)) {
        errors.push(
          `State consistency error: Work unit ${id} has status '${status}' but is in '${stateName}' array. Run 'fspec repair-work-units' to fix inconsistencies.`
        );
      }
    }
  }

  // Check 6: Example mapping data validation
  checks.push('exampleMapping');
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    // Validate rules array
    if (workUnit.rules) {
      if (!Array.isArray(workUnit.rules)) {
        errors.push(`Work unit ${id}: rules must be an array`);
      } else {
        for (let i = 0; i < workUnit.rules.length; i++) {
          if (typeof workUnit.rules[i] !== 'string' || workUnit.rules[i].trim() === '') {
            errors.push(`Work unit ${id}: rules array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }

    // Validate examples array
    if (workUnit.examples) {
      if (!Array.isArray(workUnit.examples)) {
        errors.push(`Work unit ${id}: examples must be an array`);
      } else {
        for (let i = 0; i < workUnit.examples.length; i++) {
          if (typeof workUnit.examples[i] !== 'string' || workUnit.examples[i].trim() === '') {
            errors.push(`Work unit ${id}: examples array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }

    // Validate questions array (must be QuestionItem objects)
    if (workUnit.questions) {
      if (!Array.isArray(workUnit.questions)) {
        errors.push(`Work unit ${id}: questions must be an array`);
      } else {
        for (let i = 0; i < workUnit.questions.length; i++) {
          const question = workUnit.questions[i];

          // Must be an object (QuestionItem format)
          if (typeof question !== 'object' || question === null) {
            errors.push(`Work unit ${id}: questions[${i}] must be a QuestionItem object with {text, selected, answer?}, got ${typeof question}`);
            continue;
          }

          // Must have required fields
          if (typeof question.text !== 'string' || question.text.trim() === '') {
            errors.push(`Work unit ${id}: questions[${i}].text must be a non-empty string`);
          }

          if (typeof question.selected !== 'boolean') {
            errors.push(`Work unit ${id}: questions[${i}].selected must be a boolean`);
          }

          // Optional answer field
          if (question.answer !== undefined && (typeof question.answer !== 'string' || question.answer.trim() === '')) {
            errors.push(`Work unit ${id}: questions[${i}].answer must be a non-empty string if provided`);
          }
        }
      }
    }

    // Validate assumptions array
    if (workUnit.assumptions) {
      if (!Array.isArray(workUnit.assumptions)) {
        errors.push(`Work unit ${id}: assumptions must be an array`);
      } else {
        for (let i = 0; i < workUnit.assumptions.length; i++) {
          if (
            typeof workUnit.assumptions[i] !== 'string' ||
            workUnit.assumptions[i].trim() === ''
          ) {
            errors.push(`Work unit ${id}: assumptions array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }
  }

  // Check 7: Dependency data validation
  checks.push('dependencies');
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    // Validate blocks array
    if (workUnit.blocks) {
      if (!Array.isArray(workUnit.blocks)) {
        errors.push(`Work unit ${id}: blocks must be an array`);
      } else {
        for (let i = 0; i < workUnit.blocks.length; i++) {
          if (typeof workUnit.blocks[i] !== 'string' || workUnit.blocks[i].trim() === '') {
            errors.push(`Work unit ${id}: blocks array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }

    // Validate blockedBy array
    if (workUnit.blockedBy) {
      if (!Array.isArray(workUnit.blockedBy)) {
        errors.push(`Work unit ${id}: blockedBy must be an array`);
      } else {
        for (let i = 0; i < workUnit.blockedBy.length; i++) {
          if (typeof workUnit.blockedBy[i] !== 'string' || workUnit.blockedBy[i].trim() === '') {
            errors.push(`Work unit ${id}: blockedBy array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }

    // Validate dependsOn array
    if (workUnit.dependsOn) {
      if (!Array.isArray(workUnit.dependsOn)) {
        errors.push(`Work unit ${id}: dependsOn must be an array`);
      } else {
        for (let i = 0; i < workUnit.dependsOn.length; i++) {
          if (typeof workUnit.dependsOn[i] !== 'string' || workUnit.dependsOn[i].trim() === '') {
            errors.push(`Work unit ${id}: dependsOn array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }

    // Validate relatesTo array
    if (workUnit.relatesTo) {
      if (!Array.isArray(workUnit.relatesTo)) {
        errors.push(`Work unit ${id}: relatesTo must be an array`);
      } else {
        for (let i = 0; i < workUnit.relatesTo.length; i++) {
          if (typeof workUnit.relatesTo[i] !== 'string' || workUnit.relatesTo[i].trim() === '') {
            errors.push(`Work unit ${id}: relatesTo array contains empty strings or non-strings at index ${i}`);
          }
        }
      }
    }
  }

  return {
    valid: errors.length === 0,
    checks,
    ...(errors.length > 0 && { errors }),
  };
}
