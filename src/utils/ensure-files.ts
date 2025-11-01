import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';
import type { Foundation } from '../types/foundation';
import type { Tags } from '../types/tags';
import { findOrCreateSpecDirectory } from './project-root-detection';
import { fileManager } from './file-manager';
import { ensureLatestVersion } from '../migrations';

/**
 * Current version of fspec (used for auto-migration)
 */
const CURRENT_VERSION = '0.7.0';

/**
 * Ensures spec/work-units.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty work units and all Kanban states.
 * Automatically runs migrations if needed to bring file to current version.
 *
 * Uses fileManager.readJSON() with read-lock-first pattern (LOCK-002).
 */
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'work-units.json');

  const initialData: WorkUnitsData = {
    meta: {
      version: '1.0.0',
      lastUpdated: new Date().toISOString(),
    },
    workUnits: {},
    states: {
      backlog: [],
      specifying: [],
      testing: [],
      implementing: [],
      validating: [],
      done: [],
      blocked: [],
    },
  };

  // Use fileManager.readJSON() which handles ENOENT and creates file with default data
  try {
    let data = await fileManager.readJSON(filePath, initialData);

    // Run auto-migration to ensure file is at current version
    data = await ensureLatestVersion(cwd, data, CURRENT_VERSION);

    return data;
  } catch (error: unknown) {
    // Provide helpful error message for JSON parse errors
    if (error instanceof SyntaxError) {
      throw new Error(
        `Failed to parse work-units.json: ${error.message}. The file may be corrupted or contain invalid JSON.`
      );
    }
    throw error;
  }
}

/**
 * Ensures spec/prefixes.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty prefixes object.
 *
 * Uses fileManager.readJSON() with read-lock-first pattern (LOCK-002).
 */
export async function ensurePrefixesFile(cwd: string): Promise<PrefixesData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'prefixes.json');

  const initialData: PrefixesData = {
    prefixes: {},
  };

  return await fileManager.readJSON(filePath, initialData);
}

/**
 * Ensures spec/epics.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty epics object.
 *
 * Uses fileManager.readJSON() with read-lock-first pattern (LOCK-002).
 */
export async function ensureEpicsFile(cwd: string): Promise<EpicsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'epics.json');

  const initialData: EpicsData = {
    epics: {},
  };

  return await fileManager.readJSON(filePath, initialData);
}

/**
 * Ensures spec/tags.json exists with proper initial structure.
 * If the file doesn't exist, creates it with standard tag categories.
 *
 * Uses fileManager.readJSON() with read-lock-first pattern (LOCK-002).
 */
export async function ensureTagsFile(cwd: string): Promise<Tags> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'tags.json');

  const initialData: Tags = {
    categories: [
      {
        name: 'Phase Tags',
        description: 'Phase identification tags',
        required: true,
        tags: [],
      },
      {
        name: 'Component Tags',
        description: 'Architectural component tags',
        required: true,
        tags: [],
      },
      {
        name: 'Feature Group Tags',
        description: 'Functional area tags',
        required: true,
        tags: [],
      },
      {
        name: 'Technical Tags',
        description: 'Technical concern tags',
        required: false,
        tags: [],
      },
      {
        name: 'Platform Tags',
        description: 'Platform-specific tags',
        required: false,
        tags: [],
      },
      {
        name: 'Priority Tags',
        description: 'Implementation priority tags',
        required: false,
        tags: [],
      },
      {
        name: 'Status Tags',
        description: 'Development status tags',
        required: false,
        tags: [],
      },
      {
        name: 'Testing Tags',
        description: 'Test-related tags',
        required: false,
        tags: [],
      },
      {
        name: 'Automation Tags',
        description: 'Automation integration tags',
        required: false,
        tags: [],
      },
    ],
    combinationExamples: [],
    usageGuidelines: {
      requiredCombinations: {
        title: '',
        requirements: [],
        minimumExample: '',
      },
      recommendedCombinations: {
        title: '',
        includes: [],
        recommendedExample: '',
      },
      orderingConvention: { title: '', order: [], example: '' },
    },
    addingNewTags: {
      process: [],
      namingConventions: [],
      antiPatterns: { dont: [], do: [] },
    },
    queries: { title: '', examples: [] },
    statistics: {
      lastUpdated: new Date().toISOString(),
      phaseStats: [],
      componentStats: [],
      featureGroupStats: [],
      updateCommand: 'fspec tag-stats',
    },
    validation: { rules: [], commands: [] },
    references: [],
  };

  return await fileManager.readJSON(filePath, initialData);
}

/**
 * Ensures spec/foundation.json exists with proper initial structure.
 * If the file doesn't exist, creates it with minimal foundation structure.
 *
 * Uses fileManager.readJSON() with read-lock-first pattern (LOCK-002).
 */
export async function ensureFoundationFile(cwd: string): Promise<Foundation> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'foundation.json');

  const initialData: any = {
    version: '2.0.0',
    project: {
      name: 'Project Name',
      vision: 'Project vision statement',
      projectType: 'cli-tool',
    },
    problemSpace: {
      primaryProblem: {
        title: 'Primary Problem',
        description: 'Problem description',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Solution overview',
      capabilities: [
        {
          name: 'Core Capability',
          description: 'Capability description',
        },
      ],
    },
    personas: [
      {
        name: 'Primary User',
        description: 'User description',
        goals: ['User goal'],
      },
    ],
    architectureDiagrams: [],
  };

  return await fileManager.readJSON(filePath, initialData);
}
