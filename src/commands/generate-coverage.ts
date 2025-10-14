/**
 * Generate Coverage Command
 *
 * Generates .feature.coverage files for all existing .feature files that lack coverage tracking.
 * Reuses the createCoverageFile utility for consistency with auto-created coverage files.
 */

import { readdir } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { createCoverageFile } from '../utils/coverage-file';

export interface GenerateCoverageOptions {
  cwd?: string;
  dryRun?: boolean;
}

export interface GenerateCoverageResult {
  created: number;
  skipped: number;
  recreated: number;
  dryRun?: boolean;
  files?: string[];
}

export async function generateCoverage(
  options: GenerateCoverageOptions = {}
): Promise<GenerateCoverageResult> {
  const cwd = options.cwd || process.cwd();
  const featuresDir = join(cwd, 'spec', 'features');

  // Scan for .feature files
  let files: string[];
  try {
    files = await readdir(featuresDir);
  } catch (error: any) {
    throw new Error(`Failed to read features directory: ${error.message}`);
  }

  const featureFiles = files.filter((f) => f.endsWith('.feature'));

  // Track results
  let created = 0;
  let skipped = 0;
  let recreated = 0;
  const fileList: string[] = [];

  // Process each feature file
  for (const featureFile of featureFiles) {
    const featureFilePath = join(featuresDir, featureFile);
    const coverageFileName = `${featureFile}.coverage`;

    if (options.dryRun) {
      // Dry-run mode: check what would be created
      const coverageFilePath = `${featureFilePath}.coverage`;
      try {
        const { readFile, access } = await import('fs/promises');
        await access(coverageFilePath);

        // Coverage file exists, check if valid
        try {
          const content = await readFile(coverageFilePath, 'utf-8');
          JSON.parse(content);
          // Valid, would skip
          skipped++;
        } catch {
          // Invalid JSON, would recreate
          recreated++;
          fileList.push(coverageFileName);
        }
      } catch (error: any) {
        if (error.code === 'ENOENT') {
          // Doesn't exist, would create
          created++;
          fileList.push(coverageFileName);
        }
      }
    } else {
      // Actually create coverage files
      const result = await createCoverageFile(featureFilePath);

      switch (result.status) {
        case 'created':
          created++;
          break;
        case 'skipped':
          skipped++;
          break;
        case 'recreated':
          recreated++;
          break;
      }
    }
  }

  return {
    created,
    skipped,
    recreated,
    dryRun: options.dryRun,
    files: options.dryRun ? fileList : undefined,
  };
}

export async function generateCoverageCommand(options: {
  dryRun?: boolean;
}): Promise<void> {
  try {
    const result = await generateCoverage(options);

    if (result.dryRun) {
      console.log(
        chalk.yellow(
          `Would create ${result.created} coverage files (DRY RUN)`
        )
      );
      if (result.files && result.files.length > 0) {
        console.log(chalk.cyan('\nFiles that would be created:'));
        result.files.forEach((file) => console.log(chalk.cyan(`  - ${file}`)));
      }
      if (result.skipped > 0) {
        console.log(chalk.dim(`\nWould skip ${result.skipped} existing files`));
      }
      if (result.recreated > 0) {
        console.log(
          chalk.yellow(`Would recreate ${result.recreated} invalid files`)
        );
      }
    } else {
      // Regular output
      const parts: string[] = [];
      if (result.created > 0) {
        parts.push(`Created ${result.created}`);
      }
      if (result.skipped > 0) {
        parts.push(`Skipped ${result.skipped}`);
      }
      if (result.recreated > 0) {
        parts.push(`Recreated ${result.recreated} (invalid JSON)`);
      }

      if (parts.length === 0) {
        console.log(chalk.dim('No coverage files needed'));
      } else {
        console.log(chalk.green(`✓ ${parts.join(', ')}`));
      }
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red(`✗ Error: ${error.message}`));
    process.exit(1);
  }
}
