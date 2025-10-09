import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

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

  // Load tag registry from TAGS.md
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
    if (error.message.includes('TAGS.md not found')) {
      console.error(chalk.red(error.message));
      console.log(
        chalk.yellow('  Suggestion: Create spec/TAGS.md to track tags')
      );
      process.exit(2);
    }
    console.error(chalk.red('Error:'), error.message);
    process.exit(2);
  }
}

async function loadTagRegistry(cwd: string): Promise<TagRegistry> {
  const tagsPath = join(cwd, 'spec', 'TAGS.md');

  try {
    const content = await readFile(tagsPath, 'utf-8');

    // Extract all tags from TAGS.md (lines starting with | `@tag` |)
    const tagPattern = /\|\s*`(@[a-z0-9-]+)`\s*\|/g;
    const validTags = new Set<string>();

    let match;
    while ((match = tagPattern.exec(content)) !== null) {
      validTags.add(match[1]);
    }

    // Define required tag categories - extract from validTags based on patterns
    // Phase tags: @phase1, @phase2, @phase3
    const phaseTags = Array.from(validTags).filter(tag =>
      tag.startsWith('@phase')
    );

    // Component tags: defined in TAGS.md Component Tags section
    const componentTags = Array.from(validTags).filter(tag =>
      [
        '@cli',
        '@parser',
        '@generator',
        '@validator',
        '@formatter',
        '@file-ops',
        '@integration',
      ].includes(tag)
    );

    // Feature group tags: all other registered tags that aren't phase/component/optional
    const featureGroupTags = Array.from(validTags).filter(
      tag =>
        !tag.startsWith('@phase') &&
        !componentTags.includes(tag) &&
        ![
          '@gherkin',
          '@cucumber-parser',
          '@prettier',
          '@mermaid',
          '@ast',
          '@error-handling',
          '@file-system',
          '@template',
          '@windows',
          '@macos',
          '@linux',
          '@cross-platform',
          '@critical',
          '@high',
          '@medium',
          '@low',
          '@wip',
          '@todo',
          '@done',
          '@deprecated',
          '@blocked',
          '@unit-test',
          '@integration-test',
          '@e2e-test',
          '@manual-test',
          '@cage-hook',
          '@execa',
          '@acdd',
          '@spec-alignment',
        ].includes(tag)
    );

    const requiredCategories = {
      phase: phaseTags,
      component: componentTags,
      featureGroup: featureGroupTags,
    };

    return { validTags, requiredCategories };
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new Error('TAGS.md not found: spec/TAGS.md');
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
            suggestion: `Replace ${tag} with actual tags from TAGS.md`,
          });
        } else {
          result.errors.push({
            tag,
            message: `Unregistered tag: ${tag} in ${filePath}`,
            suggestion: `Register this tag in spec/TAGS.md or use 'fspec register-tag'`,
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
