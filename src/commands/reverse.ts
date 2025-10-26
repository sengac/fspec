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
    // Strategy D: Outside-in BDD with persona-driven guidance (can work without session)
    if (options.strategy === 'D' && options.implementationContext) {
      return await handleStrategyD(cwd, options.implementationContext);
    }

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
    gaps.unmappedImplementation;
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

    // Append pagination guidance to existing guidance (preserve original guidance)
    if (result.guidance) {
      result.guidance = `${result.guidance}\n\nUse --strategy=${suggestedStrategy} to narrow scope.`;
    } else {
      result.guidance = `Use --strategy=${suggestedStrategy} to narrow scope.`;
    }
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

/**
 * Derive feature file name from implementation file path
 * Examples:
 *   src/components/MusicPlayer.tsx → music-player
 *   src/hooks/usePlaylistStore.ts → use-playlist-store
 *   src/utils/formatTime.js → format-time
 */
export function deriveFeatureName(implPath: string): string {
  const filename = implPath.split('/').pop() || '';
  const nameWithoutExt = filename.replace(/\.(ts|tsx|js|jsx)$/, '');

  return nameWithoutExt
    .replace(/([a-z])([A-Z])/g, '$1-$2') // camelCase → kebab-case
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2') // PascalCase → kebab-case
    .toLowerCase();
}

/**
 * Check if a feature file exists for an implementation file
 */
export function hasFeatureFile(
  implPath: string,
  featureFiles: string[]
): boolean {
  const featureName = deriveFeatureName(implPath);
  const expectedPath = `spec/features/${featureName}.feature`;

  // Check both full path match and filename match
  return featureFiles.some(
    f => f === expectedPath || f.includes(`/${featureName}.feature`)
  );
}

/**
 * Find implementation files that don't have corresponding feature files
 */
function findUnmappedImplementation(
  implementationFiles: string[],
  featureFiles: string[]
): string[] {
  return implementationFiles.filter(
    implFile => !hasFeatureFile(implFile, featureFiles)
  );
}

/**
 * Check if a file is a pure utility (should skip feature file creation)
 * Per FEAT-019 Rule 6: Skip utilities like formatDate, parseJSON
 */
function isPureUtility(implPath: string): boolean {
  const utilityPatterns = [
    /utils\/format/i,
    /utils\/parse/i,
    /utils\/validate/i,
    /helpers\//i,
    /constants\//i,
  ];

  return utilityPatterns.some(pattern => pattern.test(implPath));
}

function detectGaps(analysis: AnalysisResult): GapAnalysis {
  const { testFiles, featureFiles, implementationFiles, coverageAnalysis } =
    analysis;

  const unmappedCount = coverageAnalysis?.unmappedCount || 0;

  // Find implementation files without feature files
  const unmappedImplFiles = findUnmappedImplementation(
    implementationFiles,
    featureFiles
  ).filter(f => !isPureUtility(f)); // Exclude pure utilities

  // Determine which files to process based on gap type
  let files: string[] = [];

  if (testFiles.length > 0 && featureFiles.length === 0) {
    // Strategy A: Tests exist, no features → Process test files
    files = testFiles;
  } else if (featureFiles.length > 0 && testFiles.length === 0) {
    // Strategy B: Features exist, no tests → Process feature files
    files = featureFiles;
  } else if (unmappedCount > 0) {
    // Strategy C: Both exist, but scenarios unmapped → Process scenarios
    files = coverageAnalysis?.scenarios || [];
  } else if (unmappedImplFiles.length > 0) {
    // Strategy D: Implementation files without features → Process impl files
    files = unmappedImplFiles;
  }

  return {
    testsWithoutFeatures:
      testFiles.length > 0 && featureFiles.length === 0 ? testFiles.length : 0,
    featuresWithoutTests:
      featureFiles.length > 0 && testFiles.length === 0
        ? featureFiles.length
        : 0,
    unmappedScenarios: unmappedCount,
    unmappedImplementation: unmappedImplFiles.length,
    files,
  };
}

function suggestStrategy(gaps: GapAnalysis): 'A' | 'B' | 'C' | 'D' {
  if (gaps.testsWithoutFeatures > 0) return 'A';
  if (gaps.featuresWithoutTests > 0) return 'B';
  if (gaps.unmappedScenarios > 0) return 'C';
  if (gaps.unmappedImplementation > 0) return 'D';
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
  if (gaps.unmappedImplementation > 0) {
    return `${gaps.unmappedImplementation} implementation files without features`;
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
    gaps.unmappedImplementation;

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
 * Handle Strategy D: Outside-in BDD with persona-driven guidance
 */
async function handleStrategyD(
  cwd: string,
  implementationContext: string
): Promise<ReverseCommandResult> {
  // Load foundation.json to get personas
  const foundationPath = join(cwd, 'spec', 'foundation.json');
  let personas: Array<{ name: string; description: string; goals: string[] }> =
    [];

  try {
    const foundationContent = await fs.readFile(foundationPath, 'utf-8');
    const foundation = JSON.parse(foundationContent);
    personas = foundation.personas || [];
  } catch {
    // Foundation.json doesn't exist or is invalid
  }

  // Build persona-driven system reminder
  let systemReminder = 'REVERSE ACDD - PERSONA-DRIVEN DISCOVERY\n\n';
  systemReminder += `Implementation context: ${implementationContext}\n\n`;

  if (personas.length > 0) {
    systemReminder += 'WHO uses this? (Check foundation.json personas)\n';
    personas.forEach(persona => {
      systemReminder += `  - ${persona.name}\n`;
      if (persona.goals && persona.goals.length > 0) {
        systemReminder += `    Goals: ${persona.goals.join(', ')}\n`;
      }
    });
    systemReminder += '\n';
    systemReminder += `What does ${personas[0].name} want to accomplish?\n`;
    systemReminder += 'Think outside-in (BDD approach):\n';
    systemReminder += '  Not: "component has play/pause buttons"\n';
    systemReminder += `  Instead: "${personas[0].name} controls playback"\n\n`;
  } else {
    systemReminder += 'Foundation.json not found or has no personas.\n';
    systemReminder += 'Run: fspec discover-foundation\n\n';
  }

  systemReminder += 'What user behavior does this support?\n';
  systemReminder += 'not which system calls it, but who BENEFITS?\n';
  systemReminder += '  Not: "which system calls it"\n';
  systemReminder +=
    '  Instead: "who BENEFITS from accurate discounts" → Shopper\n\n';

  systemReminder += 'Transformation templates (implementation → behavior):\n';
  systemReminder += '  • UI Elements → User Actions\n';
  systemReminder += '    button → "User clicks/taps ACTION"\n';
  systemReminder += '    input → "User enters DATA"\n';
  systemReminder += '  • State → User Expectations\n';
  systemReminder += '    useState → "User sees STATE"\n';
  systemReminder += '    loading → "User waits for PROCESS"\n';
  systemReminder += '  • API Endpoints → User Needs\n';
  systemReminder += '    POST /orders → "User completes order"\n\n';

  systemReminder += 'Create user-centric scenarios based on persona goals.\n';

  // Build guidance
  let guidance = 'ACDD Workflow:\n';
  guidance += '1. Use example mapping: fspec add-example <work-unit> "..."\n';
  guidance += '2. Use example mapping: fspec add-rule <work-unit> "..."\n';
  guidance += '3. Generate scenarios: fspec generate-scenarios <work-unit>\n';
  guidance += '4. Create test skeletons based on scenarios\n';
  guidance +=
    '5. Link coverage: fspec link-coverage <feature> --scenario "..." --test-file <path> --test-lines <range> --skip-validation\n';

  return {
    systemReminder: wrapSystemReminder(systemReminder),
    guidance,
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
