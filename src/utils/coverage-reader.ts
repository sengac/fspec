/**
 * Shared utility for reading and parsing coverage files
 * Part of QRY-002: Enhanced search and comparison commands
 */

import { readFile } from 'fs/promises';
import { glob } from 'tinyglobby';
import { join } from 'path';

interface CoverageFile {
  featureName: string;
  filePath: string;
  scenarios: Array<{
    name: string;
    testMappings: Array<{
      file: string;
      lines: string;
      implMappings?: Array<{
        file: string;
        lines: number[];
      }>;
    }>;
  }>;
}

/**
 * Read all coverage files from spec/features directory
 */
export async function readAllCoverageFiles(
  cwd?: string
): Promise<CoverageFile[]> {
  const basePath = cwd || process.cwd();
  const featuresDir = join(basePath, 'spec', 'features');

  // Find all .coverage files
  const coverageFiles = await glob(['*.feature.coverage'], {
    cwd: featuresDir,
    absolute: false,
  });

  const parsedCoverage: CoverageFile[] = [];

  for (const file of coverageFiles) {
    const filePath = join(featuresDir, file);
    const content = await readFile(filePath, 'utf-8');

    try {
      const data = JSON.parse(content);
      const featureName = file.replace('.feature.coverage', '');

      parsedCoverage.push({
        featureName,
        filePath: join('spec', 'features', file),
        scenarios: data.scenarios || [],
      });
    } catch (error) {
      // Skip files with parse errors
      continue;
    }
  }

  return parsedCoverage;
}

/**
 * Extract all implementation files from coverage data
 */
export function extractImplementationFiles(
  coverageFiles: CoverageFile[]
): Array<{
  filePath: string;
  scenarioName: string;
  featureName: string;
  lines: number[];
}> {
  const implFiles: Array<{
    filePath: string;
    scenarioName: string;
    featureName: string;
    lines: number[];
  }> = [];

  for (const coverage of coverageFiles) {
    for (const scenario of coverage.scenarios) {
      for (const testMapping of scenario.testMappings) {
        if (testMapping.implMappings) {
          for (const implMapping of testMapping.implMappings) {
            implFiles.push({
              filePath: implMapping.file,
              scenarioName: scenario.name,
              featureName: coverage.featureName,
              lines: implMapping.lines,
            });
          }
        }
      }
    }
  }

  return implFiles;
}

/**
 * Extract all test files from coverage data
 */
export function extractTestFiles(coverageFiles: CoverageFile[]): Array<{
  filePath: string;
  scenarioName: string;
  featureName: string;
  lines: string;
}> {
  const testFiles: Array<{
    filePath: string;
    scenarioName: string;
    featureName: string;
    lines: string;
  }> = [];

  for (const coverage of coverageFiles) {
    for (const scenario of coverage.scenarios) {
      for (const testMapping of scenario.testMappings) {
        testFiles.push({
          filePath: testMapping.file,
          scenarioName: scenario.name,
          featureName: coverage.featureName,
          lines: testMapping.lines,
        });
      }
    }
  }

  return testFiles;
}
