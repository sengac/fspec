import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { Tags } from '../types/tags';

interface TagValidationResult {
  file: string;
  valid: boolean;
  errors: Array<{
    tag: string;
    message: string;
    suggestion?: string;
  }>;
}

interface TagRegistry {
  validTags: Set<string>;
  requiredCategories: {
    phase: string[];
    component: string[];
    featureGroup: string[];
  };
}

export async function validateTags(
  options: { file?: string; cwd?: string } = {}
): Promise<{
  results: TagValidationResult[];
  validCount: number;
  invalidCount: number;
}> {
  const cwd = options.cwd || process.cwd();

  // Load tag registry from tags.json
  const registry = await loadTagRegistry(cwd);

  // Get files to validate
  const files = options.file
    ? [options.file]
    : await glob(['spec/features/**/*.feature'], { cwd, absolute: false });

  if (files.length === 0) {
    return { results: [], validCount: 0, invalidCount: 0 };
  }

  // Validate each file
  const results = await Promise.all(
    files.map(file => validateFileTags(file, registry, cwd))
  );

  const validCount = results.filter(r => r.valid).length;
  const invalidCount = results.length - validCount;

  return { results, validCount, invalidCount };
}

export async function validateTagsCommand(file?: string): Promise<void> {
  try {
    const { results, validCount, invalidCount } = await validateTags({ file });

    // Display results
    for (const result of results) {
      if (result.valid) {
        console.log(chalk.green(`✓ All tags in ${result.file} are registered`));
      } else {
        console.log(chalk.red(`✗ ${result.file} has tag violations:`));
        for (const error of result.errors) {
          console.log(chalk.red(`  ${error.message}`));
          if (error.suggestion) {
            console.log(chalk.yellow(`  Suggestion: ${error.suggestion}`));
          }
        }
      }
    }

    // Summary
    if (results.length > 1) {
      console.log('');
      if (invalidCount === 0) {
        console.log(chalk.green(`✓ ${validCount} files passed`));
      } else {
        console.log(chalk.green(`✓ ${validCount} files passed`));
        console.log(chalk.red(`✗ ${invalidCount} files have tag violations`));
      }
    }

    if (invalidCount > 0) {
      process.exit(1);
    } else {
      process.exit(0);
    }
  } catch (error: any) {
    if (error.message.includes('tags.json not found')) {
      console.error(chalk.red(error.message));
      console.log(
        chalk.yellow('  Suggestion: Create spec/tags.json to track tags')
      );
      process.exit(2);
    }
    console.error(chalk.red('Error:'), error.message);
    process.exit(2);
  }
}

async function loadTagRegistry(cwd: string): Promise<TagRegistry> {
  const tagsJsonPath = join(cwd, 'spec', 'tags.json');

  if (!existsSync(tagsJsonPath)) {
    throw new Error('tags.json not found: spec/tags.json');
  }

  try {
    const content = await readFile(tagsJsonPath, 'utf-8');
    const tagsData: Tags = JSON.parse(content);

    // Extract all valid tags from categories
    const validTags = new Set<string>();
    const phaseTags: string[] = [];
    const componentTags: string[] = [];
    const featureGroupTags: string[] = [];

    for (const category of tagsData.categories) {
      for (const tag of category.tags) {
        validTags.add(tag.name);

        // Categorize tags based on category name
        if (category.name === 'Phase Tags') {
          phaseTags.push(tag.name);
        } else if (category.name === 'Component Tags') {
          componentTags.push(tag.name);
        } else if (category.name === 'Feature Group Tags') {
          featureGroupTags.push(tag.name);
        }
      }
    }

    const requiredCategories = {
      phase: phaseTags,
      component: componentTags,
      featureGroup: featureGroupTags,
    };

    return { validTags, requiredCategories };
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new Error('tags.json not found: spec/tags.json');
    }
    throw error;
  }
}

async function validateFileTags(
  filePath: string,
  registry: TagRegistry,
  cwd: string
): Promise<TagValidationResult> {
  const result: TagValidationResult = {
    file: filePath,
    valid: true,
    errors: [],
  };

  try {
    // Read and parse the feature file
    const content = await readFile(join(cwd, filePath), 'utf-8');

    const uuidFn = Messages.IdGenerator.uuid();
    const builder = new Gherkin.AstBuilder(uuidFn);
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    let gherkinDocument;
    try {
      gherkinDocument = parser.parse(content);
    } catch {
      // If file doesn't parse, skip tag validation
      return result;
    }

    if (!gherkinDocument.feature) {
      return result;
    }

    // Extract tags from feature
    const tags = gherkinDocument.feature.tags.map(t => t.name);

    // Check for unregistered tags
    const unregisteredTags = tags.filter(tag => !registry.validTags.has(tag));
    if (unregisteredTags.length > 0) {
      result.valid = false;
      for (const tag of unregisteredTags) {
        // Check if it's a placeholder tag
        if (tag === '@component' || tag === '@feature-group') {
          result.errors.push({
            tag,
            message: `Placeholder tag: ${tag}`,
            suggestion: `Replace ${tag} with actual tags from tags.json`,
          });
        } else {
          result.errors.push({
            tag,
            message: `Unregistered tag: ${tag} in ${filePath}`,
            suggestion: `Register this tag in spec/tags.json or use 'fspec register-tag'`,
          });
        }
      }
    }

    // Check for required phase tag
    const hasPhaseTag = tags.some(tag =>
      registry.requiredCategories.phase.includes(tag)
    );
    if (!hasPhaseTag) {
      result.valid = false;
      result.errors.push({
        tag: '',
        message: 'Missing required phase tag (@phase1, @phase2, etc.)',
        suggestion: 'Add a phase tag to the feature',
      });
    }

    // Check for required component tag
    const hasComponentTag = tags.some(tag =>
      registry.requiredCategories.component.includes(tag)
    );
    if (!hasComponentTag && !tags.includes('@component')) {
      result.valid = false;
      result.errors.push({
        tag: '',
        message: 'Missing required component tag',
        suggestion: `Add one of: ${registry.requiredCategories.component.join(', ')}`,
      });
    }

    // Check for required feature-group tag
    const hasFeatureGroupTag = tags.some(tag =>
      registry.requiredCategories.featureGroup.includes(tag)
    );
    if (!hasFeatureGroupTag && !tags.includes('@feature-group')) {
      result.valid = false;
      result.errors.push({
        tag: '',
        message: 'Missing required feature-group tag',
        suggestion: `Add one of: ${registry.requiredCategories.featureGroup.join(', ')}`,
      });
    }
  } catch (error: any) {
    result.valid = false;
    result.errors.push({
      tag: '',
      message: error.message,
    });
  }

  return result;
}
