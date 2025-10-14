import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface RemoveTagFromFeatureOptions {
  cwd?: string;
}

interface RemoveTagFromFeatureResult {
  success: boolean;
  valid: boolean;
  message?: string;
  error?: string;
}

export async function removeTagFromFeature(
  featureFilePath: string,
  tags: string[],
  options: RemoveTagFromFeatureOptions = {}
): Promise<RemoveTagFromFeatureResult> {
  const cwd = options.cwd || process.cwd();
  const filePath = join(cwd, featureFilePath);

  // Read feature file
  let content: string;
  try {
    content = await readFile(filePath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        valid: false,
        error: `File not found: ${featureFilePath}`,
      };
    }
    throw error;
  }

  // Parse Gherkin to get existing tags
  const uuidFn = Messages.IdGenerator.uuid();
  const builder = new Gherkin.AstBuilder(uuidFn);
  const matcher = new Gherkin.GherkinClassicTokenMatcher();
  const parser = new Gherkin.Parser(builder, matcher);

  let gherkinDocument;
  try {
    gherkinDocument = parser.parse(content);
  } catch (error: any) {
    return {
      success: false,
      valid: false,
      error: `Invalid Gherkin syntax: ${error.message}`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      valid: false,
      error: 'File does not contain a valid Feature',
    };
  }

  // Get existing feature-level tags
  const existingTags = gherkinDocument.feature.tags.map(t => t.name);

  // Check if tags exist
  for (const tag of tags) {
    if (!existingTags.includes(tag)) {
      return {
        success: false,
        valid: false,
        error: `Tag ${tag} not found on this feature`,
      };
    }
  }

  // Remove tags from the file
  const lines = content.split('\n');
  const tagsToRemove = new Set(tags);
  const filteredLines: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    // Skip lines that are tags we want to remove
    if (trimmed.startsWith('@') && tagsToRemove.has(trimmed)) {
      continue;
    }
    filteredLines.push(line);
  }

  const newContent = filteredLines.join('\n');

  // Validate the result is valid Gherkin
  let valid = true;
  try {
    const testBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const testMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const testParser = new Gherkin.Parser(testBuilder, testMatcher);
    testParser.parse(newContent);
  } catch {
    valid = false;
  }

  // Write file
  await writeFile(filePath, newContent, 'utf-8');

  const tagList = tags.join(', ');
  return {
    success: true,
    valid,
    message: `Removed ${tagList} from ${featureFilePath}`,
  };
}

export async function removeTagFromFeatureCommand(
  featureFilePath: string,
  tags: string[]
): Promise<void> {
  try {
    const result = await removeTagFromFeature(featureFilePath, tags);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green(`✓ ${result.message}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerRemoveTagFromFeatureCommand(program: Command): void {
  program
    .command('remove-tag-from-feature')
    .description('Remove one or more tags from a feature file')
    .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
    .argument('<tags...>', 'Tag(s) to remove (e.g., @deprecated @wip)')
    .action(async (file: string, tags: string[]) => {
      await removeTagFromFeatureCommand(file, tags);
    });
}
