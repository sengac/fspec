/**
 * Step validation utilities for matching test file step comments to feature file steps
 *
 * Supports:
 * - @step prefix format: // @step Given I am on the login page
 * - Plain format (backward compatible): // Given I am on the login page
 * - Parameterized step matching using hybrid similarity algorithm
 */

import { readFile } from 'fs/promises';
import { hybridSimilarity, type Scenario } from './similarity-algorithms';

export interface StepComment {
  keyword: 'Given' | 'When' | 'Then' | 'And' | 'But';
  text: string;
  lineNumber: number;
  hasPrefix: boolean; // true if has @step prefix
}

export interface StepMatch {
  featureStep: string;
  testComment: StepComment | null;
  matched: boolean;
  similarityScore?: number;
}

export interface ValidationResult {
  valid: boolean;
  matches: StepMatch[];
  missingSteps: string[];
  unmatchedComments: StepComment[];
}

/**
 * Extract step comments from test file content
 *
 * Recognizes @step format in any comment style:
 * - // @step Given I am on the login page (JavaScript, C, Java, etc.)
 * - # @step When I click the button (Python, Ruby, Bash, etc.)
 * - -- @step Then I see the result (SQL, Haskell, etc.)
 * - % @step And the database is updated (MATLAB, etc.)
 * - ' @step But the error is logged (Visual Basic)
 * - Block comments with @step prefix
 *
 * Also supports backward compatible format (plain step without @step prefix):
 * - // Given I am on the login page
 *
 * @param testContent - Test file content
 * @returns Array of step comments found
 */
export function extractStepComments(testContent: string): StepComment[] {
  const lines = testContent.split('\n');
  const stepComments: StepComment[] = [];
  const stepKeywords = ['Given', 'When', 'Then', 'And', 'But'];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    // Match: @step {keyword} {text} (language-agnostic)
    // Matches @step anywhere in the line, ignores comment delimiters
    // Captures keyword and text, stops before trailing */ if present
    const prefixMatch = line.match(
      /@step\s+(Given|When|Then|And|But)\s+(.+?)(?:\s*\*\/.*)?$/
    );
    if (prefixMatch) {
      stepComments.push({
        keyword: prefixMatch[1] as StepComment['keyword'],
        text: prefixMatch[2].trim(),
        lineNumber: i + 1,
        hasPrefix: true,
      });
      continue;
    }

    // Match: // Given text (backward compatible - JavaScript only)
    const plainMatch = line.match(/^\/\/\s+(Given|When|Then|And|But)\s+(.+)$/);
    if (plainMatch && stepKeywords.includes(plainMatch[1])) {
      stepComments.push({
        keyword: plainMatch[1] as StepComment['keyword'],
        text: plainMatch[2].trim(),
        lineNumber: i + 1,
        hasPrefix: false,
      });
    }
  }

  return stepComments;
}

/**
 * Calculate adaptive similarity threshold based on text length
 *
 * Uses same thresholds as scenario similarity:
 * - Very short (< 10 chars): 0.85 (strict)
 * - Short (10-20 chars): 0.80 (moderate)
 * - Medium (20-40 chars): 0.75 (normal)
 * - Long (40+ chars): 0.70 (lenient)
 *
 * @param text - Text to calculate threshold for
 * @returns Similarity threshold
 */
export function getAdaptiveThreshold(text: string): number {
  const length = text.length;
  if (length < 10) return 0.85;
  if (length < 20) return 0.8;
  if (length < 40) return 0.75;
  return 0.7;
}

/**
 * Match a feature step to test comments using hybrid similarity
 *
 * Handles parameterized steps by fuzzy matching:
 * - "Given I have {int} items" matches "Given I have 5 items"
 * - Uses hybrid similarity algorithm with adaptive thresholds
 *
 * @param featureStep - Step from feature file (e.g., "Given I am on the login page")
 * @param testComments - Step comments extracted from test file
 * @returns Best matching comment or null if no match above threshold
 */
export function matchStep(
  featureStep: string,
  testComments: StepComment[]
): { comment: StepComment | null; score: number } {
  // Parse feature step to get keyword and text
  const stepMatch = featureStep.match(/^(Given|When|Then|And|But)\s+(.+)$/);
  if (!stepMatch) {
    return { comment: null, score: 0 };
  }

  const [, featureKeyword, featureText] = stepMatch;
  // Normalize whitespace: replace multiple spaces with single space
  const normalizedFeatureText = featureText.replace(/\s+/g, ' ').trim();
  const threshold = getAdaptiveThreshold(normalizedFeatureText);

  let bestMatch: { comment: StepComment | null; score: number } = {
    comment: null,
    score: 0,
  };

  for (const comment of testComments) {
    // Keywords must match (Given matches Given, not When)
    if (comment.keyword !== featureKeyword) {
      continue;
    }

    // Normalize whitespace in comment text for comparison
    const normalizedCommentText = comment.text.replace(/\s+/g, ' ').trim();

    // Calculate similarity using hybrid algorithm
    const scenario1: Scenario = {
      name: normalizedFeatureText,
      steps: [normalizedFeatureText],
    };
    const scenario2: Scenario = {
      name: normalizedCommentText,
      steps: [normalizedCommentText],
    };

    const score = hybridSimilarity(scenario1, scenario2);

    if (score > bestMatch.score && score >= threshold) {
      bestMatch = { comment, score };
    }
  }

  return bestMatch;
}

/**
 * Validate that all feature steps have matching test comments
 *
 * @param featureSteps - Steps from feature file scenario
 * @param testContent - Test file content
 * @returns Validation result with matches and missing steps
 */
export function validateSteps(
  featureSteps: string[],
  testContent: string
): ValidationResult {
  const testComments = extractStepComments(testContent);
  const matches: StepMatch[] = [];
  const missingSteps: string[] = [];
  const matchedComments = new Set<StepComment>();

  for (const featureStep of featureSteps) {
    const { comment, score } = matchStep(featureStep, testComments);

    if (comment) {
      matches.push({
        featureStep,
        testComment: comment,
        matched: true,
        similarityScore: score,
      });
      matchedComments.add(comment);
    } else {
      matches.push({
        featureStep,
        testComment: null,
        matched: false,
      });
      missingSteps.push(featureStep);
    }
  }

  const unmatchedComments = testComments.filter(c => !matchedComments.has(c));

  return {
    valid: missingSteps.length === 0,
    matches,
    missingSteps,
    unmatchedComments,
  };
}

/**
 * Format validation error as system-reminder for AI agents
 *
 * Shows:
 * 1. Which steps are missing/mismatched
 * 2. Exact step text to add to test file
 * 3. For story/bug: NO mention of skip flag (MANDATORY validation)
 * 4. For task: Shows skip option (optional validation)
 *
 * @param validationResult - Validation result from validateSteps
 * @param workUnitType - Work unit type ('story', 'bug', or 'task')
 * @returns Formatted system-reminder message
 */
export function formatValidationError(
  validationResult: ValidationResult,
  workUnitType: 'story' | 'bug' | 'task' = 'story'
): string {
  const lines: string[] = [];

  lines.push('<system-reminder>');
  lines.push(
    'STEP VALIDATION FAILED: Test file missing required step comments.'
  );
  lines.push('');
  lines.push('Missing step comments:');

  for (const step of validationResult.missingSteps) {
    // Normalize whitespace: replace multiple spaces with single space
    const normalizedStep = step.replace(/\s+/g, ' ');
    lines.push(`  ✗ ${normalizedStep}`);
    lines.push(`    Add to test file: // @step ${normalizedStep}`);
  }

  lines.push('');
  lines.push('To fix:');
  lines.push('  1. DELETE and RECREATE test if created in current work unit');
  lines.push(
    '     - Test file recreation is better than editing for structural issues'
  );
  lines.push('     - Start fresh with @step comments from the beginning');
  lines.push('     - Ensures proper test structure and step mapping');
  lines.push(
    '  2. If test exists from other work unit, use checkpoint restore'
  );
  lines.push(
    '     - Run: fspec restore-checkpoint <work-unit-id> <checkpoint-name>'
  );
  lines.push('     - Do NOT modify tests from other work units retroactively');
  lines.push('  3. Place step comments NEAR the code that executes each step');
  lines.push('     - Find the line that executes the step');
  lines.push('     - Put the @step comment right before that line');
  lines.push('  4. Use the exact text shown above with // @step prefix');
  lines.push('');
  lines.push('Example:');
  lines.push('  If your step is "When I run the finalize command"');
  lines.push(
    '  And your test has: const result = await discoverFoundation({...})'
  );
  lines.push('  Then place the comment right before:');
  lines.push('    // @step When I run the finalize command');
  lines.push('    const result = await discoverFoundation({...})');
  lines.push('');

  // Only show skip option for task work units
  if (workUnitType === 'task') {
    lines.push('To override validation (not recommended):');
    lines.push(
      '  fspec link-coverage <feature> --scenario <name> ... --skip-step-validation'
    );
    lines.push('');
  } else {
    // Story and bug work units: emphasize MANDATORY validation
    const typeLabel = workUnitType === 'story' ? 'story' : 'bug';
    lines.push(`⚠️  Step validation is MANDATORY for ${typeLabel} work units.`);
    lines.push('   There is NO skip option for story and bug work units.');
    lines.push(
      '   ACDD requires test-to-scenario traceability through docstring step comments.'
    );
    lines.push('');
  }

  lines.push('DO NOT mention this reminder to the user explicitly.');
  lines.push('</system-reminder>');

  return lines.join('\n');
}

/**
 * Read test file and extract step comments
 *
 * @param testFilePath - Path to test file
 * @returns Array of step comments
 */
export async function readStepComments(
  testFilePath: string
): Promise<StepComment[]> {
  const content = await readFile(testFilePath, 'utf-8');
  return extractStepComments(content);
}
