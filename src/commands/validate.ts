import { readFile } from 'fs/promises';
import { resolve, relative } from 'path';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { Command } from 'commander';
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

export async function validateCommand(
  file?: string,
  options?: { verbose?: boolean }
): Promise<void> {
  try {
    const files = file ? [file] : await findAllFeatureFiles();

    if (files.length === 0) {
      console.error(chalk.red('No feature files found in spec/features/'));
      process.exit(2);
    }

    const results = await Promise.all(
      files.map(f => validateFile(f, options?.verbose))
    );

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
        console.log(
          chalk.green(`✓ All ${results.length} feature files are valid`)
        );
      } else {
        console.log(
          chalk.yellow(
            `Validated ${results.length} files: ${validCount} valid, ${invalidCount} invalid`
          )
        );
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

async function validateFile(
  filePath: string,
  verbose?: boolean
): Promise<ValidationResult> {
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

    // Additional validation checks
    const additionalErrors = checkForCommonIssues(content);
    if (additionalErrors.length > 0) {
      result.valid = false;
      result.errors.push(...additionalErrors);
    }

    // Validation successful
    if (verbose) {
      console.log(chalk.blue('  AST generated successfully'));
      if (gherkinDocument.feature) {
        console.log(chalk.blue(`  Feature: ${gherkinDocument.feature.name}`));
        console.log(
          chalk.blue(`  Scenarios: ${gherkinDocument.feature.children.length}`)
        );
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

function checkForCommonIssues(
  content: string
): Array<{ line: number; message: string; suggestion?: string }> {
  const errors: Array<{ line: number; message: string; suggestion?: string }> =
    [];
  const lines = content.split('\n');

  let inDocString = false;
  let docStringStartLine = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNum = i + 1;

    // Track DocString boundaries
    if (line.trim() === '"""' || line.trim().startsWith('"""')) {
      if (inDocString) {
        inDocString = false;
      } else {
        inDocString = true;
        docStringStartLine = lineNum;
      }
      continue;
    }

    // Check for unescaped triple quotes inside DocStrings
    if (inDocString && line.includes('"""') && !line.includes('\\"""')) {
      errors.push({
        line: lineNum,
        message: 'Unescaped triple quotes (""") found inside DocString',
        suggestion:
          'Escape triple quotes with backslashes: \\"\\"\\", or use triple backticks (```) as DocString delimiters instead',
      });
    }

    // Check for excessive consecutive blank lines (more than 2)
    if (
      i >= 2 &&
      lines[i].trim() === '' &&
      lines[i - 1].trim() === '' &&
      lines[i - 2].trim() === ''
    ) {
      // Count how many consecutive blank lines
      let blankCount = 3;
      for (let j = i + 1; j < lines.length && lines[j].trim() === ''; j++) {
        blankCount++;
      }
      if (blankCount >= 3) {
        errors.push({
          line: lineNum,
          message: `Excessive blank lines detected (${blankCount} consecutive blank lines)`,
          suggestion:
            'Remove excess blank lines - Gherkin files should have at most 2 consecutive blank lines',
        });
        // Skip ahead to avoid duplicate errors
        i += blankCount - 3;
      }
    }
  }

  return errors;
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

export function registerValidateCommand(program: Command): void {
  program
    .command('validate')
    .description('Validate Gherkin syntax in feature files')
    .argument(
      '[file]',
      'Feature file to validate (validates all if not specified)'
    )
    .option('-v, --verbose', 'Show detailed validation output', false)
    .action(validateCommand);
}
