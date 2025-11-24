import { CoverageFile } from '../../utils/coverage-file';

export function updateStats(coverage: CoverageFile): void {
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

  // Initialize stats if missing (BUG-091 fix)
  if (!coverage.stats) {
    coverage.stats = {
      totalScenarios: coverage.scenarios.length,
      coveredScenarios: 0,
      coveragePercent: 0,
      testFiles: [],
      implFiles: [],
      totalLinesCovered: 0,
    };
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
