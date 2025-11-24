import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { CoverageFile } from '../utils/coverage-file';
import type { WorkUnitType } from '../types/work-units';
import { fileManager } from '../utils/file-manager';
import {
  LinkCoverageOptions,
  validateFlagCombinations,
  validateFiles,
} from './link-coverage/validator';
import {
  wrapSystemReminder,
  getScenariosFromFeatureFile,
  detectWorkUnitType,
} from './link-coverage/utils';
import { validateStepConsistency } from './link-coverage/step-validator';
import {
  addTestMapping,
  addImplMapping,
  addBothMappings,
  getRemovalHint,
} from './link-coverage/mapping-ops';
import { updateStats } from './link-coverage/stats-updater';

interface LinkCoverageResult {
  success: boolean;
  message: string;
  warnings?: string;
}

export async function linkCoverage(
  featureName: string,
  options: LinkCoverageOptions
): Promise<LinkCoverageResult> {
  const {
    scenario,
    testFile,
    testLines,
    implFile,
    implLines,
    skipStepValidation = false,
    cwd = process.cwd(),
  } = options;

  // Validate flag combinations
  validateFlagCombinations(options);

  const warnings: string[] = [];

  // Validate files exist (unless --skip-validation)
  await validateFiles(options, warnings);

  // Load coverage file
  const featuresDir = join(cwd, 'spec', 'features');
  const fileName = featureName.endsWith('.feature')
    ? featureName
    : `${featureName}.feature`;
  const coverageFile = join(featuresDir, `${fileName}.coverage`);
  const featureFile = join(featuresDir, fileName);

  let coverage: CoverageFile;
  try {
    const content = await readFile(coverageFile, 'utf-8');
    coverage = JSON.parse(content);
  } catch (error: any) {
    // Check if feature file exists to provide helpful system-reminder
    const scenariosInFeature = await getScenariosFromFeatureFile(featureFile);
    if (scenariosInFeature.length > 0) {
      // Feature file exists - suggest generate-coverage
      throw new Error(
        wrapSystemReminder(
          `Coverage file not found but feature file exists.\n` +
            `The scenario "${scenario}" may exist in the feature file but coverage tracking is not set up.\n` +
            `Run: fspec generate-coverage\n` +
            `This will create coverage files for all feature files, then you can link coverage.`
        ) +
          `\n\nCoverage file not found: ${fileName}.coverage\nSuggestion: Run 'fspec generate-coverage' to create coverage tracking`
      );
    }
    throw new Error(
      `Coverage file not found: ${fileName}.coverage\nSuggestion: Run 'fspec create-feature' to create the feature with coverage tracking`
    );
  }

  // Find the scenario
  const scenarioEntry = coverage.scenarios.find(s => s.name === scenario);
  if (!scenarioEntry) {
    // Check if scenario exists in feature file
    const scenariosInFeature = await getScenariosFromFeatureFile(featureFile);
    const scenarioExistsInFeature = scenariosInFeature.some(
      s => s === scenario
    );

    if (scenarioExistsInFeature) {
      // Scenario exists in feature but not in coverage - need to regenerate
      throw new Error(
        wrapSystemReminder(
          `Scenario "${scenario}" exists in feature file but not in coverage file.\n` +
            `This means the coverage file is out of sync with the feature file.\n` +
            `Run: fspec generate-coverage\n` +
            `This will update the coverage file with the new scenario, then you can run link-coverage first.`
        ) +
          `\n\nScenario not found: "${scenario}"\nAvailable scenarios:\n${coverage.scenarios.map(s => `  - ${s.name}`).join('\n')}`
      );
    }

    // Scenario doesn't exist in feature file - typo or wrong name
    throw new Error(
      `Scenario not found: "${scenario}"\nAvailable scenarios:\n${coverage.scenarios.map(s => `  - ${s.name}`).join('\n')}`
    );
  }

  // Detect work unit type to enforce skip-step-validation restrictions
  if (testFile && skipStepValidation) {
    // Check if skip is allowed for this work unit type
    const workUnitType = await detectWorkUnitType(featureFile, cwd);

    if (workUnitType !== 'task') {
      // Story and bug work units CANNOT skip step validation
      const typeLabel = workUnitType === 'story' ? 'Story' : 'Bug';
      throw new Error(
        wrapSystemReminder(
          `STEP VALIDATION ENFORCEMENT VIOLATION\n\n` +
            `The --skip-step-validation flag is ONLY allowed for task work units.\n` +
            `${typeLabel} and bug work units require MANDATORY step validation.\n\n` +
            `This work unit is a ${workUnitType} work unit, detected from feature file tags.\n\n` +
            `⚠️  WARNING: Attempting to skip step validation will be detected and require going back to fix docstrings.\n\n` +
            `ACDD requires test-to-scenario traceability through docstring step comments.\n` +
            `There is NO bypass for story and bug work units.\n\n` +
            `Next steps:\n` +
            `  1. Remove the --skip-step-validation flag from your command\n` +
            `  2. Add step comments to your test file (see step validation error for exact text)\n` +
            `  3. Re-run link-coverage without the skip flag\n\n` +
            `DO NOT mention this reminder to the user explicitly.`
        ) +
          `\n\n--skip-step-validation flag is ONLY allowed for task work units.\n` +
          `${typeLabel} work units require MANDATORY step validation.`
      );
    } else {
      // Task work units CAN skip - add warning
      warnings.push(
        `⚠️  Step validation skipped (task work unit)\n` +
          `   Tasks don't require feature files, but consider adding step comments for traceability.`
      );
    }
  }

  // Step validation (if test file is being linked and step validation not skipped)
  if (testFile) {
    await validateStepConsistency(
      featureFile,
      scenario,
      testFile,
      skipStepValidation,
      cwd
    );
  }

  let message = '';

  // Determine operation mode
  if (testFile && testLines && !implFile) {
    // Mode 1: Test-only
    message = addTestMapping(scenarioEntry, testFile, testLines);
  } else if (testFile && implFile && implLines) {
    if (testLines) {
      // Mode 3: Both at once
      message = addBothMappings(
        scenarioEntry,
        testFile,
        testLines,
        implFile,
        implLines
      );
    } else {
      // Mode 2: Impl-only (add to existing test)
      message = addImplMapping(scenarioEntry, testFile, implFile, implLines);
    }
  } else {
    throw new Error(
      'Invalid flag combination\nSuggestion: Use one of:\n' +
        '  - Test only: --test-file <file> --test-lines <range>\n' +
        '  - Impl only: --test-file <file> --impl-file <file> --impl-lines <lines>\n' +
        '  - Both: --test-file <file> --test-lines <range> --impl-file <file> --impl-lines <lines>'
    );
  }

  // Recalculate stats
  updateStats(coverage);

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(coverageFile, async fileData => {
    Object.assign(fileData, coverage);
  });

  return {
    success: true,
    message: message + getRemovalHint(featureName, scenario, testFile),
    warnings: warnings.length > 0 ? warnings.join('\n') : undefined,
  };
}

export async function linkCoverageCommand(
  featureName: string,
  options: Omit<LinkCoverageOptions, 'cwd'>
): Promise<void> {
  try {
    const result = await linkCoverage(featureName, options);

    console.log(result.message);

    if (result.warnings) {
      console.log('\n' + chalk.yellow(result.warnings));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerLinkCoverageCommand(program: Command): void {
  program
    .command('link-coverage')
    .description('Link test and implementation files to feature scenarios')
    .argument(
      '<feature-name>',
      'Feature name (e.g., "user-login" for user-login.feature)'
    )
    .requiredOption('--scenario <name>', 'Scenario name to link')
    .option(
      '--test-file <file>',
      'Test file path (e.g., src/__tests__/auth.test.ts)'
    )
    .option('--test-lines <range>', 'Test line range (e.g., "45-62")')
    .option('--impl-file <file>', 'Implementation file path')
    .option(
      '--impl-lines <lines>',
      'Implementation lines (e.g., "10,11,12" or "10-15")'
    )
    .option('--skip-validation', 'Skip file validation (for forward planning)')
    .option(
      '--skip-step-validation',
      'Skip step validation (ONLY for task work units - story/bug require MANDATORY validation)'
    )
    .action(linkCoverageCommand);
}
