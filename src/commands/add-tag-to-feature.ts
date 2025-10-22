import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { isWorkUnitTag } from '../utils/work-unit-tags';
import {
  getUnregisteredTagReminder,
  getMissingRequiredTagsReminder,
} from '../utils/system-reminder';

interface AddTagToFeatureOptions {
  cwd?: string;
  validateRegistry?: boolean;
}

interface AddTagToFeatureResult {
  success: boolean;
  valid: boolean;
  message?: string;
  error?: string;
  systemReminders?: string[];
}

export async function addTagToFeature(
  featureFilePath: string,
  tags: string[],
  options: AddTagToFeatureOptions = {}
): Promise<AddTagToFeatureResult> {
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

  // Validate tag formats
  for (const tag of tags) {
    if (!tag.startsWith('@')) {
      return {
        success: false,
        valid: false,
        error: `Invalid tag format. Tags must start with @`,
      };
    }
    // Allow work unit tags (@AUTH-001) or regular tags (@lowercase-with-hyphens)
    const isWorkUnit = isWorkUnitTag(tag);
    const isRegularTag = /^@[a-z0-9-#]+$/.test(tag);

    if (!isWorkUnit && !isRegularTag) {
      return {
        success: false,
        valid: false,
        error: `Invalid tag format. Regular tags must use lowercase-with-hyphens, work unit tags must match @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001)`,
      };
    }
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

  // Check for duplicates
  for (const tag of tags) {
    if (existingTags.includes(tag)) {
      return {
        success: false,
        valid: false,
        error: `Tag ${tag} already exists on this feature`,
      };
    }
  }

  // Validate against registry if requested
  if (options.validateRegistry) {
    try {
      const tagsJsonPath = join(cwd, 'spec', 'tags.json');
      const tagsJson = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const registeredTags = new Set<string>();

      for (const category of tagsJson.categories) {
        for (const tag of category.tags) {
          registeredTags.add(tag.name);
        }
      }

      for (const tag of tags) {
        if (!registeredTags.has(tag)) {
          return {
            success: false,
            valid: false,
            error: `Tag ${tag} is not registered in spec/tags.json`,
          };
        }
      }
    } catch (error: any) {
      return {
        success: false,
        valid: false,
        error: `Failed to validate against registry: ${error.message}`,
      };
    }
  }

  // Add tags to the beginning of the file (before Feature keyword)
  const lines = content.split('\n');
  let featureLineIndex = -1;

  // Find the Feature line
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith('Feature:')) {
      featureLineIndex = i;
      break;
    }
  }

  if (featureLineIndex === -1) {
    return {
      success: false,
      valid: false,
      error: 'Could not find Feature keyword in file',
    };
  }

  // Find where existing tags end (or where to insert new tags)
  let insertIndex = featureLineIndex;
  for (let i = featureLineIndex - 1; i >= 0; i--) {
    const trimmed = lines[i].trim();
    if (!trimmed.startsWith('@') && trimmed !== '') {
      // Found non-tag, non-empty line
      insertIndex = i + 1;
      break;
    }
    if (i === 0) {
      insertIndex = 0;
      break;
    }
  }

  // If all lines before Feature are tags or empty, insert at the end of tags
  if (insertIndex === featureLineIndex && existingTags.length > 0) {
    // Find last tag line
    for (let i = featureLineIndex - 1; i >= 0; i--) {
      const trimmed = lines[i].trim();
      if (trimmed.startsWith('@')) {
        insertIndex = i + 1;
        break;
      }
    }
  }

  // Insert new tags
  const tagLines = tags.map(tag => tag);
  lines.splice(insertIndex, 0, ...tagLines);

  const newContent = lines.join('\n');

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

  // Check for system reminders
  const systemReminders: string[] = [];

  // Check if tags are registered (unless validateRegistry was true, which already validated)
  if (!options.validateRegistry) {
    try {
      const tagsJsonPath = join(cwd, 'spec', 'tags.json');
      const tagsJson = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const registeredTags = new Set<string>();

      for (const category of tagsJson.categories) {
        for (const tag of category.tags) {
          registeredTags.add(tag.name);
        }
      }

      // Check each new tag
      for (const tag of tags) {
        // Skip work unit tags - they don't need to be registered
        if (isWorkUnitTag(tag)) {
          continue;
        }

        const isRegistered = registeredTags.has(tag);
        const reminder = getUnregisteredTagReminder(tag, isRegistered);
        if (reminder) {
          systemReminders.push(reminder);
        }
      }
    } catch {
      // If tags.json doesn't exist or is malformed, skip reminder check
    }
  }

  // Check for missing required tags
  const allTags = [...existingTags, ...tags];
  const hasComponentTag = allTags.some(t =>
    [
      '@cli',
      '@parser',
      '@validator',
      '@formatter',
      '@generator',
      '@file-ops',
    ].includes(t)
  );
  const hasFeatureGroupTag = allTags.some(t =>
    [
      '@feature-management',
      '@tag-management',
      '@validation',
      '@querying',
      '@work-unit-management',
      '@example-mapping',
      '@metrics',
      '@dependency-management',
      '@workflow',
    ].includes(t)
  );

  const missingTags: string[] = [];
  if (!hasComponentTag) {
    missingTags.push('component');
  }
  if (!hasFeatureGroupTag) {
    missingTags.push('feature-group');
  }

  if (missingTags.length > 0) {
    const reminder = getMissingRequiredTagsReminder(
      featureFilePath,
      missingTags
    );
    if (reminder) {
      systemReminders.push(reminder);
    }
  }

  const tagList = tags.join(', ');
  return {
    success: true,
    valid,
    message: `Added ${tagList} to ${featureFilePath}`,
    ...(systemReminders.length > 0 && { systemReminders }),
  };
}

export async function addTagToFeatureCommand(
  featureFilePath: string,
  tags: string[],
  options: { validateRegistry?: boolean } = {}
): Promise<void> {
  try {
    const result = await addTagToFeature(featureFilePath, tags, options);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green(`✓ ${result.message}`));

    // Display system reminders if any
    if (result.systemReminders && result.systemReminders.length > 0) {
      for (const reminder of result.systemReminders) {
        console.log('\n' + reminder);
      }
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerAddTagToFeatureCommand(program: Command): void {
  program
    .command('add-tag-to-feature')
    .description('Add one or more tags to a feature file')
    .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
    .argument('<tags...>', 'Tag(s) to add (e.g., @critical @security)')
    .option('--validate-registry', 'Validate tags against spec/tags.json')
    .action(
      async (
        file: string,
        tags: string[],
        options: { validateRegistry?: boolean }
      ) => {
        await addTagToFeatureCommand(file, tags, options);
      }
    );
}
