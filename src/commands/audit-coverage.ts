/**
 * Audit Coverage Command
 *
 * Audits coverage files to verify that all test files and implementation files exist.
 * Provides actionable recommendations for missing files.
 */

import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { CoverageFile } from '../utils/coverage-file';

export interface AuditCoverageOptions {
  featureName: string;
  cwd?: string;
}

export interface AuditCoverageResult {
  output: string;
  exitCode: number;
}

export async function auditCoverage(
  options: AuditCoverageOptions
): Promise<AuditCoverageResult> {
  const cwd = options.cwd || process.cwd();
  const featuresDir = join(cwd, 'spec', 'features');
  const coverageFilePath = join(
    featuresDir,
    `${options.featureName}.feature.coverage`
  );

  // Check if coverage file exists
  if (!existsSync(coverageFilePath)) {
    return {
      output: chalk.red(`✗ Coverage file not found: ${coverageFilePath}`),
      exitCode: 1,
    };
  }

  // Read coverage file
  const coverageContent = await readFile(coverageFilePath, 'utf-8');
  const coverage: CoverageFile = JSON.parse(coverageContent);

  // Collect all files referenced in coverage
  const allFiles: string[] = [];
  const missingFiles: Array<{ file: string; type: 'test' | 'implementation' }> =
    [];

  for (const scenario of coverage.scenarios) {
    // Check test mappings
    for (const testMapping of scenario.testMappings) {
      allFiles.push(testMapping.file);
      const fullPath = join(cwd, testMapping.file);
      if (!existsSync(fullPath)) {
        missingFiles.push({ file: testMapping.file, type: 'test' });
      }

      // Check implementation mappings within this test
      for (const implMapping of testMapping.implMappings) {
        allFiles.push(implMapping.file);
        const implFullPath = join(cwd, implMapping.file);
        if (!existsSync(implFullPath)) {
          missingFiles.push({ file: implMapping.file, type: 'implementation' });
        }
      }
    }
  }

  // Generate report
  let output = '';

  if (missingFiles.length === 0) {
    // All files found
    output += chalk.green(
      `✅ All files found (${allFiles.length}/${allFiles.length})`
    );
    output += '\n';
    output += chalk.green('All mappings valid');

    return {
      output,
      exitCode: 0,
    };
  }

  // Missing files found
  output += chalk.red(
    `✗ ${missingFiles.length} missing file(s) out of ${allFiles.length} total files`
  );
  output += '\n\n';

  for (const missing of missingFiles) {
    if (missing.type === 'test') {
      output += chalk.red(`❌ Test file not found: ${missing.file}`);
      output += '\n';
      output += chalk.yellow(
        '   Recommendation: Remove this mapping or restore the deleted file'
      );
      output += '\n\n';
    } else {
      output += chalk.red(`❌ Implementation file not found: ${missing.file}`);
      output += '\n';
      output += chalk.yellow(
        '   Recommendation: Remove this mapping or restore the deleted file'
      );
      output += '\n\n';
    }
  }

  return {
    output,
    exitCode: 1,
  };
}

export async function auditCoverageCommand(featureName: string): Promise<void> {
  const result = await auditCoverage({ featureName });
  console.log(result.output);
  process.exit(result.exitCode);
}

export function registerAuditCoverageCommand(program: Command): void {
  program
    .command('audit-coverage')
    .description(
      'Audit coverage file to verify test and implementation files exist'
    )
    .argument(
      '<feature-name>',
      'Feature name (e.g., "user-login" for user-login.feature.coverage)'
    )
    .action(auditCoverageCommand);
}
