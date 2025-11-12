/**
 * Shared utility for parsing Gherkin feature files
 * Part of QRY-002: Enhanced search and comparison commands
 */

import { readFile } from 'fs/promises';
import { glob } from 'tinyglobby';
import { join } from 'path';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { WorkUnit } from '../types';

interface ParsedFeature {
  filePath: string;
  feature: Messages.Feature;
  scenarios: Messages.Scenario[];
  workUnitId?: string;
}

const uuidFn = Messages.IdGenerator.uuid();
const builder = new Gherkin.AstBuilder(uuidFn);
const matcher = new Gherkin.GherkinClassicTokenMatcher();

/**
 * Parse all feature files in the spec/features directory
 */
export async function parseAllFeatures(cwd?: string): Promise<ParsedFeature[]> {
  const basePath = cwd || process.cwd();
  const featuresDir = join(basePath, 'spec', 'features');

  // Find all .feature files
  const featureFiles = await glob(['*.feature'], {
    cwd: featuresDir,
    absolute: false,
  });

  const parsedFeatures: ParsedFeature[] = [];

  for (const file of featureFiles) {
    const filePath = join(featuresDir, file);
    const content = await readFile(filePath, 'utf-8');

    try {
      const parser = new Gherkin.Parser(builder, matcher);
      const gherkinDocument = parser.parse(content);

      if (gherkinDocument.feature) {
        const feature = gherkinDocument.feature;
        const scenarios: Messages.Scenario[] = [];

        // Extract scenarios from feature
        for (const child of feature.children) {
          if (child.scenario) {
            scenarios.push(child.scenario);
          }
        }

        // Extract work unit ID from tags
        let workUnitId: string | undefined;
        for (const tag of feature.tags) {
          if (tag.name.match(/^@[A-Z]+-\d+$/)) {
            workUnitId = tag.name.substring(1); // Remove @ prefix
            break;
          }
        }

        parsedFeatures.push({
          filePath: join('spec', 'features', file),
          feature,
          scenarios,
          workUnitId,
        });
      }
    } catch (error) {
      // Skip files with parse errors
      continue;
    }
  }

  return parsedFeatures;
}

/**
 * Helper function to check if text matches query (literal or regex)
 */
function matchesQuery(
  text: string,
  query: string,
  pattern: RegExp | null
): boolean {
  return pattern
    ? pattern.test(text)
    : text.toLowerCase().includes(query.toLowerCase());
}

/**
 * Search scenarios by text or regex pattern (includes work unit titles)
 */
export async function searchScenarios(
  parsedFeatures: ParsedFeature[],
  query: string,
  useRegex: boolean = false,
  cwd?: string
): Promise<
  Array<{
    scenarioName: string;
    featureFilePath: string;
    workUnitId: string;
  }>
> {
  const results: Array<{
    scenarioName: string;
    featureFilePath: string;
    workUnitId: string;
  }> = [];

  // Validate regex pattern if useRegex is true
  let pattern: RegExp | null = null;
  if (useRegex) {
    try {
      pattern = new RegExp(query, 'i');
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      throw new Error(`Invalid regex pattern: "${query}". ${errorMessage}`);
    }
  }

  // BUG-059: Load work units to search work unit titles
  const basePath = cwd || process.cwd();
  const workUnitsPath = join(basePath, 'spec', 'work-units.json');
  let workUnits: Record<string, WorkUnit> = {};

  try {
    const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
    const workUnitsData = JSON.parse(workUnitsContent);
    workUnits = workUnitsData.workUnits || {};
  } catch (error) {
    // If work-units.json doesn't exist or is invalid, continue without work unit title search
  }

  // BUG-059: Search across feature file names, feature names, descriptions, work unit titles, and scenario names
  for (const parsed of parsedFeatures) {
    // Check if feature file path, feature name, or description matches
    const featureName = parsed.feature.name || '';
    const featureDescription = parsed.feature.description || '';
    const featureFilePath = parsed.filePath || '';

    // BUG-059: Check work unit title if workUnitId exists
    const workUnitTitle =
      parsed.workUnitId && workUnits[parsed.workUnitId]
        ? workUnits[parsed.workUnitId].title
        : '';

    const featureMatches =
      matchesQuery(featureName, query, pattern) ||
      matchesQuery(featureDescription, query, pattern) ||
      matchesQuery(featureFilePath, query, pattern) ||
      matchesQuery(workUnitTitle, query, pattern);

    // If feature matches, return all scenarios from this feature
    if (featureMatches) {
      for (const scenario of parsed.scenarios) {
        results.push({
          scenarioName: scenario.name,
          featureFilePath: parsed.filePath,
          workUnitId: parsed.workUnitId || 'unknown',
        });
      }
    } else {
      // Check individual scenarios
      for (const scenario of parsed.scenarios) {
        const scenarioName = scenario.name;

        if (matchesQuery(scenarioName, query, pattern)) {
          results.push({
            scenarioName,
            featureFilePath: parsed.filePath,
            workUnitId: parsed.workUnitId || 'unknown',
          });
        }
      }
    }
  }

  return results;
}

/**
 * Parse a single feature file and extract scenario steps
 *
 * @param featureFilePath - Absolute path to feature file
 * @param scenarioName - Name of scenario to extract steps from
 * @returns Array of step strings (e.g., ["Given I am on the login page", "When I click login"])
 */
export async function getScenarioSteps(
  featureFilePath: string,
  scenarioName: string
): Promise<string[]> {
  const content = await readFile(featureFilePath, 'utf-8');

  const parser = new Gherkin.Parser(builder, matcher);
  const gherkinDocument = parser.parse(content);

  if (!gherkinDocument.feature) {
    throw new Error(`No feature found in ${featureFilePath}`);
  }

  // Find the scenario
  for (const child of gherkinDocument.feature.children) {
    if (child.scenario && child.scenario.name === scenarioName) {
      const steps: string[] = [];

      for (const step of child.scenario.steps) {
        // Format: "Given I am on the login page"
        steps.push(`${step.keyword.trim()} ${step.text}`);
      }

      return steps;
    }
  }

  throw new Error(`Scenario "${scenarioName}" not found in ${featureFilePath}`);
}

/**
 * Find all feature files tagged with a specific work unit ID
 * Part of COV-054: Auto-discover features from @TAG when linkedFeatures is empty
 *
 * @param workUnitId - Work unit ID to search for (e.g., "AUTH-001")
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns Array of feature names (without .feature extension)
 */
export async function findFeaturesByTag(
  workUnitId: string,
  cwd?: string
): Promise<string[]> {
  const basePath = cwd || process.cwd();
  const featuresDir = join(basePath, 'spec', 'features');

  const featureNames: string[] = [];

  try {
    // Find all .feature files
    const featureFiles = await glob(['*.feature'], {
      cwd: featuresDir,
      absolute: false,
    });

    // Parse each feature file and check for work unit tag
    for (const file of featureFiles) {
      const filePath = join(featuresDir, file);
      const content = await readFile(filePath, 'utf-8');

      try {
        const parser = new Gherkin.Parser(builder, matcher);
        const gherkinDocument = parser.parse(content);

        if (gherkinDocument.feature) {
          // Check feature-level tags for @WORK-UNIT-ID
          for (const tag of gherkinDocument.feature.tags) {
            if (tag.name === `@${workUnitId}`) {
              // Extract feature name from filename (remove .feature extension)
              const featureName = file.replace('.feature', '');
              featureNames.push(featureName);
              break; // Found the tag, no need to check other tags
            }
          }
        }
      } catch {
        // Skip files with parse errors
        continue;
      }
    }
  } catch {
    // If features directory doesn't exist, return empty array
    return [];
  }

  return featureNames;
}
