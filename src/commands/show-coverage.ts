import { readFile, readdir, access } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { CoverageFile, CoverageStats } from '../utils/coverage-file';

interface ShowCoverageOptions {
  format?: 'markdown' | 'json';
  cwd?: string;
}

interface CoverageStatus {
  status: 'fully-covered' | 'partially-covered' | 'uncovered';
  symbol: string;
}

export async function showCoverage(
  featureFile: string | undefined,
  options: ShowCoverageOptions = {}
): Promise<string> {
  const { format = 'markdown', cwd = process.cwd() } = options;

  if (featureFile) {
    // Single file mode
    return showSingleFileCoverage(featureFile, format, cwd);
  } else {
    // All files mode
    return showAllFilesCoverage(format, cwd);
  }
}

async function showSingleFileCoverage(
  featureFile: string,
  format: 'markdown' | 'json',
  cwd: string
): Promise<string> {
  // Resolve feature file path
  const featuresDir = join(cwd, 'spec', 'features');
  const fileName = featureFile.endsWith('.feature')
    ? featureFile
    : `${featureFile}.feature`;
  const coverageFile = join(featuresDir, `${fileName}.coverage`);

  // Check if coverage file exists
  try {
    await access(coverageFile);
  } catch {
    throw new Error(
      `Coverage file not found: ${fileName}.coverage\nSuggestion: Run 'fspec create-feature' to create the feature with coverage tracking`
    );
  }

  // Read and parse coverage file
  let coverage: CoverageFile;
  try {
    const content = await readFile(coverageFile, 'utf-8');
    coverage = JSON.parse(content);
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Invalid JSON in coverage file: ${fileName}.coverage\n  Parse error: ${message}\nSuggestion: Validate the JSON or recreate the file`
    );
  }

  // Validate file paths and enrich with warnings
  const enrichedCoverage = await validateAndEnrichCoverage(coverage, cwd);

  // Format output
  if (format === 'json') {
    return formatCoverageAsJSON(enrichedCoverage, fileName);
  } else {
    return formatCoverageAsMarkdown(enrichedCoverage, fileName);
  }
}

async function showAllFilesCoverage(
  format: 'markdown' | 'json',
  cwd: string
): Promise<string> {
  const featuresDir = join(cwd, 'spec', 'features');

  // Find all .coverage files
  let files: string[];
  try {
    files = await readdir(featuresDir);
  } catch {
    throw new Error(
      `Features directory not found: spec/features/\nSuggestion: Run 'fspec create-feature' to create your first feature`
    );
  }

  const coverageFiles = files.filter(f => f.endsWith('.feature.coverage'));

  if (coverageFiles.length === 0) {
    throw new Error(
      `No coverage files found in spec/features/\nSuggestion: Run 'fspec create-feature' to create features with coverage tracking`
    );
  }

  // Load all coverage files
  const allCoverage: Array<{
    fileName: string;
    coverage: CoverageFile & { warnings?: string[] };
  }> = [];

  for (const file of coverageFiles) {
    const coverageFile = join(featuresDir, file);
    try {
      const content = await readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(content);
      const enrichedCoverage = await validateAndEnrichCoverage(coverage, cwd);
      allCoverage.push({
        fileName: file.replace('.feature.coverage', '.feature'),
        coverage: enrichedCoverage,
      });
    } catch {
      // Skip invalid coverage files
      continue;
    }
  }

  // Calculate aggregated stats
  const aggregated = {
    totalFeatures: allCoverage.length,
    totalScenarios: allCoverage.reduce(
      (sum, c) => sum + c.coverage.stats.totalScenarios,
      0
    ),
    coveredScenarios: allCoverage.reduce(
      (sum, c) => sum + c.coverage.stats.coveredScenarios,
      0
    ),
    coveragePercent: 0,
  };

  aggregated.coveragePercent = aggregated.totalScenarios
    ? Math.round(
        (aggregated.coveredScenarios / aggregated.totalScenarios) * 100
      )
    : 0;

  // Format output
  if (format === 'json') {
    return JSON.stringify({ aggregated, features: allCoverage }, null, 2);
  } else {
    return formatAllCoverageAsMarkdown(aggregated, allCoverage);
  }
}

/**
 * Calculate stats from scenarios array when stats object is missing
 * Ensures backward compatibility with legacy coverage files
 */
function calculateStats(coverage: {
  scenarios: Array<{
    testMappings: Array<{
      file: string;
      implMappings?: Array<{ file: string; lines: number[] }>;
    }>;
  }>;
}): CoverageStats {
  const totalScenarios = coverage.scenarios.length;
  const coveredScenarios = coverage.scenarios.filter(
    s => s.testMappings.length > 0
  ).length;

  // Extract unique test files
  const testFilesSet = new Set<string>();
  for (const scenario of coverage.scenarios) {
    for (const testMapping of scenario.testMappings) {
      testFilesSet.add(testMapping.file);
    }
  }

  // Extract unique impl files
  const implFilesSet = new Set<string>();
  for (const scenario of coverage.scenarios) {
    for (const testMapping of scenario.testMappings) {
      for (const implMapping of testMapping.implMappings || []) {
        implFilesSet.add(implMapping.file);
      }
    }
  }

  // Calculate coverage percentage
  const coveragePercent = totalScenarios
    ? Math.round((coveredScenarios / totalScenarios) * 100)
    : 0;

  return {
    totalScenarios,
    coveredScenarios,
    coveragePercent,
    testFiles: Array.from(testFilesSet),
    implFiles: Array.from(implFilesSet),
    totalLinesCovered: 0, // Not calculated for legacy files
  };
}

async function validateAndEnrichCoverage(
  coverage: CoverageFile,
  cwd: string
): Promise<CoverageFile & { warnings?: string[] }> {
  const warnings: string[] = [];

  // If stats is missing, calculate it from scenarios
  if (!coverage.stats) {
    coverage.stats = calculateStats(coverage);
  }

  // Validate test and implementation file paths
  for (const scenario of coverage.scenarios) {
    for (const testMapping of scenario.testMappings) {
      // Check test file exists
      try {
        await access(join(cwd, testMapping.file));
      } catch {
        warnings.push(`⚠️  File not found: ${testMapping.file}`);
      }

      // Check implementation files exist
      for (const implMapping of testMapping.implMappings) {
        try {
          await access(join(cwd, implMapping.file));
        } catch {
          warnings.push(`⚠️  File not found: ${implMapping.file}`);
        }
      }
    }
  }

  return {
    ...coverage,
    warnings: warnings.length > 0 ? warnings : undefined,
  };
}

function formatCoverageAsJSON(
  coverage: CoverageFile & { warnings?: string[] },
  fileName: string
): string {
  const enriched = {
    fileName,
    scenarios: coverage.scenarios.map(scenario => ({
      ...scenario,
      coverageStatus: getCoverageStatus(scenario).status,
    })),
    stats: coverage.stats,
    warnings: coverage.warnings,
  };

  return JSON.stringify(enriched, null, 2);
}

function formatCoverageAsMarkdown(
  coverage: CoverageFile & { warnings?: string[] },
  fileName: string
): string {
  const lines: string[] = [];

  // Title
  lines.push(`# Coverage Report: ${fileName}`);
  lines.push('');

  // Coverage percentage
  const percent = coverage.stats.coveragePercent;
  lines.push(
    `**Coverage**: ${percent}% (${coverage.stats.coveredScenarios}/${coverage.stats.totalScenarios} scenarios)`
  );
  lines.push('');

  // Summary stats
  lines.push('## Summary');
  lines.push(`- Total Scenarios: ${coverage.stats.totalScenarios}`);
  lines.push(`- Covered: ${coverage.stats.coveredScenarios}`);
  lines.push(
    `- Uncovered: ${coverage.stats.totalScenarios - coverage.stats.coveredScenarios}`
  );
  lines.push(`- Test Files: ${coverage.stats.testFiles.length}`);
  lines.push(`- Implementation Files: ${coverage.stats.implFiles.length}`);

  // Calculate line counts
  const lineCounts = calculateLineCounts(coverage);
  lines.push(`- Test Lines: ${lineCounts.testLines}`);
  lines.push(`- Implementation Lines: ${lineCounts.implLines}`);
  lines.push(`- Total Lines: ${lineCounts.totalLines}`);
  lines.push('');

  // Warnings
  if (coverage.warnings && coverage.warnings.length > 0) {
    lines.push('## Warnings');
    coverage.warnings.forEach(warning => lines.push(warning));
    lines.push('');
  }

  // Scenarios breakdown
  lines.push('## Scenarios');
  lines.push('');

  for (const scenario of coverage.scenarios) {
    const { status, symbol } = getCoverageStatus(scenario);
    const statusLabel =
      status === 'fully-covered'
        ? 'FULLY COVERED'
        : status === 'partially-covered'
          ? 'PARTIALLY COVERED'
          : 'UNCOVERED';

    lines.push(`### ${symbol} ${scenario.name} (${statusLabel})`);

    if (scenario.testMappings.length > 0) {
      for (const testMapping of scenario.testMappings) {
        lines.push(`- **Test**: \`${testMapping.file}:${testMapping.lines}\``);

        if (testMapping.implMappings.length > 0) {
          for (const implMapping of testMapping.implMappings) {
            const implLines = implMapping.lines.join(',');
            lines.push(
              `- **Implementation**: \`${implMapping.file}:${implLines}\``
            );
          }
        } else {
          lines.push('- **Implementation**: ⚠️  No implementation mappings');
        }
      }
    } else {
      lines.push('- No test mappings');
    }

    lines.push('');
  }

  // Coverage Gaps section
  const uncoveredScenarios = coverage.scenarios.filter(
    s => s.testMappings.length === 0
  );

  if (uncoveredScenarios.length > 0) {
    lines.push('---');
    lines.push('');
    lines.push('## ⚠️  Coverage Gaps');
    lines.push('');
    lines.push('The following scenarios need test coverage:');
    lines.push('');
    uncoveredScenarios.forEach(scenario => {
      lines.push(`- ${scenario.name}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}

function formatAllCoverageAsMarkdown(
  aggregated: {
    totalFeatures: number;
    totalScenarios: number;
    coveredScenarios: number;
    coveragePercent: number;
  },
  allCoverage: Array<{
    fileName: string;
    coverage: CoverageFile & { warnings?: string[] };
  }>
): string {
  const lines: string[] = [];

  // Project title
  lines.push('# Project Coverage Report');
  lines.push('');

  // Overall coverage
  lines.push(
    `**Overall Coverage**: ${aggregated.coveragePercent}% (${aggregated.coveredScenarios}/${aggregated.totalScenarios} scenarios)`
  );
  lines.push('');

  // Project summary
  lines.push('## Project Summary');
  lines.push(`- Total Features: ${aggregated.totalFeatures}`);
  lines.push(`- Total Scenarios: ${aggregated.totalScenarios}`);
  lines.push(`- Covered: ${aggregated.coveredScenarios}`);
  lines.push(
    `- Uncovered: ${aggregated.totalScenarios - aggregated.coveredScenarios}`
  );
  lines.push('');

  // Features overview
  lines.push('## Features Overview');
  lines.push('');
  for (const { fileName, coverage } of allCoverage) {
    const percent = coverage.stats.coveragePercent;
    const symbol = percent === 100 ? '✅' : percent >= 50 ? '⚠️' : '❌';
    lines.push(
      `- ${fileName}: ${percent}% (${coverage.stats.coveredScenarios}/${coverage.stats.totalScenarios}) ${symbol}`
    );
  }
  lines.push('');

  // Detailed breakdown
  lines.push('---');
  lines.push('');
  lines.push('## Detailed Coverage by Feature');
  lines.push('');

  for (const { fileName, coverage } of allCoverage) {
    lines.push(`### ${fileName}`);
    lines.push(
      `**Coverage**: ${coverage.stats.coveragePercent}% (${coverage.stats.coveredScenarios}/${coverage.stats.totalScenarios} scenarios)`
    );
    lines.push('');

    // Show scenario breakdown (abbreviated)
    for (const scenario of coverage.scenarios) {
      const { symbol } = getCoverageStatus(scenario);
      lines.push(`- ${symbol} ${scenario.name}`);
    }

    lines.push('');
  }

  return lines.join('\n');
}

function getCoverageStatus(scenario: {
  testMappings: Array<{ implMappings: unknown[] }>;
}): CoverageStatus {
  if (scenario.testMappings.length === 0) {
    return { status: 'uncovered', symbol: '❌' };
  }

  const hasImplMappings = scenario.testMappings.some(
    tm => tm.implMappings.length > 0
  );

  if (hasImplMappings) {
    return { status: 'fully-covered', symbol: '✅' };
  } else {
    return { status: 'partially-covered', symbol: '⚠️' };
  }
}

function calculateLineCounts(coverage: CoverageFile): {
  testLines: number;
  implLines: number;
  totalLines: number;
} {
  let testLines = 0;
  let implLines = 0;

  for (const scenario of coverage.scenarios) {
    for (const testMapping of scenario.testMappings) {
      // Parse test line range (e.g., "45-62" = 18 lines)
      const range = testMapping.lines.split('-');
      if (range.length === 2) {
        const start = parseInt(range[0], 10);
        const end = parseInt(range[1], 10);
        testLines += end - start + 1;
      }

      // Count implementation lines (array of line numbers)
      for (const implMapping of testMapping.implMappings) {
        implLines += implMapping.lines.length;
      }
    }
  }

  return {
    testLines,
    implLines,
    totalLines: testLines + implLines,
  };
}

export async function showCoverageCommand(
  featureFile: string | undefined
): Promise<void> {
  try {
    const output = await showCoverage(featureFile);
    console.log(output);
    process.exit(0);
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(chalk.red('Error:'), message);
    process.exit(1);
  }
}

export function registerShowCoverageCommand(program: Command): void {
  program
    .command('show-coverage')
    .description('Show coverage report for feature or all features')
    .argument(
      '[feature-name]',
      'Feature name (optional - shows all if omitted)'
    )
    .action(showCoverageCommand);
}
