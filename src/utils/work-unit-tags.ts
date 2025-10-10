import * as Messages from '@cucumber/messages';
import type { WorkUnit, WorkUnitsData } from '../types/index';

export interface WorkUnitTag {
  id: string;
  level: 'feature' | 'scenario';
  scenarios: Array<{
    name: string;
    line: number;
  }>;
}

export interface WorkUnitInfo extends WorkUnitTag {
  title: string;
  status: string;
}

const WORK_UNIT_TAG_PATTERN = /^@([A-Z]{2,6}-\d+)$/;
const WORK_UNIT_LIKE_PATTERN = /^@([a-zA-Z]{2,6}-\d+)$/;

/**
 * Check if a tag is a work unit tag
 */
export function isWorkUnitTag(tag: string): boolean {
  return WORK_UNIT_TAG_PATTERN.test(tag);
}

/**
 * Check if a tag looks like a work unit tag but might have invalid format
 */
export function looksLikeWorkUnitTag(tag: string): boolean {
  return WORK_UNIT_LIKE_PATTERN.test(tag);
}

/**
 * Extract work unit ID from tag
 */
export function extractWorkUnitId(tag: string): string | null {
  const match = tag.match(WORK_UNIT_TAG_PATTERN);
  return match ? match[1] : null;
}

/**
 * Extract work unit tags from a parsed Gherkin document
 */
export function extractWorkUnitTags(
  gherkinDocument: Messages.GherkinDocument
): WorkUnitTag[] {
  const workUnitTags: Map<string, WorkUnitTag> = new Map();

  if (!gherkinDocument.feature) {
    return [];
  }

  const feature = gherkinDocument.feature;

  // Check feature-level tags
  const featureLevelWorkUnits: string[] = [];
  for (const tag of feature.tags || []) {
    const workUnitId = extractWorkUnitId(tag.name);
    if (workUnitId) {
      featureLevelWorkUnits.push(workUnitId);
    }
  }

  // Collect all scenarios from the feature
  const scenarios: Messages.Scenario[] = [];
  for (const child of feature.children || []) {
    if (child.scenario) {
      scenarios.push(child.scenario);
    }
  }

  // Process feature-level work units
  for (const workUnitId of featureLevelWorkUnits) {
    const scenarioList = scenarios
      .filter(scenario => {
        // Include scenario if it doesn't have its own work unit tag override
        const scenarioWorkUnits = (scenario.tags || [])
          .map(t => extractWorkUnitId(t.name))
          .filter(Boolean);
        return scenarioWorkUnits.length === 0;
      })
      .map(scenario => ({
        name: scenario.name,
        line: scenario.location?.line || 0,
      }));

    workUnitTags.set(workUnitId, {
      id: workUnitId,
      level: 'feature',
      scenarios: scenarioList,
    });
  }

  // Process scenario-level work unit tags
  for (const scenario of scenarios) {
    for (const tag of scenario.tags || []) {
      const workUnitId = extractWorkUnitId(tag.name);
      if (workUnitId) {
        // This is a scenario-level work unit tag
        if (!workUnitTags.has(workUnitId)) {
          workUnitTags.set(workUnitId, {
            id: workUnitId,
            level: 'scenario',
            scenarios: [],
          });
        }

        const existing = workUnitTags.get(workUnitId)!;
        existing.scenarios.push({
          name: scenario.name,
          line: scenario.location?.line || 0,
        });

        // Update level to scenario if it was feature before
        if (existing.level === 'feature') {
          existing.level = 'scenario';
        }
      }
    }
  }

  return Array.from(workUnitTags.values());
}

/**
 * Load work units data from JSON file
 */
export async function loadWorkUnitsData(
  cwd: string
): Promise<WorkUnitsData | null> {
  try {
    const { readFile } = await import('fs/promises');
    const { join } = await import('path');
    const content = await readFile(join(cwd, 'spec/work-units.json'), 'utf-8');
    return JSON.parse(content) as WorkUnitsData;
  } catch {
    return null;
  }
}

/**
 * Enrich work unit tags with work unit data
 */
export function enrichWorkUnitTags(
  workUnitTags: WorkUnitTag[],
  workUnitsData: WorkUnitsData | null
): WorkUnitInfo[] {
  if (!workUnitsData) {
    return workUnitTags.map(tag => ({
      ...tag,
      title: 'Unknown',
      status: 'unknown',
    }));
  }

  return workUnitTags.map(tag => {
    const workUnit = workUnitsData.workUnits[tag.id];
    return {
      ...tag,
      title: workUnit?.title || 'Unknown',
      status: workUnit?.status || 'unknown',
    };
  });
}
