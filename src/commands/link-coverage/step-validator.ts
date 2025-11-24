import { access } from 'fs/promises';
import { join } from 'path';
import { getScenarioSteps } from '../../utils/feature-parser';
import {
  validateSteps,
  formatValidationError,
} from '../../utils/step-validation';
import { WorkUnitType } from '../../types/work-units';
import { wrapSystemReminder } from './utils';
import { detectWorkUnitType } from './utils';

export async function validateStepConsistency(
  featureFile: string,
  scenario: string,
  testFile: string,
  skipStepValidation: boolean,
  cwd: string
): Promise<void> {
  if (!testFile || skipStepValidation) {
    return;
  }

  try {
    // Check if feature file exists before trying to parse it
    await access(featureFile);

    // Extract steps from feature file scenario
    const featureSteps = await getScenarioSteps(featureFile, scenario);

    // Read test file content
    const testFilePath = join(cwd, testFile);
    // We need to read the file content here, but we don't want to duplicate logic
    // For now, let's assume the caller handles reading or we import readFile
    // To avoid circular deps or complexity, let's just use fs/promises
    const { readFile } = await import('fs/promises');
    const testContent = await readFile(testFilePath, 'utf-8');

    // Detect work unit type for error message customization
    const workUnitType = await detectWorkUnitType(featureFile, cwd);

    // Validate steps match
    const validationResult = validateSteps(featureSteps, testContent);

    if (!validationResult.valid) {
      // Step validation failed - throw error with system-reminder
      const errorMessage = formatValidationError(
        validationResult,
        workUnitType
      );
      throw new Error(errorMessage + '\n\nStep validation failed');
    }
  } catch (error: any) {
    // If feature file doesn't exist, skip step validation silently
    if (error.code === 'ENOENT' && error.path === featureFile) {
      // Feature file not found - skip step validation
      // This allows forward planning where feature file may not exist yet
    } else {
      // Re-throw other errors (includes validation failures with system-reminder)
      throw error;
    }
  }
}
