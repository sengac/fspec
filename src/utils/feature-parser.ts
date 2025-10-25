/**
 * Shared utility for parsing Gherkin feature files
 * Part of QRY-002: Enhanced search and comparison commands
 */

import { readFile } from 'fs/promises';
import { glob } from 'tinyglobby';
import { join } from 'path';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

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
 * Search scenarios by text or regex pattern
 */
export function searchScenarios(
  parsedFeatures: ParsedFeature[],
  query: string,
  useRegex: boolean = false
): Array<{
  scenarioName: string;
  featureFilePath: string;
  workUnitId: string;
}> {
  const results: Array<{
    scenarioName: string;
    featureFilePath: string;
    workUnitId: string;
  }> = [];

  const pattern = useRegex ? new RegExp(query, 'i') : null;

  for (const parsed of parsedFeatures) {
    for (const scenario of parsed.scenarios) {
      const scenarioName = scenario.name;
      const matches = pattern
        ? pattern.test(scenarioName)
        : scenarioName.toLowerCase().includes(query.toLowerCase());

      if (matches) {
        results.push({
          scenarioName,
          featureFilePath: parsed.filePath,
          workUnitId: parsed.workUnitId || 'unknown',
        });
      }
    }
  }

  return results;
}
