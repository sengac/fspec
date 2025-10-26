/**
 * Show test patterns for work units by tag
 * Part of QRY-002: Enhanced search and comparison commands
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { queryWorkUnits } from './query-work-units';
import {
  readAllCoverageFiles,
  extractTestFiles,
} from '../utils/coverage-reader';
import { parseAllFeatures } from '../utils/feature-parser';

interface ShowTestPatternsOptions {
  tag: string;
  includeCoverage?: boolean;
  json?: boolean;
  cwd?: string;
}

interface ShowTestPatternsResult {
  workUnits: Array<{ tags: string[] }>;
  testFiles: string[];
  patterns: Array<unknown>;
  format: string;
}

export async function showTestPatterns(
  options: ShowTestPatternsOptions
): Promise<ShowTestPatternsResult> {
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

  // Extract test files if coverage requested
  let testFiles: string[] = [];
  if (options.includeCoverage) {
    const testFileMappings = extractTestFiles(coverageFiles);
    const testFileSet = new Set(testFileMappings.map(t => t.filePath));
    testFiles = Array.from(testFileSet);
  }

  // Analyze test patterns (placeholder for future analysis)
  const patterns: Array<unknown> = [];
  // TODO: Implement pattern analysis (e.g., common describe blocks, mocking patterns, assertion patterns)

  return {
    workUnits: workUnits.map(wu => ({ tags: (wu.tags as string[]) || [] })),
    testFiles,
    patterns,
    format: options.json ? 'json' : 'table',
  };
}

export function registerShowTestPatternsCommand(program: Command): void {
  program
    .command('show-test-patterns')
    .description(
      'Analyze and display common testing patterns across work units'
    )
    .requiredOption(
      '--tag <tag>',
      'Filter work units by tag (e.g., @high, @cli)'
    )
    .option(
      '--include-coverage',
      'Include test file paths and coverage information'
    )
    .option('--json', 'Output results in JSON format')
    .action(
      async (options: {
        tag: string;
        includeCoverage?: boolean;
        json?: boolean;
      }) => {
        try {
          const result = await showTestPatterns(options);
          if (options.json) {
            console.log(JSON.stringify(result, null, 2));
          } else {
            console.log(
              chalk.green(
                `✓ Analyzed testing patterns for ${result.workUnits.length} work units tagged with ${options.tag}`
              )
            );
          }
        } catch (error: unknown) {
          if (error instanceof Error) {
            console.error(chalk.red('✗ Analysis failed:'), error.message);
          }
          process.exit(1);
        }
      }
    );
}
