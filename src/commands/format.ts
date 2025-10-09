import { access, readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { formatGherkinDocument } from '../utils/gherkin-formatter';

export interface FormatOptions {
  cwd?: string;
  file?: string;
}

export interface FormatResult {
  formattedCount: number;
}

export async function formatFeatures(
  options: FormatOptions = {}
): Promise<FormatResult> {
  const cwd = options.cwd || process.cwd();

  let files: string[];

  if (options.file) {
    // Format specific file
    const filePath = join(cwd, options.file);
    try {
      await access(filePath);
      files = [options.file];
    } catch (error) {
      throw new Error(`File not found: ${options.file}`);
    }
  } else {
    // Format all feature files
    files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    if (files.length === 0) {
      return { formattedCount: 0 };
    }
  }

  try {
    // Create Gherkin parser
    const uuidFn = Messages.IdGenerator.uuid();
    const builder = new Gherkin.AstBuilder(uuidFn);
    const matcher = new Gherkin.GherkinClassicTokenMatcher();

    // Format each file individually to ensure consistent counting
    let formattedCount = 0;

    for (const file of files) {
      const absoluteFilePath = join(cwd, file);

      try {
        // Read file content
        const content = await readFile(absoluteFilePath, 'utf-8');

        // Parse Gherkin to AST
        const parser = new Gherkin.Parser(builder, matcher);
        const gherkinDocument = parser.parse(content);

        // Format using custom formatter
        const formatted = formatGherkinDocument(gherkinDocument);

        // Write formatted content back
        await writeFile(absoluteFilePath, formatted, 'utf-8');

        formattedCount++;
      } catch (parseError: any) {
        // If parsing fails, skip this file and continue
        console.error(
          chalk.yellow(`Warning: Skipped ${file} due to parse error:`)
        );
        console.error(chalk.gray(parseError.message));
      }
    }

    return { formattedCount };
  } catch (error: any) {
    throw new Error(`Failed to format files: ${error.message}`);
  }
}

export async function formatCommand(file?: string): Promise<void> {
  try {
    const result = await formatFeatures({ file });

    if (result.formattedCount === 0) {
      console.log(chalk.yellow('No feature files found to format'));
      process.exit(0);
    }

    if (file) {
      console.log(chalk.green(`✓ Formatted ${file}`));
    } else {
      console.log(
        chalk.green(`✓ Formatted ${result.formattedCount} feature files`)
      );
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
