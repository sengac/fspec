import chalk from 'chalk';

interface ImplMapping {
  file: string;
  lines: number[];
}

interface TestMapping {
  file: string;
  lines: string;
  implMappings: ImplMapping[];
}

interface ScenarioEntry {
  testMappings: TestMapping[];
}

export function addTestMapping(
  scenarioEntry: ScenarioEntry,
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

export function addImplMapping(
  scenarioEntry: ScenarioEntry,
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
    im => im.file === implFile
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

export function addBothMappings(
  scenarioEntry: ScenarioEntry,
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
  // Support comma-separated individual lines, ranges, and mixed formats
  // Examples: "10-15" → [10..15], "10,11,12" → [10,11,12], "89-99,316-405" → [89..99, 316..405]
  const result: number[] = [];
  const segments = implLines.split(',').map(s => s.trim());
  for (const segment of segments) {
    if (segment.includes('-')) {
      const [start, end] = segment.split('-').map(n => parseInt(n.trim(), 10));
      for (let i = start; i <= end; i++) {
        result.push(i);
      }
    } else {
      const num = parseInt(segment, 10);
      if (!isNaN(num)) {
        result.push(num);
      }
    }
  }
  return result;
}

export function getRemovalHint(
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
