import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { Tags } from '../types/tags';
import {
  isWorkUnitTag,
  looksLikeWorkUnitTag,
  extractWorkUnitId,
  loadWorkUnitsData,
} from '../utils/work-unit-tags';
import type { WorkUnitsData } from '../types';
import { ensureTagsFile } from '../utils/ensure-files';

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

  // Load work units data from work-units.json (optional)
  const workUnitsData = await loadWorkUnitsData(cwd);

  // Get files to validate
  const files = options.file
    ? [options.file]
    : await glob(['spec/features/**/*.feature'], { cwd, absolute: false });

  if (files.length === 0) {
    return { results: [], validCount: 0, invalidCount: 0 };
  }

  // Validate each file
  const results = await Promise.all(
    files.map(file => validateFileTags(file, registry, workUnitsData, cwd))
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
    console.error(chalk.red('Error:'), error.message);
    process.exit(2);
  }
}

async function loadTagRegistry(cwd: string): Promise<TagRegistry> {
  // Load or create tags.json using ensureTagsFile
  const tagsData: Tags = await ensureTagsFile(cwd);

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
}

async function validateFileTags(
  filePath: string,
  registry: TagRegistry,
  workUnitsData: WorkUnitsData | null,
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
    const featureTags = gherkinDocument.feature.tags.map(t => t.name);

    // Extract tags from scenarios
    const scenarioTags: string[] = [];
    for (const child of gherkinDocument.feature.children) {
      if (child.scenario) {
        scenarioTags.push(...child.scenario.tags.map(t => t.name));
      }
    }

    // Combine all tags for validation (feature tags + scenario tags)
    const allTags = [...featureTags, ...scenarioTags];
    const tags = featureTags; // Only feature tags for required category checks

    // Check for unregistered FEATURE-LEVEL tags (strict validation)
    const unregisteredFeatureTags = featureTags.filter(
      tag => !registry.validTags.has(tag)
    );
    if (unregisteredFeatureTags.length > 0) {
      for (const tag of unregisteredFeatureTags) {
        // Check if it's a valid work unit tag
        if (isWorkUnitTag(tag)) {
          const workUnitId = extractWorkUnitId(tag);

          if (!workUnitId) {
            // Invalid work unit tag format (this shouldn't happen if isWorkUnitTag returns true)
            result.valid = false;
            result.errors.push({
              tag,
              message: `Invalid work unit tag format: ${tag}`,
              suggestion:
                'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
            });
          } else if (!workUnitsData) {
            // No work-units.json file
            result.valid = false;
            result.errors.push({
              tag,
              message: `Work unit ${tag} found but spec/work-units.json does not exist`,
              suggestion: 'Create spec/work-units.json to define work units',
            });
          } else if (!workUnitsData.workUnits[workUnitId]) {
            // Work unit doesn't exist
            result.valid = false;
            result.errors.push({
              tag,
              message: `Work unit ${tag} not found in spec/work-units.json`,
              suggestion: `Add work unit ${workUnitId} to spec/work-units.json or use 'fspec create-work-unit'`,
            });
          }
          // If work unit exists and format is valid, it's valid - no error
        } else if (looksLikeWorkUnitTag(tag)) {
          // Tag looks like work unit tag but has invalid format (e.g., lowercase)
          result.valid = false;
          result.errors.push({
            tag,
            message: `Invalid work unit tag format: ${tag}`,
            suggestion:
              'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
          });
        } else if (tag === '@component' || tag === '@feature-group') {
          // Check if it's a placeholder tag
          result.valid = false;
          result.errors.push({
            tag,
            message: `Placeholder tag: ${tag}`,
            suggestion: `Replace ${tag} with actual tags from tags.json`,
          });
        } else {
          // Regular unregistered tag
          result.valid = false;
          result.errors.push({
            tag,
            message: `Unregistered tag: ${tag} in ${filePath}`,
            suggestion: `Register this tag in spec/tags.json or use 'fspec register-tag'`,
          });
        }
      }
    }

    // Check SCENARIO-LEVEL tags (validate work unit tags for traceability)
    // CRITICAL: Reject scenario-level work unit ID tags (BUG-005)
    const scenarioWorkUnitTags = scenarioTags.filter(tag => isWorkUnitTag(tag));
    if (scenarioWorkUnitTags.length > 0) {
      for (const tag of scenarioWorkUnitTags) {
        result.valid = false;
        result.errors.push({
          tag,
          message: `Work unit ID tag ${tag} must be at feature level, not scenario level`,
          suggestion: `Move ${tag} to feature-level tags. Use coverage files for fine-grained scenario traceability.`,
        });
      }
    }

    const unregisteredScenarioTags = scenarioTags.filter(
      tag => !registry.validTags.has(tag)
    );
    if (unregisteredScenarioTags.length > 0) {
      for (const tag of unregisteredScenarioTags) {
        // Skip work unit tags (already validated above)
        if (isWorkUnitTag(tag)) {
          // Already handled in scenario-level work unit tag check above
          continue;
        }

        // Scenario-level work unit tags must exist in work-units.json
        if (isWorkUnitTag(tag)) {
          const workUnitId = extractWorkUnitId(tag);

          if (!workUnitId) {
            // Invalid work unit tag format
            result.valid = false;
            result.errors.push({
              tag,
              message: `Invalid work unit tag format: ${tag}`,
              suggestion:
                'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
            });
          } else if (!workUnitsData) {
            // No work-units.json file
            result.valid = false;
            result.errors.push({
              tag,
              message: `Work unit ${tag} found but spec/work-units.json does not exist`,
              suggestion: 'Create spec/work-units.json to define work units',
            });
          } else if (!workUnitsData.workUnits[workUnitId]) {
            // Work unit doesn't exist
            result.valid = false;
            result.errors.push({
              tag,
              message: `Work unit ${tag} not found in spec/work-units.json`,
              suggestion: `Add work unit ${workUnitId} to spec/work-units.json or use 'fspec create-work-unit'`,
            });
          }
          // If work unit exists and format is valid, continue
          continue;
        } else if (looksLikeWorkUnitTag(tag)) {
          // Tag looks like work unit tag but has invalid format
          result.valid = false;
          result.errors.push({
            tag,
            message: `Invalid work unit tag format: ${tag}`,
            suggestion:
              'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
          });
        } else {
          // Regular unregistered tag at scenario level
          result.valid = false;
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
