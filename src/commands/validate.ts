import { readFile } from 'fs/promises';
import { resolve, relative } from 'path';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import chalk from 'chalk';
import { glob } from 'tinyglobby';

interface ValidationResult {
  file: string;
  valid: boolean;
  errors: Array<{
    line: number;
    message: string;
    suggestion?: string;
  }>;
}

export async function validateCommand(file?: string, options?: { verbose?: boolean }): Promise<void> {
  try {
    const files = file ? [file] : await findAllFeatureFiles();

    if (files.length === 0) {
      console.error(chalk.red('No feature files found in spec/features/'));
      process.exit(2);
    }

    const results = await Promise.all(files.map(f => validateFile(f, options?.verbose)));

    // Display results
    for (const result of results) {
      if (result.valid) {
        console.log(chalk.green(`✓ ${result.file} is valid`));
      } else {
        console.log(chalk.red(`✗ ${result.file} has syntax errors:`));
        for (const error of result.errors) {
          console.log(chalk.red(`  Line ${error.line}: ${error.message}`));
          if (error.suggestion) {
            console.log(chalk.yellow(`  Suggestion: ${error.suggestion}`));
          }
        }
      }
    }

    // Summary
    const validCount = results.filter(r => r.valid).length;
    const invalidCount = results.length - validCount;

    if (results.length > 1) {
      console.log('');
      if (invalidCount === 0) {
        console.log(chalk.green(`✓ All ${results.length} feature files are valid`));
      } else {
        console.log(chalk.yellow(`Validated ${results.length} files: ${validCount} valid, ${invalidCount} invalid`));
      }
    }

    // Exit with appropriate code
    if (invalidCount > 0) {
      process.exit(1);
    }
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(2);
  }
}

async function validateFile(filePath: string, verbose?: boolean): Promise<ValidationResult> {
  const result: ValidationResult = {
    file: filePath,
    valid: true,
    errors: [],
  };

  try {
    // Check file exists
    const resolvedPath = resolve(process.cwd(), filePath);
    const content = await readFile(resolvedPath, 'utf-8');

    if (verbose) {
      console.log(chalk.blue(`Parsing ${filePath}...`));
    }

    // Parse with @cucumber/gherkin
    const uuidFn = Messages.IdGenerator.uuid();
    const builder = new Gherkin.AstBuilder(uuidFn);
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    let gherkinDocument;
    try {
      gherkinDocument = parser.parse(content);
    } catch (parseError: any) {
      result.valid = false;
      result.errors.push({
        line: parseError.location?.line || 0,
        message: parseError.message,
        suggestion: getSuggestion(parseError.message),
      });
      return result;
    }

    // Validation successful
    if (verbose) {
      console.log(chalk.blue('  AST generated successfully'));
      if (gherkinDocument.feature) {
        console.log(chalk.blue(`  Feature: ${gherkinDocument.feature.name}`));
        console.log(chalk.blue(`  Scenarios: ${gherkinDocument.feature.children.length}`));
      }
    }

  } catch (error: any) {
    if (error.code === 'ENOENT') {
      result.valid = false;
      result.errors.push({
        line: 0,
        message: `File not found: ${filePath}`,
      });
    } else {
      result.valid = false;
      result.errors.push({
        line: 0,
        message: error.message,
      });
    }
  }

  return result;
}

async function findAllFeatureFiles(): Promise<string[]> {
  const cwd = process.cwd();
  const pattern = 'spec/features/**/*.feature';

  const files = await glob([pattern], {
    cwd,
    absolute: false,
  });

  return files;
}

function getSuggestion(errorMessage: string): string | undefined {
  const message = errorMessage.toLowerCase();

  if (message.includes('expected') && message.includes('feature')) {
    return 'Add Feature keyword at the beginning of the file';
  }

  if (message.includes('unexpected') || message.includes('invalid')) {
    if (message.includes('while') || message.includes('whilst')) {
      return 'Use: Given, When, Then, And, or But';
    }
    if (message.includes('indent')) {
      return 'Check indentation - steps should be indented 2 spaces from Scenario';
    }
  }

  if (message.includes('doc string') || message.includes('"""')) {
    return 'Add closing """';
  }

  if (message.includes('table')) {
    return 'Check data table formatting - each row must have same number of columns';
  }

  return undefined;
}
