import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';
import type { Foundation } from '../types/foundation';
import type { Tags } from '../types/tags';
import { findOrCreateSpecDirectory } from './project-root-detection';

/**
 * Ensures spec/work-units.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty work units and all Kanban states.
 */
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'work-units.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
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

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/prefixes.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty prefixes object.
 */
export async function ensurePrefixesFile(cwd: string): Promise<PrefixesData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'prefixes.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      const initialData: PrefixesData = {
        prefixes: {},
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/epics.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty epics object.
 */
export async function ensureEpicsFile(cwd: string): Promise<EpicsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'epics.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      const initialData: EpicsData = {
        epics: {},
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/tags.json exists with proper initial structure.
 * If the file doesn't exist, creates it with standard tag categories.
 */
export async function ensureTagsFile(cwd: string): Promise<Tags> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'tags.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
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
            name: 'CAGE Integration Tags',
            description: 'CAGE-specific tags',
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

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/foundation.json exists with proper initial structure.
 * If the file doesn't exist, creates it with minimal foundation structure.
 */
export async function ensureFoundationFile(cwd: string): Promise<Foundation> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'foundation.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      const initialData: Foundation = {
        project: {
          name: 'Project',
          description: 'Project description',
          repository: 'https://github.com/user/repo',
          license: 'MIT',
          importantNote: 'Important project note',
        },
        whatWeAreBuilding: {
          projectOverview: 'Project overview',
          technicalRequirements: {
            coreTechnologies: [],
            architecture: {
              pattern: 'Architecture pattern',
              fileStructure: 'File structure',
              deploymentTarget: 'Deployment target',
              integrationModel: [],
            },
            developmentAndOperations: {
              developmentTools: 'Development tools',
              testingStrategy: 'Testing strategy',
              logging: 'Logging approach',
              validation: 'Validation approach',
              formatting: 'Formatting approach',
            },
            keyLibraries: [],
          },
          nonFunctionalRequirements: [],
        },
        whyWeAreBuildingIt: {
          problemDefinition: {
            primary: {
              title: 'Primary Problem',
              description: 'Description',
              points: [],
            },
            secondary: [],
          },
          painPoints: {
            currentState: 'Current state',
            specific: [],
          },
          stakeholderImpact: [],
          theoreticalSolutions: [],
          developmentMethodology: {
            name: 'Methodology',
            description: 'Description',
            steps: [],
            ensures: [],
          },
          successCriteria: [],
          constraintsAndAssumptions: {
            constraints: [],
            assumptions: [],
          },
        },
        architectureDiagrams: [],
        coreCommands: {
          categories: [],
        },
        featureInventory: {
          phases: [],
          tagUsageSummary: {
            phaseDistribution: [],
            componentDistribution: [],
            featureGroupDistribution: [],
            priorityDistribution: [],
            testingCoverage: [],
          },
        },
        notes: {
          developmentStatus: [],
        },
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}
