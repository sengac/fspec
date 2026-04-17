import { readFile } from 'fs/promises';
import { join } from 'path';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import {
  isWorkUnitTag,
  looksLikeWorkUnitTag,
  extractWorkUnitId,
} from '../utils/work-unit-tags';
import type { WorkUnitsData } from '../types';
import type { TagRegistry } from './validate-tags-registry';

/**
 * Result of validating the tags on a single feature file.
 */
export interface TagValidationResult {
  file: string;
  valid: boolean;
  errors: Array<{
    tag: string;
    message: string;
    suggestion?: string;
  }>;
}

/**
 * Validate a single feature file's feature-level and scenario-level tags
 * against the tag registry and (optionally) the work-units data. Unregistered
 * tags, placeholder tags, malformed work-unit tags, missing required category
 * tags, and misplaced scenario-level work-unit tags are all reported.
 *
 * @param filePath - Path to the feature file (relative to cwd).
 * @param registry - Tag registry produced by `loadTagRegistry`.
 * @param workUnitsData - Loaded work-units.json, or null if missing.
 * @param cwd - Project root for resolving `filePath`.
 * @returns A {@link TagValidationResult} describing validity + per-tag errors.
 */
export async function validateFileTags(
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

    const featureTags = gherkinDocument.feature.tags.map(t => t.name);
    const scenarioTags: string[] = [];
    for (const child of gherkinDocument.feature.children) {
      if (child.scenario) {
        scenarioTags.push(...child.scenario.tags.map(t => t.name));
      }
    }

    const tags = featureTags;

    validateUnregisteredFeatureTags(
      featureTags,
      registry,
      workUnitsData,
      filePath,
      result
    );
    validateScenarioTags(
      scenarioTags,
      registry,
      workUnitsData,
      filePath,
      result
    );
    validateRequiredCategoryTags(tags, registry, result);
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    result.valid = false;
    result.errors.push({
      tag: '',
      message,
    });
  }

  return result;
}

function validateUnregisteredFeatureTags(
  featureTags: string[],
  registry: TagRegistry,
  workUnitsData: WorkUnitsData | null,
  filePath: string,
  result: TagValidationResult
): void {
  const unregistered = featureTags.filter(tag => !registry.validTags.has(tag));
  if (unregistered.length === 0) {
    return;
  }

  for (const tag of unregistered) {
    if (isWorkUnitTag(tag)) {
      reportWorkUnitTag(tag, workUnitsData, result);
    } else if (looksLikeWorkUnitTag(tag)) {
      result.valid = false;
      result.errors.push({
        tag,
        message: `Invalid work unit tag format: ${tag}`,
        suggestion:
          'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
      });
    } else if (tag === '@component' || tag === '@feature-group') {
      result.valid = false;
      result.errors.push({
        tag,
        message: `Placeholder tag: ${tag}`,
        suggestion: `Replace ${tag} with actual tags from tags.json`,
      });
    } else {
      result.valid = false;
      result.errors.push({
        tag,
        message: `Unregistered tag: ${tag} in ${filePath}`,
        suggestion: `Register this tag in spec/tags.json or use 'fspec register-tag'`,
      });
    }
  }
}

function validateScenarioTags(
  scenarioTags: string[],
  registry: TagRegistry,
  workUnitsData: WorkUnitsData | null,
  filePath: string,
  result: TagValidationResult
): void {
  // CRITICAL: Reject scenario-level work unit ID tags (BUG-005)
  const scenarioWorkUnitTags = scenarioTags.filter(tag => isWorkUnitTag(tag));
  for (const tag of scenarioWorkUnitTags) {
    result.valid = false;
    result.errors.push({
      tag,
      message: `Work unit ID tag ${tag} must be at feature level, not scenario level`,
      suggestion: `Move ${tag} to feature-level tags. Use coverage files for fine-grained scenario traceability.`,
    });
  }

  const unregistered = scenarioTags.filter(tag => !registry.validTags.has(tag));
  for (const tag of unregistered) {
    if (isWorkUnitTag(tag)) {
      // Already handled above
      continue;
    }

    if (looksLikeWorkUnitTag(tag)) {
      result.valid = false;
      result.errors.push({
        tag,
        message: `Invalid work unit tag format: ${tag}`,
        suggestion:
          'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
      });
    } else {
      result.valid = false;
      result.errors.push({
        tag,
        message: `Unregistered tag: ${tag} in ${filePath}`,
        suggestion: `Register this tag in spec/tags.json or use 'fspec register-tag'`,
      });
    }
  }
}

function reportWorkUnitTag(
  tag: string,
  workUnitsData: WorkUnitsData | null,
  result: TagValidationResult
): void {
  const workUnitId = extractWorkUnitId(tag);

  if (!workUnitId) {
    result.valid = false;
    result.errors.push({
      tag,
      message: `Invalid work unit tag format: ${tag}`,
      suggestion:
        'Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)',
    });
    return;
  }

  if (!workUnitsData) {
    result.valid = false;
    result.errors.push({
      tag,
      message: `Work unit ${tag} found but spec/work-units.json does not exist`,
      suggestion: 'Create spec/work-units.json to define work units',
    });
    return;
  }

  if (!workUnitsData.workUnits[workUnitId]) {
    result.valid = false;
    result.errors.push({
      tag,
      message: `Work unit ${tag} not found in spec/work-units.json`,
      suggestion: `Add work unit ${workUnitId} to spec/work-units.json or use 'fspec create-story/create-bug/create-task'`,
    });
  }
}

function validateRequiredCategoryTags(
  tags: string[],
  registry: TagRegistry,
  result: TagValidationResult
): void {
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
}
