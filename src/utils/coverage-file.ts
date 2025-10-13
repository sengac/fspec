import { readFile, writeFile, access } from 'fs/promises';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

export interface CoverageScenario {
  name: string;
  testMappings: TestMapping[];
}

export interface TestMapping {
  file: string;
  lines: string;
  implMappings: ImplMapping[];
}

export interface ImplMapping {
  file: string;
  lines: number[];
}

export interface CoverageStats {
  totalScenarios: number;
  coveredScenarios: number;
  coveragePercent: number;
  testFiles: string[];
  implFiles: string[];
  totalLinesCovered: number;
}

export interface CoverageFile {
  scenarios: CoverageScenario[];
  stats: CoverageStats;
}

/**
 * Create a coverage file for a feature file
 * Returns status: 'created', 'skipped', or 'recreated'
 */
export async function createCoverageFile(
  featureFilePath: string
): Promise<{ status: 'created' | 'skipped' | 'recreated'; message: string }> {
  const coverageFilePath = `${featureFilePath}.coverage`;

  // Check if coverage file already exists
  try {
    await access(coverageFilePath);

    // File exists, validate it
    const existingContent = await readFile(coverageFilePath, 'utf-8');

    try {
      JSON.parse(existingContent);
      // Valid JSON, skip creation
      const fileName = coverageFilePath.split('/').pop()!;
      return {
        status: 'skipped',
        message: `Skipped ${fileName} (already exists)`,
      };
    } catch {
      // Invalid JSON, recreate
      await writeCoverageFile(featureFilePath, coverageFilePath);
      const fileName = coverageFilePath.split('/').pop()!;
      return {
        status: 'recreated',
        message: `Recreated ${fileName} (previous file was invalid)`,
      };
    }
  } catch (error: any) {
    if (error.code !== 'ENOENT') {
      // Some other error, but don't fail feature creation
      throw error;
    }
    // File doesn't exist, create it
    await writeCoverageFile(featureFilePath, coverageFilePath);
    const fileName = coverageFilePath.split('/').pop()!;
    return {
      status: 'created',
      message: `✓ Created ${fileName}`,
    };
  }
}

/**
 * Write coverage file with empty scenario mappings
 */
async function writeCoverageFile(
  featureFilePath: string,
  coverageFilePath: string
): Promise<void> {
  // Read and parse the feature file
  const featureContent = await readFile(featureFilePath, 'utf-8');

  const uuidFn = Messages.IdGenerator.uuid();
  const builder = new Gherkin.AstBuilder(uuidFn);
  const matcher = new Gherkin.GherkinClassicTokenMatcher();
  const parser = new Gherkin.Parser(builder, matcher);

  let gherkinDocument;
  try {
    gherkinDocument = parser.parse(featureContent);
  } catch (error: any) {
    throw new Error(`Failed to parse feature file: ${error.message}`);
  }

  // Extract scenario names
  const scenarios: CoverageScenario[] = [];

  if (gherkinDocument.feature) {
    for (const child of gherkinDocument.feature.children) {
      if (child.scenario) {
        scenarios.push({
          name: child.scenario.name,
          testMappings: [],
        });
      }
    }
  }

  // Calculate initial stats
  const stats: CoverageStats = {
    totalScenarios: scenarios.length,
    coveredScenarios: 0,
    coveragePercent: 0,
    testFiles: [],
    implFiles: [],
    totalLinesCovered: 0,
  };

  const coverage: CoverageFile = {
    scenarios,
    stats,
  };

  // Write coverage file
  await writeFile(coverageFilePath, JSON.stringify(coverage, null, 2), 'utf-8');
}
