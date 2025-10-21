/**
 * Interactive reverse ACDD strategy planning command
 * Analyzes project gaps and guides AI through reverse ACDD workflow
 */

import { promises as fs } from 'fs';
import { join } from 'path';
import type {
  ReverseCommandOptions,
  ReverseCommandResult,
  GapAnalysis,
  AnalysisResult,
} from '../types/reverse-session.js';
import {
  sessionExists,
  loadSession,
  saveSession,
  deleteSession,
  createSession,
  setStrategy,
  incrementStep,
  validateCompletion,
} from '../utils/reverse-session.js';

export async function reverse(
  options: ReverseCommandOptions = {}
): Promise<ReverseCommandResult> {
  const cwd = options.cwd || process.cwd();

  // Handle --reset flag
  if (options.reset) {
    await deleteSession(cwd);
    return { message: 'Session reset' };
  }

  // Handle --status flag
  if (options.status) {
    const session = await loadSession(cwd);
    if (!session) {
      return { message: 'No active reverse session' };
    }

    return {
      phase: session.phase,
      strategy: session.strategy,
      strategyName:
        session.strategyName ||
        (session.strategy ? getStrategyName(session.strategy) : undefined),
      gapsDetected: formatGaps(session.gaps),
      progress:
        session.currentStep && session.totalSteps
          ? `Step ${session.currentStep} of ${session.totalSteps}`
          : undefined,
      gapList: session.gaps.files.map((file, idx) => ({
        file,
        // A file is completed if its index is less than (currentStep - 1)
        // e.g., if currentStep is 2, files at index 0 are completed
        completed: session.currentStep ? idx < session.currentStep - 1 : false,
      })),
    };
  }

  // Handle --complete flag
  if (options.complete) {
    const session = await loadSession(cwd);
    if (!session) {
      return { message: 'No active reverse session to complete', exitCode: 1 };
    }

    const isComplete = validateCompletion(session);
    if (!isComplete) {
      return {
        message: 'Cannot complete: not all steps are finished',
        exitCode: 1,
      };
    }

    await deleteSession(cwd);
    return {
      message: '✓ Reverse ACDD session complete',
      systemReminder: wrapSystemReminder(
        'Session completed successfully.\nAll gaps filled.'
      ),
      validationComplete: true,
      gapsFilled: true,
    };
  }

  // Handle --continue flag
  if (options.continue) {
    const session = await loadSession(cwd);
    if (!session) {
      return { message: 'No active reverse session', exitCode: 1 };
    }

    const updatedSession = incrementStep(session);
    await saveSession(cwd, updatedSession);

    const isFinalStep =
      updatedSession.currentStep === updatedSession.totalSteps;
    const nextFile = updatedSession.gaps.files[updatedSession.currentStep! - 1];

    return {
      systemReminder: wrapSystemReminder(
        `Step ${updatedSession.currentStep} of ${updatedSession.totalSteps}\n` +
          `Process file: ${nextFile}\n` +
          (isFinalStep
            ? 'After completing this final step, run: fspec reverse --complete'
            : 'After completing this step, run: fspec reverse --continue')
      ),
      guidance: `Process test file: ${nextFile}. Read the file, create feature file, then link coverage.`,
    };
  }

  // Handle --strategy flag
  if (options.strategy) {
    const session = await loadSession(cwd);
    if (!session) {
      return { message: 'No active reverse session', exitCode: 1 };
    }

    const strategyName = getStrategyName(options.strategy);
    const totalSteps = session.gaps.files.length;
    const updatedSession = setStrategy(
      session,
      options.strategy as any,
      strategyName,
      totalSteps
    );
    await saveSession(cwd, updatedSession);

    const firstFile = session.gaps.files[0];

    return {
      systemReminder: wrapSystemReminder(
        `Step 1 of ${totalSteps}\n` +
          `Strategy: ${options.strategy} (${strategyName})\n` +
          'After completing this step, run: fspec reverse --continue'
      ),
      guidance: `Read test file: ${firstFile}. Then create feature file. Then run fspec link-coverage with --skip-validation.`,
    };
  }

  // Check for existing session (prevent starting new session)
  if (await sessionExists(cwd)) {
    const existingSession = await loadSession(cwd);
    if (!existingSession) {
      return { message: 'Session file corrupted', exitCode: 1 };
    }

    return {
      existingSessionDetected: true,
      exitCode: 1,
      message: 'Existing reverse session detected',
      currentPhase: existingSession.phase,
      currentStrategy:
        existingSession.strategy && existingSession.strategyName
          ? `${existingSession.strategy} (${existingSession.strategyName})`
          : undefined,
      currentProgress:
        existingSession.currentStep && existingSession.totalSteps
          ? `Step ${existingSession.currentStep} of ${existingSession.totalSteps}`
          : undefined,
      suggestions: [
        'fspec reverse --continue',
        'fspec reverse --status',
        'fspec reverse --reset',
        'fspec reverse --complete',
      ],
      systemReminder: wrapSystemReminder(
        'Existing session detected. DO NOT start new session.\n' +
          'Either continue the existing session or reset it first.'
      ),
    };
  }

  // Initial analysis - detect gaps
  const analysis = await analyzeProject(cwd);
  const gaps = detectGaps(analysis);
  const suggestedStrategy = suggestStrategy(gaps);
  const strategyName = getStrategyName(suggestedStrategy);

  // Handle --dry-run mode (no session creation)
  if (options.dryRun) {
    return {
      analysis,
      gaps,
      suggestedStrategy,
      strategyName,
      message: 'Dry-run mode - no session created',
      systemReminder: wrapSystemReminder(
        `DRY-RUN MODE: Analysis complete, no session created.\n` +
          `Detected: ${formatGaps(gaps)}\n` +
          `Suggested: Strategy ${suggestedStrategy} (${strategyName})`
      ),
      guidance: generateStrategyGuidance(suggestedStrategy, gaps),
    };
  }

  // Create session in gap-detection phase
  const session = createSession(
    'gap-detection',
    gaps,
    suggestedStrategy,
    strategyName
  );
  await saveSession(cwd, session);

  const totalGaps =
    gaps.testsWithoutFeatures +
    gaps.featuresWithoutTests +
    gaps.unmappedScenarios +
    gaps.rawImplementation;
  const result: ReverseCommandResult = {
    analysis,
    gaps,
    suggestedStrategy,
    strategyName,
    systemReminder: wrapSystemReminder(
      `Gap analysis complete.\n` +
        `Detected: ${formatGaps(gaps)}\n` +
        `Suggested: Strategy ${suggestedStrategy} (${strategyName})\n` +
        `To choose this strategy, run: fspec reverse --strategy=${suggestedStrategy}`
    ),
    guidance: generateStrategyGuidance(suggestedStrategy, gaps),
    effortEstimate: getEffortEstimate(suggestedStrategy, gaps),
  };

  // Add pagination and summary for large projects (100+ gaps)
  if (totalGaps >= 100) {
    result.pagination = {
      total: totalGaps,
      perPage: 50,
      page: 1,
    };
    result.summary = `Found ${formatGaps(gaps)}`;
    result.guidance = `${result.guidance}\n\nUse --strategy=${suggestedStrategy} to narrow scope.`;
  }

  return result;
}

async function analyzeProject(cwd: string): Promise<AnalysisResult> {
  const testFiles = await findTestFiles(cwd);
  const featureFiles = await findFeatureFiles(cwd);
  const implementationFiles = await findImplementationFiles(cwd);
  const coverageAnalysis = await analyzeCoverage(cwd, featureFiles);

  return {
    testFiles,
    featureFiles,
    implementationFiles,
    coverageAnalysis,
    summary: `Found ${testFiles.length} test files, ${featureFiles.length} feature files, ${implementationFiles.length} implementation files`,
  };
}

async function findTestFiles(cwd: string): Promise<string[]> {
  const testDirs = ['src/__tests__', 'test', 'tests', '__tests__'];
  const testFiles: string[] = [];

  for (const dir of testDirs) {
    const fullPath = join(cwd, dir);
    try {
      const entries = await fs.readdir(fullPath, { withFileTypes: true });
      for (const entry of entries) {
        if (entry.isFile() && /\.test\.(ts|js|tsx|jsx)$/.test(entry.name)) {
          testFiles.push(join(dir, entry.name));
        }
      }
    } catch {
      // Directory doesn't exist, skip
    }
  }

  return testFiles;
}

async function findFeatureFiles(cwd: string): Promise<string[]> {
  const featuresDir = join(cwd, 'spec', 'features');
  try {
    const entries = await fs.readdir(featuresDir, { withFileTypes: true });
    return entries
      .filter(entry => entry.isFile() && entry.name.endsWith('.feature'))
      .map(entry => join('spec', 'features', entry.name));
  } catch {
    return [];
  }
}

async function findImplementationFiles(cwd: string): Promise<string[]> {
  const srcDir = join(cwd, 'src');
  const implFiles: string[] = [];

  try {
    await scanDirectory(srcDir, implFiles, cwd);
  } catch {
    // src directory doesn't exist
  }

  return implFiles;
}

async function scanDirectory(
  dir: string,
  files: string[],
  cwd: string
): Promise<void> {
  const entries = await fs.readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);

    if (entry.isDirectory()) {
      // Skip test directories
      if (!['__tests__', 'tests', 'test'].includes(entry.name)) {
        await scanDirectory(fullPath, files, cwd);
      }
    } else if (
      entry.isFile() &&
      /\.(ts|js|tsx|jsx)$/.test(entry.name) &&
      !entry.name.endsWith('.test.ts')
    ) {
      files.push(fullPath.replace(cwd + '/', ''));
    }
  }
}

function detectGaps(analysis: AnalysisResult): GapAnalysis {
  const { testFiles, featureFiles, implementationFiles, coverageAnalysis } =
    analysis;

  const unmappedCount = coverageAnalysis?.unmappedCount || 0;

  return {
    testsWithoutFeatures:
      testFiles.length > 0 && featureFiles.length === 0 ? testFiles.length : 0,
    featuresWithoutTests:
      featureFiles.length > 0 && testFiles.length === 0
        ? featureFiles.length
        : 0,
    unmappedScenarios: unmappedCount,
    rawImplementation:
      testFiles.length === 0 && featureFiles.length === 0
        ? implementationFiles.length
        : 0,
    files:
      testFiles.length > 0 && featureFiles.length === 0
        ? testFiles
        : featureFiles.length > 0 && testFiles.length === 0
          ? featureFiles
          : coverageAnalysis?.scenarios || [],
  };
}

function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  if (gaps.testsWithoutFeatures > 0) return 'A';
  if (gaps.featuresWithoutTests > 0) return 'B';
  if (gaps.unmappedScenarios > 0) return 'C';
  if (gaps.rawImplementation > 0) return 'D';
  return 'A'; // Default fallback
}

function getStrategyName(strategy: string): string {
  const strategies: Record<string, string> = {
    A: 'Spec Gap Filling',
    B: 'Test Gap Filling',
    C: 'Coverage Mapping',
    D: 'Full Reverse ACDD',
  };
  return strategies[strategy] || 'Unknown Strategy';
}

function formatGaps(gaps: GapAnalysis): string {
  if (gaps.testsWithoutFeatures > 0) {
    return `${gaps.testsWithoutFeatures} test files without features`;
  }
  if (gaps.featuresWithoutTests > 0) {
    return `${gaps.featuresWithoutTests} feature files without tests`;
  }
  if (gaps.unmappedScenarios > 0) {
    return `${gaps.unmappedScenarios} scenarios without coverage mappings`;
  }
  if (gaps.rawImplementation > 0) {
    return `${gaps.rawImplementation} implementation files without specs or tests`;
  }
  return 'No gaps detected';
}

function wrapSystemReminder(content: string): string {
  return `<system-reminder>\n${content}\n</system-reminder>`;
}

function generateStrategyGuidance(strategy: string, gaps: GapAnalysis): string {
  const guidanceMap: Record<string, string> = {
    A: 'Create feature files for existing tests. Reverse engineer acceptance criteria from test assertions.',
    B: 'Create test skeletons for existing feature files. Use --skip-validation when linking coverage.',
    C: 'Quick wins - no new files needed. Link existing tests to scenarios using fspec link-coverage.',
    D: 'Highest effort - analyze code, create features, tests, and work units from scratch.',
  };
  return guidanceMap[strategy] || '';
}

function getEffortEstimate(strategy: string, gaps: GapAnalysis): string {
  const totalGaps =
    gaps.testsWithoutFeatures +
    gaps.featuresWithoutTests +
    gaps.unmappedScenarios +
    gaps.rawImplementation;

  switch (strategy) {
    case 'A':
      return `${totalGaps * 2}-${totalGaps * 3} points`;
    case 'B':
      return `${totalGaps}-${totalGaps * 2} points`;
    case 'C':
      return '1 point total';
    case 'D':
      return `${totalGaps * 3}-${totalGaps * 5} points`;
    default:
      return 'Unknown';
  }
}

async function analyzeCoverage(
  cwd: string,
  featureFiles: string[]
): Promise<{ unmappedCount: number; scenarios: string[] } | undefined> {
  if (featureFiles.length === 0) {
    return undefined;
  }

  let unmappedCount = 0;
  const unmappedScenarios: string[] = [];

  for (const featureFile of featureFiles) {
    const coverageFile = join(cwd, `${featureFile}.coverage`);
    try {
      const content = await fs.readFile(coverageFile, 'utf-8');
      const coverage = JSON.parse(content);

      // Count scenarios without testMappings
      if (coverage.scenarios && Array.isArray(coverage.scenarios)) {
        for (const scenario of coverage.scenarios) {
          if (!scenario.testMappings || scenario.testMappings.length === 0) {
            unmappedCount++;
            unmappedScenarios.push(`${featureFile}:${scenario.name}`);
          }
        }
      }
    } catch {
      // Coverage file doesn't exist or is invalid - skip
    }
  }

  return {
    unmappedCount,
    scenarios: unmappedScenarios,
  };
}

/**
 * Command wrapper for CLI execution
 */
async function reverseCommand(options: ReverseCommandOptions): Promise<void> {
  try {
    const result = await reverse(options);

    // Display system-reminder if present
    if (result.systemReminder) {
      console.log(result.systemReminder);
    }

    // Display message if present
    if (result.message) {
      console.log(result.message);
    }

    // Display guidance if present
    if (result.guidance) {
      console.log(result.guidance);
    }

    // Display suggestions if present
    if (result.suggestions && result.suggestions.length > 0) {
      console.log('\nNext steps:');
      result.suggestions.forEach(suggestion => {
        console.log(`  - ${suggestion}`);
      });
    }

    // Exit with appropriate code
    const exitCode = result.exitCode || 0;
    process.exit(exitCode);
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error('Error:', errorMessage);
    process.exit(1);
  }
}

/**
 * Register reverse command with commander
 */
export function registerReverseCommand(program: any): void {
  program
    .command('reverse')
    .description(
      'Interactive reverse ACDD strategy planning command. Analyzes project state, detects gaps (missing features, tests, coverage), suggests strategic approaches, and guides AI step-by-step through gap-filling workflow.'
    )
    .option(
      '--strategy <A|B|C|D>',
      'Choose reverse ACDD strategy: A=Spec Gap Filling, B=Test Gap Filling, C=Coverage Mapping, D=Full Reverse ACDD'
    )
    .option('--continue', 'Continue to next step in current strategy execution')
    .option('--status', 'Show current session status and progress')
    .option('--reset', 'Delete session file and start fresh')
    .option('--complete', 'Mark session as complete and delete session file')
    .option('--dry-run', 'Preview gap analysis without creating session file')
    .action(async options => {
      await reverseCommand(options);
    });
}
