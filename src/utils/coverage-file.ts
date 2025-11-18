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
 * Returns status: 'created', 'skipped', 'recreated', or 'updated'
 */
export async function createCoverageFile(featureFilePath: string): Promise<{
  status: 'created' | 'skipped' | 'recreated' | 'updated';
  message: string;
}> {
  const coverageFilePath = `${featureFilePath}.coverage`;

  // Check if coverage file already exists
  try {
    await access(coverageFilePath);

    // File exists, validate it
    const existingContent = await readFile(coverageFilePath, 'utf-8');

    try {
      const existingCoverage = JSON.parse(existingContent);
      // Valid JSON, check if scenarios need updating
      const updated = await updateCoverageFile(
        featureFilePath,
        coverageFilePath,
        existingCoverage
      );

      if (updated) {
        const fileName = coverageFilePath.split('/').pop()!;
        return {
          status: 'updated',
          message: `Updated ${fileName} (added missing scenarios)`,
        };
      }

      // No updates needed, skip
      const fileName = coverageFilePath.split('/').pop()!;
      return {
        status: 'skipped',
        message: `Skipped ${fileName} (already up to date)`,
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
 * Update existing coverage file with new scenarios from feature file
 * Returns true if scenarios were added, false if no changes needed
 */
export async function updateCoverageFile(
  featureFilePath: string,
  coverageFilePath?: string,
  existingCoverage?: CoverageFile
): Promise<boolean> {
  // If called with just featureFilePath (for testing), load coverage file
  if (!coverageFilePath) {
    coverageFilePath = `${featureFilePath}.coverage`;
  }
  if (!existingCoverage) {
    const coverageContent = await readFile(coverageFilePath, 'utf-8');
    existingCoverage = JSON.parse(coverageContent);
  }
  // Read and parse the feature file to get current scenarios
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

  // Extract scenario names from feature file
  const currentScenarioNames: string[] = [];
  if (gherkinDocument.feature) {
    for (const child of gherkinDocument.feature.children) {
      if (child.scenario) {
        currentScenarioNames.push(child.scenario.name);
      }
    }
  }

  // Find existing scenario names
  const existingScenarioNames = new Set(
    existingCoverage.scenarios.map(s => s.name)
  );
  const currentScenarioNamesSet = new Set(currentScenarioNames);

  // Find new scenarios (in feature file but not in coverage file)
  const newScenarios: CoverageScenario[] = [];
  for (const name of currentScenarioNames) {
    if (!existingScenarioNames.has(name)) {
      newScenarios.push({
        name,
        testMappings: [],
      });
    }
  }

  // Find deleted scenarios (in coverage file but not in feature file)
  const keptScenarios = existingCoverage.scenarios.filter(s =>
    currentScenarioNamesSet.has(s.name)
  );

  // If no new scenarios and no deleted scenarios, return false (no update needed)
  if (
    newScenarios.length === 0 &&
    keptScenarios.length === existingCoverage.scenarios.length
  ) {
    return false;
  }

  // Rebuild scenarios array: kept scenarios + new scenarios
  const updatedScenarios = [...keptScenarios, ...newScenarios];

  // Recalculate stats
  const coveredScenarios = updatedScenarios.filter(
    s => s.testMappings && s.testMappings.length > 0
  ).length;

  const updatedCoverage: CoverageFile = {
    scenarios: updatedScenarios,
    stats: {
      ...existingCoverage.stats,
      totalScenarios: updatedScenarios.length,
      coveredScenarios,
      coveragePercent:
        updatedScenarios.length > 0
          ? Math.round((coveredScenarios / updatedScenarios.length) * 100)
          : 0,
    },
  };

  // Write updated coverage file
  await writeFile(
    coverageFilePath,
    JSON.stringify(updatedCoverage, null, 2),
    'utf-8'
  );

  return true;
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
