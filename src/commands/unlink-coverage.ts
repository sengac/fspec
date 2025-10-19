/**
 * Unlink Coverage Command
 *
 * Removes test and/or implementation mappings from scenario coverage files.
 * Supports three modes: remove all, remove test mapping, remove only implementation.
 */

import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { CoverageFile } from '../utils/coverage-file';

interface UnlinkCoverageOptions {
  scenario: string;
  testFile?: string;
  implFile?: string;
  all?: boolean;
  cwd?: string;
}

interface UnlinkCoverageResult {
  success: boolean;
  message: string;
}

export async function unlinkCoverage(
  featureName: string,
  options: UnlinkCoverageOptions
): Promise<UnlinkCoverageResult> {
  const {
    scenario,
    testFile,
    implFile,
    all = false,
    cwd = process.cwd(),
  } = options;

  // Validate flag combinations
  if (!all && !testFile) {
    throw new Error(
      'Must specify either --all or --test-file\n' +
        'Use --all to remove all mappings, or --test-file to remove specific test mapping'
    );
  }

  if (implFile && !testFile) {
    throw new Error(
      '--test-file is required when specifying --impl-file\n' +
        'Implementation mappings are attached to test mappings'
    );
  }

  // Load coverage file
  const featuresDir = join(cwd, 'spec', 'features');
  const fileName = featureName.endsWith('.feature')
    ? featureName
    : `${featureName}.feature`;
  const coverageFile = join(featuresDir, `${fileName}.coverage`);

  let coverage: CoverageFile;
  try {
    const content = await readFile(coverageFile, 'utf-8');
    coverage = JSON.parse(content);
  } catch (error: any) {
    throw new Error(
      `Coverage file not found: ${fileName}.coverage\n` +
        'Suggestion: Run fspec show-coverage to see available features'
    );
  }

  // Find the scenario
  const scenarioEntry = coverage.scenarios.find(s => s.name === scenario);
  if (!scenarioEntry) {
    throw new Error(
      `Scenario not found: "${scenario}"\n` +
        `Available scenarios:\n${coverage.scenarios.map(s => `  - ${s.name}`).join('\n')}`
    );
  }

  let message = '';

  if (all) {
    // Mode 1: Remove all mappings
    scenarioEntry.testMappings = [];
    message = `✓ Removed all coverage mappings for scenario "${scenario}"`;
  } else if (testFile && implFile) {
    // Mode 2: Remove only implementation mapping
    const testMapping = scenarioEntry.testMappings.find(
      tm => tm.file === testFile
    );

    if (!testMapping) {
      throw new Error(
        `Test file not found in scenario mappings: ${testFile}\n` +
          'Suggestion: Run fspec show-coverage to see current mappings'
      );
    }

    const implIndex = testMapping.implMappings.findIndex(
      im => im.file === implFile
    );

    if (implIndex === -1) {
      throw new Error(
        `Implementation file not found in test mapping: ${implFile}\n` +
          'Suggestion: Run fspec show-coverage to see current mappings'
      );
    }

    testMapping.implMappings.splice(implIndex, 1);
    message = `✓ Removed implementation mapping ${implFile} from scenario "${scenario}"`;
  } else if (testFile) {
    // Mode 3: Remove entire test mapping (and all its impl mappings)
    const testIndex = scenarioEntry.testMappings.findIndex(
      tm => tm.file === testFile
    );

    if (testIndex === -1) {
      throw new Error(
        `Test file not found in scenario mappings: ${testFile}\n` +
          'Suggestion: Run fspec show-coverage to see current mappings'
      );
    }

    scenarioEntry.testMappings.splice(testIndex, 1);
    message = `✓ Removed test mapping ${testFile} (and all its implementation mappings) from scenario "${scenario}"`;
  }

  // Recalculate stats
  updateStats(coverage);

  // Write updated coverage file
  await writeFile(coverageFile, JSON.stringify(coverage, null, 2), 'utf-8');

  return {
    success: true,
    message,
  };
}

function updateStats(coverage: CoverageFile): void {
  const testFiles = new Set<string>();
  const implFiles = new Set<string>();
  let totalTestLines = 0;
  let totalImplLines = 0;
  let coveredScenarios = 0;

  for (const scenario of coverage.scenarios) {
    if (scenario.testMappings.length > 0) {
      coveredScenarios++;
    }

    for (const testMapping of scenario.testMappings) {
      testFiles.add(testMapping.file);

      // Count test lines
      const range = testMapping.lines.split('-');
      if (range.length === 2) {
        const start = parseInt(range[0], 10);
        const end = parseInt(range[1], 10);
        totalTestLines += end - start + 1;
      }

      for (const implMapping of testMapping.implMappings) {
        implFiles.add(implMapping.file);
        totalImplLines += implMapping.lines.length;
      }
    }
  }

  coverage.stats.coveredScenarios = coveredScenarios;
  coverage.stats.coveragePercent =
    coverage.stats.totalScenarios > 0
      ? Math.round((coveredScenarios / coverage.stats.totalScenarios) * 100)
      : 0;
  coverage.stats.testFiles = Array.from(testFiles);
  coverage.stats.implFiles = Array.from(implFiles);
  coverage.stats.totalLinesCovered = totalTestLines + totalImplLines;
}

export async function unlinkCoverageCommand(
  featureName: string,
  options: Omit<UnlinkCoverageOptions, 'cwd'>
): Promise<void> {
  try {
    const result = await unlinkCoverage(featureName, options);
    console.log(result.message);
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerUnlinkCoverageCommand(program: Command): void {
  program
    .command('unlink-coverage')
    .description(
      'Remove test or implementation links from scenario coverage mappings'
    )
    .argument(
      '<feature-name>',
      'Feature name (e.g., "user-login" for user-login.feature)'
    )
    .requiredOption('--scenario <name>', 'Scenario name to unlink')
    .option('--test-file <path>', 'Test file path to remove')
    .option('--impl-file <path>', 'Implementation file path to remove')
    .option('--all', 'Remove all mappings for the scenario')
    .action(unlinkCoverageCommand);
}
