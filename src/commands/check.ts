import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { formatGherkinDocument } from '../utils/gherkin-formatter';
import { validateTags } from './validate-tags';

interface CheckOptions {
  verbose?: boolean;
  cwd?: string;
}

interface CheckResult {
  success: boolean;
  gherkinStatus?: 'PASS' | 'FAIL' | 'SKIP';
  tagStatus?: 'PASS' | 'FAIL' | 'SKIP';
  formatStatus?: 'PASS' | 'FAIL' | 'SKIP';
  fileCount?: number;
  message?: string;
  errors?: string[];
  details?: any;
}

export async function check(options: CheckOptions = {}): Promise<CheckResult> {
  const { verbose = false, cwd = process.cwd() } = options;
  const errors: string[] = [];
  let gherkinStatus: 'PASS' | 'FAIL' | 'SKIP' = 'SKIP';
  let tagStatus: 'PASS' | 'FAIL' | 'SKIP' = 'SKIP';
  let formatStatus: 'PASS' | 'FAIL' | 'SKIP' = 'SKIP';

  try {
    // Get all feature files
    const files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    if (files.length === 0) {
      return {
        success: true,
        message: 'No feature files found',
        fileCount: 0,
      };
    }

    const fileCount = files.length;

    // 1. Check Gherkin syntax
    gherkinStatus = 'PASS';
    const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    for (const file of files) {
      const filePath = join(cwd, file);
      try {
        const content = await readFile(filePath, 'utf-8');
        parser.parse(content);
      } catch (error: any) {
        gherkinStatus = 'FAIL';
        errors.push(`Gherkin syntax error in ${file}: ${error.message}`);
      }
    }

    // 2. Check tag validation (use proper validateTags function with work unit ID support)
    tagStatus = 'PASS';
    try {
      const tagResults = await validateTags({ cwd });

      if (tagResults.invalidCount > 0) {
        tagStatus = 'FAIL';

        // Collect errors from validation results
        for (const result of tagResults.results) {
          if (!result.valid) {
            for (const error of result.errors) {
              errors.push(`${error.message}`);
            }
          }
        }
      }
    } catch (error: any) {
      tagStatus = 'FAIL';
      errors.push(`Tag validation error: ${error.message}`);
    }

    // 3. Check formatting
    formatStatus = 'PASS';
    try {
      // Create parser for formatting check
      const uuidFn = Messages.IdGenerator.uuid();
      const formatBuilder = new Gherkin.AstBuilder(uuidFn);
      const formatMatcher = new Gherkin.GherkinClassicTokenMatcher();

      for (const file of files) {
        const filePath = join(cwd, file);
        try {
          const content = await readFile(filePath, 'utf-8');

          // Parse and format
          const formatParser = new Gherkin.Parser(formatBuilder, formatMatcher);
          const gherkinDocument = formatParser.parse(content);
          const formatted = formatGherkinDocument(gherkinDocument);

          // Compare with original
          if (content !== formatted) {
            formatStatus = 'FAIL';
            errors.push(`Formatting check failed: ${file} needs formatting`);
          }
        } catch (error: any) {
          // Skip files that fail to parse (already caught in Gherkin check)
        }
      }
    } catch (error: any) {
      formatStatus = 'SKIP';
    }

    // Determine overall success
    const success =
      gherkinStatus !== 'FAIL' &&
      tagStatus !== 'FAIL' &&
      formatStatus !== 'FAIL';

    const result: CheckResult = {
      success,
      gherkinStatus,
      tagStatus,
      formatStatus,
      fileCount,
      errors: errors.length > 0 ? errors : undefined,
    };

    if (success) {
      result.message = 'All checks passed';
    }

    if (verbose) {
      result.details = {
        files: files,
        gherkinChecked: gherkinStatus !== 'SKIP',
        tagsChecked: tagStatus !== 'SKIP',
        formattingChecked: formatStatus !== 'SKIP',
      };
    }

    return result;
  } catch (error: any) {
    return {
      success: false,
      errors: [error.message],
      message: 'Check failed with error',
    };
  }
}

export async function checkCommand(
  options: { verbose?: boolean } = {}
): Promise<void> {
  try {
    const result = await check({
      verbose: options.verbose,
    });

    // Display results
    console.log(chalk.bold('\nRunning validation checks...\n'));

    if (result.fileCount !== undefined && result.fileCount > 0) {
      console.log(chalk.gray(`Checked ${result.fileCount} feature file(s)\n`));
    }

    // Show status of each check
    if (result.gherkinStatus) {
      const status =
        result.gherkinStatus === 'PASS'
          ? chalk.green('PASS')
          : result.gherkinStatus === 'FAIL'
            ? chalk.red('FAIL')
            : chalk.yellow('SKIP');
      console.log(`Gherkin syntax: ${status}`);
    }

    if (result.tagStatus) {
      const status =
        result.tagStatus === 'PASS'
          ? chalk.green('PASS')
          : result.tagStatus === 'FAIL'
            ? chalk.red('FAIL')
            : chalk.yellow('SKIP');
      console.log(`Tag validation: ${status}`);
    }

    if (result.formatStatus) {
      const status =
        result.formatStatus === 'PASS'
          ? chalk.green('PASS')
          : result.formatStatus === 'FAIL'
            ? chalk.red('FAIL')
            : chalk.yellow('SKIP');
      console.log(`Formatting: ${status}`);
    }

    // Show errors if any
    if (result.errors && result.errors.length > 0) {
      console.log(chalk.red('\nErrors:'));
      for (const error of result.errors) {
        console.log(chalk.red(`  - ${error}`));
      }
    }

    // Show final message
    console.log();
    if (result.success) {
      console.log(chalk.green('✓'), result.message);
    } else {
      console.log(chalk.red('✗'), 'Some checks failed');
    }

    process.exit(result.success ? 0 : 1);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
