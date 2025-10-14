import { readFile, unlink } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface DeleteFeaturesByTagOptions {
  tags: string[];
  dryRun?: boolean;
  cwd?: string;
}

interface DeleteFeaturesByTagResult {
  success: boolean;
  deletedCount: number;
  message?: string;
  files?: string[];
  error?: string;
}

export async function deleteFeaturesByTag(
  options: DeleteFeaturesByTagOptions
): Promise<DeleteFeaturesByTagResult> {
  const { tags, dryRun = false, cwd = process.cwd() } = options;

  // Require at least one tag
  if (!tags || tags.length === 0) {
    return {
      success: false,
      deletedCount: 0,
      error: 'At least one --tag is required',
    };
  }

  try {
    // Get all feature files
    const files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    if (files.length === 0) {
      return {
        success: true,
        deletedCount: 0,
        message: 'No feature files found',
      };
    }

    // Find all files matching the tags (AND logic)
    const matchingFiles: string[] = [];

    for (const file of files) {
      const filePath = join(cwd, file);
      const content = await readFile(filePath, 'utf-8');

      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      try {
        gherkinDocument = parser.parse(content);
      } catch {
        // Skip files with invalid syntax
        continue;
      }

      if (!gherkinDocument.feature) {
        continue;
      }

      // Get feature-level tags
      const featureTags = gherkinDocument.feature.tags.map(t => t.name);

      // Check if feature has ALL specified tags (AND logic)
      const hasAllTags = tags.every(tag => featureTags.includes(tag));

      if (hasAllTags) {
        matchingFiles.push(file);
      }
    }

    // No matching files
    if (matchingFiles.length === 0) {
      return {
        success: true,
        deletedCount: 0,
        message: 'No feature files found matching tags',
      };
    }

    // Dry run - just report what would happen
    if (dryRun) {
      return {
        success: true,
        deletedCount: matchingFiles.length,
        message: `Would delete ${matchingFiles.length} feature file(s)`,
        files: matchingFiles,
      };
    }

    // Perform deletions
    for (const file of matchingFiles) {
      const filePath = join(cwd, file);
      await unlink(filePath);
    }

    return {
      success: true,
      deletedCount: matchingFiles.length,
      message: `Deleted ${matchingFiles.length} feature file(s)`,
      files: matchingFiles,
    };
  } catch (error: any) {
    return {
      success: false,
      deletedCount: 0,
      error: error.message,
    };
  }
}

export async function deleteFeaturesByTagCommand(options: {
  tag?: string | string[];
  dryRun?: boolean;
}): Promise<void> {
  try {
    // Normalize tag input to array
    let tags: string[];
    if (Array.isArray(options.tag)) {
      tags = options.tag;
    } else if (options.tag) {
      tags = [options.tag];
    } else {
      tags = [];
    }

    const result = await deleteFeaturesByTag({
      tags,
      dryRun: options.dryRun,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (options.dryRun && result.files) {
      console.log(chalk.yellow('Dry run mode - no files modified'));
      console.log(
        chalk.cyan(`\nWould delete ${result.deletedCount} feature file(s):\n`)
      );

      for (const file of result.files) {
        console.log(chalk.gray(`  - ${file}`));
      }
    } else if (result.files && result.files.length > 0) {
      console.log(chalk.green(`✓ ${result.message}`));
      console.log(chalk.gray('\nDeleted files:'));
      for (const file of result.files) {
        console.log(chalk.gray(`  - ${file}`));
      }
    } else {
      console.log(chalk.yellow(result.message));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerDeleteFeaturesCommand(program: Command): void {
  program
    .command('delete-features')
    .description('Bulk delete feature files by tag')
    .option(
      '--tag <tag>',
      'Filter by tag (can specify multiple times for AND logic)',
      (value, previous) => {
        return previous ? [...previous, value] : [value];
      }
    )
    .option('--dry-run', 'Preview deletions without making changes')
    .action(deleteFeaturesByTagCommand);
}
