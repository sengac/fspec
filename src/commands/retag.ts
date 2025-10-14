import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface RetagOptions {
  from: string;
  to: string;
  dryRun?: boolean;
  cwd?: string;
}

interface RetagResult {
  success: boolean;
  fileCount: number;
  occurrenceCount: number;
  message?: string;
  files?: string[];
  error?: string;
}

export async function retag(options: RetagOptions): Promise<RetagResult> {
  const { from, to, dryRun = false, cwd = process.cwd() } = options;

  // Validate required parameters
  if (!from || !to) {
    return {
      success: false,
      fileCount: 0,
      occurrenceCount: 0,
      error: 'Both --from and --to are required',
    };
  }

  // Validate tag format for 'to' parameter
  if (!to.startsWith('@')) {
    return {
      success: false,
      fileCount: 0,
      occurrenceCount: 0,
      error: `Invalid tag format: "${to}". Valid format is @lowercase-with-hyphens`,
    };
  }

  if (!/^@[a-z0-9-#]+$/.test(to)) {
    return {
      success: false,
      fileCount: 0,
      occurrenceCount: 0,
      error: `Invalid tag format: "${to}". Valid format is @lowercase-with-hyphens`,
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
        fileCount: 0,
        occurrenceCount: 0,
        message: 'No feature files found',
      };
    }

    // Find all files containing the 'from' tag
    const matchingFiles: Array<{ file: string; occurrences: number }> = [];

    for (const file of files) {
      const filePath = join(cwd, file);
      const content = await readFile(filePath, 'utf-8');

      // Simple regex to count tag occurrences
      // Match tag as whole word (preceded by whitespace or start, followed by whitespace or end)
      const escapedFrom = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const tagRegex = new RegExp(`(^|\\s)${escapedFrom}(?=\\s|$)`, 'gm');
      const matches = content.match(tagRegex);

      if (matches && matches.length > 0) {
        matchingFiles.push({
          file,
          occurrences: matches.length,
        });
      }
    }

    // No files found with the tag
    if (matchingFiles.length === 0) {
      return {
        success: false,
        fileCount: 0,
        occurrenceCount: 0,
        error: `Tag ${from} not found in any feature files`,
      };
    }

    const totalOccurrences = matchingFiles.reduce(
      (sum, f) => sum + f.occurrences,
      0
    );

    // Dry run - just report what would happen
    if (dryRun) {
      return {
        success: true,
        fileCount: matchingFiles.length,
        occurrenceCount: totalOccurrences,
        message: `Would rename ${from} to ${to} in ${matchingFiles.length} file(s) (${totalOccurrences} occurrence(s))`,
        files: matchingFiles.map(f => f.file),
      };
    }

    // Perform renaming
    for (const { file } of matchingFiles) {
      const filePath = join(cwd, file);
      let content = await readFile(filePath, 'utf-8');

      // Replace all occurrences of the tag
      // Match tag as whole word (preceded by whitespace or start, followed by whitespace or end)
      const escapedFrom = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const tagRegex = new RegExp(`(^|\\s)${escapedFrom}(?=\\s|$)`, 'gm');
      content = content.replace(tagRegex, `$1${to}`);

      // Validate the new content is valid Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      try {
        parser.parse(content);
      } catch (error: any) {
        return {
          success: false,
          fileCount: 0,
          occurrenceCount: 0,
          error: `Validation failed after renaming in ${file}: ${error.message}`,
        };
      }

      // Write the modified content
      await writeFile(filePath, content, 'utf-8');
    }

    return {
      success: true,
      fileCount: matchingFiles.length,
      occurrenceCount: totalOccurrences,
      message: `Renamed ${from} to ${to} in ${matchingFiles.length} file(s) (${totalOccurrences} occurrence(s)). All modified files validated successfully.`,
      files: matchingFiles.map(f => f.file),
    };
  } catch (error: any) {
    return {
      success: false,
      fileCount: 0,
      occurrenceCount: 0,
      error: error.message,
    };
  }
}

export async function retagCommand(options: {
  from?: string;
  to?: string;
  dryRun?: boolean;
}): Promise<void> {
  try {
    const result = await retag({
      from: options.from || '',
      to: options.to || '',
      dryRun: options.dryRun,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (options.dryRun && result.files) {
      console.log(chalk.yellow('Dry run mode - no files modified'));
      console.log(
        chalk.cyan(
          `\nWould rename ${options.from} to ${options.to} in ${result.fileCount} file(s) (${result.occurrenceCount} occurrence(s)):\n`
        )
      );

      for (const file of result.files) {
        console.log(chalk.gray(`  - ${file}`));
      }
    } else if (result.files && result.files.length > 0) {
      console.log(chalk.green(`✓ ${result.message}`));
      console.log(chalk.gray('\nModified files:'));
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

export function registerRetagCommand(program: Command): void {
  program
    .command('retag')
    .description('Bulk rename tags across all feature files')
    .option('--from <tag>', 'Tag to rename from (e.g., @old-tag)')
    .option('--to <tag>', 'Tag to rename to (e.g., @new-tag)')
    .option('--dry-run', 'Preview changes without making modifications')
    .action(retagCommand);
}
