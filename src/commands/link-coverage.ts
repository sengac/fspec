import { readFile, writeFile, access } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { CoverageFile } from '../utils/coverage-file';

interface LinkCoverageOptions {
  scenario: string;
  testFile?: string;
  testLines?: string;
  implFile?: string;
  implLines?: string;
  skipValidation?: boolean;
  cwd?: string;
}

interface LinkCoverageResult {
  success: boolean;
  message: string;
  warnings?: string;
}

export async function linkCoverage(
  featureName: string,
  options: LinkCoverageOptions
): Promise<LinkCoverageResult> {
  const {
    scenario,
    testFile,
    testLines,
    implFile,
    implLines,
    skipValidation = false,
    cwd = process.cwd(),
  } = options;

  // Validate flag combinations
  validateFlagCombinations(options);

  const warnings: string[] = [];

  // Validate files exist (unless --skip-validation)
  if (!skipValidation) {
    if (testFile) {
      await validateFileExists(join(cwd, testFile));
    }
    if (implFile) {
      await validateFileExists(join(cwd, implFile));
    }
  } else {
    // Add warnings for missing files when skipping validation
    if (testFile) {
      try {
        await access(join(cwd, testFile));
      } catch {
        warnings.push(`⚠️  File not found: ${testFile} (validation skipped)`);
      }
    }
    if (implFile) {
      try {
        await access(join(cwd, implFile));
      } catch {
        warnings.push(`⚠️  File not found: ${implFile} (validation skipped)`);
      }
    }
  }

  // Load coverage file
  const featuresDir = join(cwd, 'spec', 'features');
  const fileName = featureName.endsWith('.feature')
    ? featureName
    : `${featureName}.feature`;
  const coverageFile = join(featuresDir, `${fileName}.coverage`);
  const featureFile = join(featuresDir, fileName);

  let coverage: CoverageFile;
  try {
    const content = await readFile(coverageFile, 'utf-8');
    coverage = JSON.parse(content);
  } catch (error: any) {
    // Check if feature file exists to provide helpful system-reminder
    const scenariosInFeature = await getScenariosFromFeatureFile(featureFile);
    if (scenariosInFeature.length > 0) {
      // Feature file exists - suggest generate-coverage
      throw new Error(
        wrapSystemReminder(
          `Coverage file not found but feature file exists.\n` +
            `The scenario "${scenario}" may exist in the feature file but coverage tracking is not set up.\n` +
            `Run: fspec generate-coverage\n` +
            `This will create coverage files for all feature files, then you can link coverage.`
        ) +
          `\n\nCoverage file not found: ${fileName}.coverage\nSuggestion: Run 'fspec generate-coverage' to create coverage tracking`
      );
    }
    throw new Error(
      `Coverage file not found: ${fileName}.coverage\nSuggestion: Run 'fspec create-feature' to create the feature with coverage tracking`
    );
  }

  // Find the scenario
  const scenarioEntry = coverage.scenarios.find(s => s.name === scenario);
  if (!scenarioEntry) {
    // Check if scenario exists in feature file
    const scenariosInFeature = await getScenariosFromFeatureFile(featureFile);
    const scenarioExistsInFeature = scenariosInFeature.some(
      s => s === scenario
    );

    if (scenarioExistsInFeature) {
      // Scenario exists in feature but not in coverage - need to regenerate
      throw new Error(
        wrapSystemReminder(
          `Scenario "${scenario}" exists in feature file but not in coverage file.\n` +
            `This means the coverage file is out of sync with the feature file.\n` +
            `Run: fspec generate-coverage\n` +
            `This will update the coverage file with the new scenario, then you can run link-coverage first.`
        ) +
          `\n\nScenario not found: "${scenario}"\nAvailable scenarios:\n${coverage.scenarios.map(s => `  - ${s.name}`).join('\n')}`
      );
    }

    // Scenario doesn't exist in feature file - typo or wrong name
    throw new Error(
      `Scenario not found: "${scenario}"\nAvailable scenarios:\n${coverage.scenarios.map(s => `  - ${s.name}`).join('\n')}`
    );
  }

  let message = '';

  // Determine operation mode
  if (testFile && testLines && !implFile) {
    // Mode 1: Test-only
    message = addTestMapping(scenarioEntry, testFile, testLines);
  } else if (testFile && implFile && implLines) {
    if (testLines) {
      // Mode 3: Both at once
      message = addBothMappings(
        scenarioEntry,
        testFile,
        testLines,
        implFile,
        implLines
      );
    } else {
      // Mode 2: Impl-only (add to existing test)
      message = addImplMapping(scenarioEntry, testFile, implFile, implLines);
    }
  } else {
    throw new Error(
      'Invalid flag combination\nSuggestion: Use one of:\n' +
        '  - Test only: --test-file <file> --test-lines <range>\n' +
        '  - Impl only: --test-file <file> --impl-file <file> --impl-lines <lines>\n' +
        '  - Both: --test-file <file> --test-lines <range> --impl-file <file> --impl-lines <lines>'
    );
  }

  // Recalculate stats
  updateStats(coverage);

  // Write updated coverage file
  await writeFile(coverageFile, JSON.stringify(coverage, null, 2), 'utf-8');

  return {
    success: true,
    message: message + getRemovalHint(featureName, scenario, testFile),
    warnings: warnings.length > 0 ? warnings.join('\n') : undefined,
  };
}

function validateFlagCombinations(options: LinkCoverageOptions): void {
  const { testFile, testLines, implFile, implLines } = options;

  // Impl-only requires test-file
  if (implFile && !testFile) {
    throw new Error(
      '--test-file is required when adding implementation mappings\n' +
        'Implementation mappings attach to specific test mappings'
    );
  }

  // Test-only requires both test-file and test-lines
  if (testFile && !implFile && !testLines) {
    throw new Error(
      '--test-lines is required when linking test file\n' +
        'Example: --test-file src/__tests__/auth.test.ts --test-lines 45-62'
    );
  }

  // Impl mapping requires impl-lines
  if (implFile && !implLines) {
    throw new Error(
      '--impl-lines is required when linking implementation file\n' +
        'Example: --impl-file src/auth/login.ts --impl-lines 10,11,12'
    );
  }
}

async function validateFileExists(filePath: string): Promise<void> {
  try {
    await access(filePath);
  } catch {
    throw new Error(
      `File not found: ${filePath}\n` +
        'Suggestion: Ensure the file exists or use --skip-validation for forward planning'
    );
  }
}

function addTestMapping(
  scenarioEntry: { testMappings: any[] },
  testFile: string,
  testLines: string
): string {
  // Append test mapping (allow multiple for same file)
  scenarioEntry.testMappings.push({
    file: testFile,
    lines: testLines,
    implMappings: [],
  });

  const count = scenarioEntry.testMappings.filter(
    tm => tm.file === testFile
  ).length;

  if (count > 1) {
    return `✓ Added second test mapping for ${testFile}:${testLines}`;
  } else {
    return `✓ Linked test mapping: ${testFile}:${testLines}`;
  }
}

function addImplMapping(
  scenarioEntry: { testMappings: any[] },
  testFile: string,
  implFile: string,
  implLines: string
): string {
  // Find the test mapping
  const testMapping = scenarioEntry.testMappings.find(
    tm => tm.file === testFile
  );

  if (!testMapping) {
    throw new Error(
      `Test mapping not found: ${testFile}\n` +
        'Suggestion: Link the test file first using --test-file and --test-lines'
    );
  }

  // Parse impl lines
  const parsedLines = parseImplLines(implLines);

  // Check if impl file already exists (smart append)
  const existingImplIndex = testMapping.implMappings.findIndex(
    (im: any) => im.file === implFile
  );

  if (existingImplIndex >= 0) {
    // Update existing
    testMapping.implMappings[existingImplIndex].lines = parsedLines;
    return `✓ Updated implementation mapping: ${implFile}:${implLines}`;
  } else {
    // Add new
    testMapping.implMappings.push({
      file: implFile,
      lines: parsedLines,
    });
    return `✓ Added implementation mapping: ${implFile}:${implLines}`;
  }
}

function addBothMappings(
  scenarioEntry: { testMappings: any[] },
  testFile: string,
  testLines: string,
  implFile: string,
  implLines: string
): string {
  // Parse impl lines
  const parsedLines = parseImplLines(implLines);

  // Add test mapping with impl mapping
  scenarioEntry.testMappings.push({
    file: testFile,
    lines: testLines,
    implMappings: [
      {
        file: implFile,
        lines: parsedLines,
      },
    ],
  });

  return `✓ Linked test mapping with implementation: ${testFile}:${testLines} → ${implFile}:${implLines}`;
}

function parseImplLines(implLines: string): number[] {
  // Support both comma-separated and ranges
  if (implLines.includes('-')) {
    // Range format: "10-15" → [10, 11, 12, 13, 14, 15]
    const [start, end] = implLines.split('-').map(n => parseInt(n.trim(), 10));
    const result: number[] = [];
    for (let i = start; i <= end; i++) {
      result.push(i);
    }
    return result;
  } else {
    // Comma-separated: "10,11,12" → [10, 11, 12]
    return implLines.split(',').map(n => parseInt(n.trim(), 10));
  }
}

function updateStats(coverage: CoverageFile): void {
  const testFiles = new Set<string>();
  const implFiles = new Set<string>();
  let totalTestLines = 0;
  let totalImplLines = 0;
  let coveredScenarios = 0;

  for (const scenario of coverage.scenarios) {
    if (scenario.testMappings.length > 0) {
      coveredScenarios++;
    }

    for (const testMapping of scenario.testMappings) {
      testFiles.add(testMapping.file);

      // Count test lines
      const range = testMapping.lines.split('-');
      if (range.length === 2) {
        const start = parseInt(range[0], 10);
        const end = parseInt(range[1], 10);
        totalTestLines += end - start + 1;
      }

      for (const implMapping of testMapping.implMappings) {
        implFiles.add(implMapping.file);
        totalImplLines += implMapping.lines.length;
      }
    }
  }

  coverage.stats.coveredScenarios = coveredScenarios;
  coverage.stats.coveragePercent =
    coverage.stats.totalScenarios > 0
      ? Math.round((coveredScenarios / coverage.stats.totalScenarios) * 100)
      : 0;
  coverage.stats.testFiles = Array.from(testFiles);
  coverage.stats.implFiles = Array.from(implFiles);
  coverage.stats.totalLinesCovered = totalTestLines + totalImplLines;
}

function getRemovalHint(
  featureName: string,
  scenario: string,
  testFile?: string
): string {
  return (
    '\n\n' +
    chalk.gray('To remove this mapping:') +
    '\n' +
    chalk.gray(
      `  fspec unlink-coverage ${featureName} --scenario "${scenario}"${testFile ? ` --test-file ${testFile}` : ''}`
    )
  );
}

function wrapSystemReminder(content: string): string {
  return `<system-reminder>\n${content}\n</system-reminder>`;
}

async function getScenariosFromFeatureFile(
  featureFilePath: string
): Promise<string[]> {
  try {
    const content = await readFile(featureFilePath, 'utf-8');
    const scenarios: string[] = [];

    // Simple regex to extract scenario names
    // Matches: "Scenario: Name" or "Scenario Outline: Name"
    const scenarioRegex = /^\s*Scenario(?:\s+Outline)?:\s*(.+)$/gm;
    let match;

    while ((match = scenarioRegex.exec(content)) !== null) {
      scenarios.push(match[1].trim());
    }

    return scenarios;
  } catch {
    // Feature file doesn't exist
    return [];
  }
}

export async function linkCoverageCommand(
  featureName: string,
  options: Omit<LinkCoverageOptions, 'cwd'>
): Promise<void> {
  try {
    const result = await linkCoverage(featureName, options);

    console.log(result.message);

    if (result.warnings) {
      console.log('\n' + chalk.yellow(result.warnings));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerLinkCoverageCommand(program: Command): void {
  program
    .command('link-coverage')
    .description('Link test and implementation files to feature scenarios')
    .argument(
      '<feature-name>',
      'Feature name (e.g., "user-login" for user-login.feature)'
    )
    .requiredOption('--scenario <name>', 'Scenario name to link')
    .option(
      '--test-file <file>',
      'Test file path (e.g., src/__tests__/auth.test.ts)'
    )
    .option('--test-lines <range>', 'Test line range (e.g., "45-62")')
    .option('--impl-file <file>', 'Implementation file path')
    .option(
      '--impl-lines <lines>',
      'Implementation lines (e.g., "10,11,12" or "10-15")'
    )
    .option('--skip-validation', 'Skip file validation (for forward planning)')
    .action(linkCoverageCommand);
}
