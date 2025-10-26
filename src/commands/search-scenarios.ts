/**
 * Search scenarios across feature files
 * Part of QRY-002: Enhanced search and comparison commands
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import {
  parseAllFeatures,
  searchScenarios as searchScenariosUtil,
} from '../utils/feature-parser';

interface SearchScenariosOptions {
  query: string;
  regex?: boolean;
  json?: boolean;
  cwd?: string;
}

interface SearchScenariosResult {
  searchedFiles: number;
  scenarios: Array<{
    name: string;
    scenarioName: string;
    featureFilePath: string;
    workUnitId: string;
  }>;
  format: string;
  searchMode?: string;
}

export async function searchScenarios(
  options: SearchScenariosOptions
): Promise<SearchScenariosResult> {
  // Parse all feature files
  const parsedFeatures = await parseAllFeatures(options.cwd);

  // Search scenarios using utility function
  const results = searchScenariosUtil(
    parsedFeatures,
    options.query,
    options.regex || false
  );

  // Transform results to match expected format
  const scenarios = results.map(result => ({
    name: result.scenarioName,
    scenarioName: result.scenarioName,
    featureFilePath: result.featureFilePath,
    workUnitId: result.workUnitId,
  }));

  return {
    searchedFiles: parsedFeatures.length,
    scenarios,
    format: options.json ? 'json' : 'table',
    searchMode: options.regex ? 'regex' : 'literal',
  };
}

export function registerSearchScenariosCommand(program: Command): void {
  program
    .command('search-scenarios')
    .description(
      'Search for scenarios across all feature files by text or regex pattern'
    )
    .requiredOption(
      '--query <pattern>',
      'Search pattern (literal text or regex)'
    )
    .option('--regex', 'Enable regex pattern matching (default: literal)')
    .option('--json', 'Output results in JSON format')
    .action(
      async (options: { query: string; regex?: boolean; json?: boolean }) => {
        try {
          const result = await searchScenarios(options);
          if (options.json) {
            console.log(JSON.stringify(result, null, 2));
          } else {
            console.log(
              chalk.green(
                `✓ Found ${result.scenarios.length} scenarios matching "${options.query}"`
              )
            );
          }
        } catch (error: unknown) {
          if (error instanceof Error) {
            console.error(chalk.red('✗ Search failed:'), error.message);
          }
          process.exit(1);
        }
      }
    );
}
