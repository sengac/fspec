/**
 * Search implementation code for specific function usage
 * Part of QRY-002: Enhanced search and comparison commands
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { readFile } from 'fs/promises';
import { readAllCoverageFiles, extractImplementationFiles } from '../utils/coverage-reader';
import { queryWorkUnits } from './query-work-units';

interface SearchImplementationOptions {
  function: string;
  showWorkUnits?: boolean;
  json?: boolean;
  cwd?: string;
}

interface SearchImplementationResult {
  searchedFiles: number;
  files: Array<{
    content: string;
    filePath: string;
    workUnits: Array<{ workUnitId: string }>;
  }>;
}

export async function searchImplementation(
  options: SearchImplementationOptions
): Promise<SearchImplementationResult> {
  // Read all coverage files
  const coverageFiles = await readAllCoverageFiles(options.cwd);

  // Extract implementation files
  const implFiles = extractImplementationFiles(coverageFiles);

  // Search for function usage in implementation files
  const matchingFiles = new Map<string, Set<string>>();

  for (const implFile of implFiles) {
    try {
      const content = await readFile(implFile.filePath, 'utf-8');

      // Check if file contains the function
      if (content.includes(options.function)) {
        if (!matchingFiles.has(implFile.filePath)) {
          matchingFiles.set(implFile.filePath, new Set());
        }
        matchingFiles.get(implFile.filePath)?.add(implFile.featureName);
      }
    } catch (error) {
      // Skip files that cannot be read
      continue;
    }
  }

  // Build result
  const files = await Promise.all(
    Array.from(matchingFiles.entries()).map(async ([filePath, featureNames]) => {
      const content = await readFile(filePath, 'utf-8');

      // Get work unit IDs from coverage data
      const workUnitIds = new Set<string>();
      for (const implFile of implFiles) {
        if (implFile.filePath === filePath) {
          // Feature name is used as work unit ID lookup
          const featureName = implFile.featureName;
          // Try to find work unit ID from parsed features
          workUnitIds.add(featureName.toUpperCase().replace(/-/g, '-'));
        }
      }

      return {
        content,
        filePath,
        workUnits: Array.from(workUnitIds).map(id => ({ workUnitId: id })),
      };
    })
  );

  return {
    searchedFiles: implFiles.length,
    files,
  };
}

export function registerSearchImplementationCommand(program: Command): void {
  program
    .command('search-implementation')
    .description('Search implementation code for specific function usage across work units')
    .requiredOption('--function <name>', 'Function name to search for')
    .option('--show-work-units', 'Display which work units use each file')
    .option('--json', 'Output results in JSON format')
    .action(
      async (options: {
        function: string;
        showWorkUnits?: boolean;
        json?: boolean;
      }) => {
        try {
          const result = await searchImplementation(options);
          if (options.json) {
            console.log(JSON.stringify(result, null, 2));
          } else {
            console.log(chalk.green(`✓ Found "${options.function}" in ${result.files.length} file(s)`));
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
