/**
 * Compare implementation approaches across work units
 * Part of QRY-002: Enhanced search and comparison commands
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { readFile } from 'fs/promises';
import { queryWorkUnits } from './query-work-units';
import { readAllCoverageFiles, extractImplementationFiles, extractTestFiles } from '../utils/coverage-reader';
import { parseAllFeatures } from '../utils/feature-parser';

interface CompareImplementationsOptions {
  tag: string;
  showCoverage?: boolean;
  json?: boolean;
  cwd?: string;
}

interface CompareImplementationsResult {
  workUnits: Array<{ tags: string[] }>;
  comparison: { type: string };
  namingConventionDifferences: Array<unknown>;
  coverage: Array<{
    testFiles: string[];
    implementationFiles: string[];
  }>;
}

export async function compareImplementations(
  options: CompareImplementationsOptions
): Promise<CompareImplementationsResult> {
  // Query work units with the specified tag
  const result = await queryWorkUnits({
    tag: options.tag,
    cwd: options.cwd,
  });

  const workUnits = result.workUnits || [];

  // Parse features to get work unit IDs
  const parsedFeatures = await parseAllFeatures(options.cwd);
  const featuresByWorkUnit = new Map<string, string>();
  for (const parsed of parsedFeatures) {
    if (parsed.workUnitId) {
      featuresByWorkUnit.set(parsed.workUnitId, parsed.filePath);
    }
  }

  // Read coverage files
  const coverageFiles = await readAllCoverageFiles(options.cwd);

  // Build coverage data for matching work units
  const coverage: Array<{
    testFiles: string[];
    implementationFiles: string[];
  }> = [];

  if (options.showCoverage) {
    const testFiles = extractTestFiles(coverageFiles);
    const implFiles = extractImplementationFiles(coverageFiles);

    const testFileSet = new Set(testFiles.map(t => t.filePath));
    const implFileSet = new Set(implFiles.map(i => i.filePath));

    coverage.push({
      testFiles: Array.from(testFileSet),
      implementationFiles: Array.from(implFileSet),
    });
  }

  // Detect naming convention differences (simple heuristic)
  const namingConventionDifferences: Array<unknown> = [];
  // TODO: Implement naming convention analysis

  return {
    workUnits: workUnits.map(wu => ({ tags: (wu.tags as string[]) || [] })),
    comparison: { type: 'side-by-side' },
    namingConventionDifferences,
    coverage,
  };
}

export function registerCompareImplementationsCommand(program: Command): void {
  program
    .command('compare-implementations')
    .description('Compare implementation approaches across work units to identify patterns and inconsistencies')
    .requiredOption('--tag <tag>', 'Filter work units by tag (e.g., @authentication, @cli)')
    .option('--show-coverage', 'Include test and implementation file paths from coverage data')
    .option('--json', 'Output results in JSON format')
    .action(
      async (options: {
        tag: string;
        showCoverage?: boolean;
        json?: boolean;
      }) => {
        try {
          const result = await compareImplementations(options);
          if (options.json) {
            console.log(JSON.stringify(result, null, 2));
          } else {
            console.log(chalk.green(`✓ Compared ${result.workUnits.length} work units tagged with ${options.tag}`));
          }
        } catch (error: unknown) {
          if (error instanceof Error) {
            console.error(chalk.red('✗ Comparison failed:'), error.message);
          }
          process.exit(1);
        }
      }
    );
}
