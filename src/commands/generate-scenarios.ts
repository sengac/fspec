import { writeFile, mkdir } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join, dirname } from 'path';
import { existsSync } from 'fs';
import type { WorkUnitsData, QuestionItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { readFile } from 'fs/promises';
import {
  getUnansweredQuestionsReminder,
  getEmptyExampleMappingReminder,
  getPostGenerationReminder,
} from '../utils/system-reminder';
import { extractStepsFromExample } from '../utils/step-extraction';
import { detectPrefill } from '../utils/prefill-detection';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import {
  findMatchingScenarios,
  extractKeywords,
} from '../utils/scenario-similarity.js';

interface ScenarioMatch {
  feature: string;
  scenario: string;
  similarityScore: number;
  featurePath: string;
}

interface GenerateScenariosOptions {
  workUnitId: string;
  feature?: string;
  template?: 'given-when-then' | 'basic';
  cwd?: string;
  confirmUpdate?: boolean; // For testing - auto-confirm updates
  ignorePossibleDuplicates?: boolean; // Skip duplicate scenario detection
}

interface GenerateScenariosResult {
  success: boolean;
  featureFile: string;
  scenariosCount: number;
  systemReminders?: string[];
  detectedMatches?: ScenarioMatch[];
  updatedFeatures?: string[];
  createdFeature?: string;
  coverageRegenerated?: boolean;
}

/**
 * Categorize architecture notes by detected prefix
 *
 * Recognizes common prefixes like:
 * - Dependency: / Dependencies:
 * - Performance:
 * - Refactoring: / Refactor:
 * - Security:
 * - UI/UX:
 * - Implementation:
 *
 * Notes without recognized prefixes go into "General" category.
 */
function categorizeArchitectureNotes(
  notes: string[]
): Record<string, string[]> {
  const categories: Record<string, string[]> = {
    General: [],
  };

  const knownPrefixes = [
    'Dependency',
    'Dependencies',
    'Performance',
    'Refactoring',
    'Refactor',
    'Security',
    'UI/UX',
    'Implementation',
  ];

  for (const note of notes) {
    let categorized = false;

    for (const prefix of knownPrefixes) {
      const regex = new RegExp(`^${prefix}:\\s*`, 'i');
      if (regex.test(note)) {
        // Normalize category name (Dependencies → Dependency, Refactor → Refactoring)
        let categoryName = prefix;
        if (prefix === 'Dependencies') categoryName = 'Dependency';
        if (prefix === 'Refactor') categoryName = 'Refactoring';

        if (!categories[categoryName]) {
          categories[categoryName] = [];
        }
        categories[categoryName].push(note);
        categorized = true;
        break;
      }
    }

    if (!categorized) {
      categories.General.push(note);
    }
  }

  // Remove empty categories
  for (const [category, items] of Object.entries(categories)) {
    if (items.length === 0) {
      delete categories[category];
    }
  }

  return categories;
}

/**
 * Generate example mapping context as comments
 *
 * This creates a comment block containing:
 * - User story
 * - Business rules
 * - Examples
 * - Answered questions
 * - Assumptions
 *
 * The comment block provides context for AI agents to write scenarios.
 */
function generateExampleMappingComments(
  workUnitId: string,
  workUnit: any
): string {
  const lines: string[] = [];

  // Visual border (top)
  lines.push('  # ========================================');
  lines.push('  # EXAMPLE MAPPING CONTEXT');
  lines.push('  # ========================================');
  lines.push('  #');

  // Business rules
  if (workUnit.rules && workUnit.rules.length > 0) {
    lines.push('  # BUSINESS RULES:');
    workUnit.rules.forEach((rule: string, index: number) => {
      lines.push(`  #   ${index + 1}. ${rule}`);
    });
    lines.push('  #');
  }

  // Examples
  if (workUnit.examples && workUnit.examples.length > 0) {
    lines.push('  # EXAMPLES:');
    workUnit.examples.forEach((example: string, index: number) => {
      lines.push(`  #   ${index + 1}. ${example}`);
    });
    lines.push('  #');
  }

  // Answered questions
  if (workUnit.questions && workUnit.questions.length > 0) {
    const answeredQuestions = workUnit.questions.filter(
      (q: QuestionItem) => q.selected
    );
    if (answeredQuestions.length > 0) {
      lines.push('  # QUESTIONS (ANSWERED):');
      answeredQuestions.forEach((q: QuestionItem) => {
        // Remove @human: prefix from question text
        const questionText = q.text.replace(/^@human:\s*/i, '');
        lines.push(`  #   Q: ${questionText}`);
        lines.push(`  #   A: ${q.selected}`);
        lines.push('  #');
      });
    }
  }

  // Assumptions
  if (workUnit.assumptions && workUnit.assumptions.length > 0) {
    lines.push('  # ASSUMPTIONS:');
    workUnit.assumptions.forEach((assumption: string, index: number) => {
      lines.push(`  #   ${index + 1}. ${assumption}`);
    });
    lines.push('  #');
  }

  // Visual border (bottom)
  lines.push('  # ========================================');

  return lines.join('\n');
}

/**
 * Scan existing feature files and find matching scenarios
 */
async function scanExistingFeatures(
  cwd: string,
  examples: string[]
): Promise<{ matches: Map<number, ScenarioMatch[]>; allFeatures: any[] }> {
  const matches = new Map<number, ScenarioMatch[]>();

  // Find all existing feature files
  const featureFiles = await glob('spec/features/**/*.feature', {
    cwd,
    absolute: true,
    onlyFiles: true,
  });

  // Parse all features
  const allFeatures: any[] = [];
  for (const featureFile of featureFiles) {
    try {
      const content = await readFile(featureFile, 'utf-8');

      // Parse with @cucumber/gherkin
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);
      const doc = parser.parse(content);

      if (!doc.feature) {
        continue;
      }

      const scenarios = doc.feature.children
        .filter(child => child.scenario)
        .map(child => ({
          name: child.scenario!.name,
          steps: child.scenario!.steps.map(
            step => `${step.keyword}${step.text}`
          ),
        }));

      if (scenarios.length > 0) {
        // Extract filename from path (without directory)
        const filename = featureFile.split('/').pop() || '';

        allFeatures.push({
          path: featureFile,
          name: filename,
          scenarios,
        });
      }
    } catch (error) {
      // Skip invalid feature files
    }
  }

  // Check each example against existing scenarios
  for (let i = 0; i < examples.length; i++) {
    const example = examples[i];

    // Extract steps from example
    const extractedSteps = extractStepsFromExample(example);

    // Convert to array format for similarity comparison
    const stepsArray: string[] = [];
    if (extractedSteps.given) {
      stepsArray.push(`Given ${extractedSteps.given}`);
    }
    if (extractedSteps.when) {
      stepsArray.push(`When ${extractedSteps.when}`);
    }
    if (extractedSteps.then) {
      stepsArray.push(`Then ${extractedSteps.then}`);
    }

    // Create scenario object for matching
    const targetScenario = {
      name: example,
      steps: stepsArray,
    };

    // Find matches
    const scenarioMatches = findMatchingScenarios(
      targetScenario,
      allFeatures,
      0.7 // threshold
    );

    if (scenarioMatches.length > 0) {
      matches.set(i, scenarioMatches);
    }
  }

  return { matches, allFeatures };
}

export async function generateScenarios(
  options: GenerateScenariosOptions
): Promise<GenerateScenariosResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Check for unanswered questions (BEFORE generation)
  let unansweredCount = 0;
  if (workUnit.questions && workUnit.questions.length > 0) {
    unansweredCount = workUnit.questions.filter(q => {
      const questionItem = q as QuestionItem;
      return !questionItem.selected;
    }).length;

    if (unansweredCount > 0) {
      const reminder = getUnansweredQuestionsReminder(
        options.workUnitId,
        unansweredCount
      );
      if (reminder) {
        throw new Error(
          `Cannot generate scenarios: ${unansweredCount} unanswered question${
            unansweredCount > 1 ? 's' : ''
          } found.\n\n${reminder}\n\nAnswer questions with 'fspec answer-question ${options.workUnitId} <index>' before generating.`
        );
      }
    }
  }

  // Check for empty Example Mapping
  const hasRules = workUnit.rules && workUnit.rules.length > 0;
  const hasExamples = workUnit.examples && workUnit.examples.length > 0;
  if (!hasRules && !hasExamples) {
    const reminder = getEmptyExampleMappingReminder(
      options.workUnitId,
      hasRules,
      hasExamples
    );
    if (reminder) {
      throw new Error(
        `Cannot generate scenarios: No Example Mapping data found.\n\n${reminder}\n\nComplete Example Mapping before generating scenarios.`
      );
    }
  }

  // Validate examples exist
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error(
      `Work unit ${options.workUnitId} has no examples to generate scenarios from`
    );
  }

  // SCAN EXISTING FEATURES FOR MATCHES (Deduplication Detection)
  const { matches: detectedMatches, allFeatures } = await scanExistingFeatures(
    cwd,
    workUnit.examples
  );

  // If matches found, prepare for user interaction
  const matchArray: ScenarioMatch[] = [];
  const updatedFeatures: string[] = [];

  if (detectedMatches.size > 0) {
    // If NOT ignoring duplicates, throw error with system-reminder
    if (!options.ignorePossibleDuplicates) {
      const featureFiles = new Set<string>();
      const matchDetails: string[] = [];

      for (const [exampleIndex, matches] of detectedMatches) {
        const bestMatch = matches[0]; // Highest similarity
        matchArray.push(bestMatch);
        featureFiles.add(bestMatch.feature);

        matchDetails.push(
          `  - Example ${exampleIndex + 1}: "${workUnit.examples[exampleIndex]}"\n` +
            `    Matches: "${bestMatch.scenario}" in ${bestMatch.feature}\n` +
            `    Similarity: ${(bestMatch.similarityScore * 100).toFixed(1)}%`
        );
      }

      const systemReminder = `<system-reminder>
DUPLICATE SCENARIOS DETECTED

Found ${detectedMatches.size} potential duplicate scenario${detectedMatches.size > 1 ? 's' : ''} in existing feature files.

Detected matches:
${matchDetails.join('\n\n')}

Feature files to investigate:
${Array.from(featureFiles)
  .map(f => `  - spec/features/${f}`)
  .join('\n')}

Next steps:
  1. Investigate the feature files listed above
  2. Determine if the scenarios are truly duplicates
  3. If they are duplicates:
     - Consider refactoring to reuse existing scenarios
     - Or update the existing feature file instead of creating a new one
  4. If they are NOT duplicates (false positive):
     - Run: fspec generate-scenarios ${options.workUnitId} --ignore-possible-duplicates
     - This will bypass the duplicate check and proceed with generation

This check prevents accidental duplication across feature files.
DO NOT mention this reminder to the user explicitly.
</system-reminder>`;

      throw new Error(
        `Cannot generate scenarios: ${detectedMatches.size} duplicate scenarios detected above threshold.\n\n${systemReminder}\n\nInvestigate feature files or use --ignore-possible-duplicates to proceed.`
      );
    }

    // If ignoring duplicates, just log the warnings
    for (const [exampleIndex, matches] of detectedMatches) {
      const bestMatch = matches[0]; // Highest similarity
      matchArray.push(bestMatch);

      // Log detected match
      console.log(chalk.yellow(`\n⚠ Detected potential refactor (ignored):`));
      console.log(
        chalk.white(
          `   Example ${exampleIndex + 1}: "${workUnit.examples[exampleIndex]}"`
        )
      );
      console.log(
        chalk.cyan(
          `   Matches: "${bestMatch.scenario}" in ${bestMatch.feature}`
        )
      );
      console.log(
        chalk.gray(
          `   Similarity: ${(bestMatch.similarityScore * 100).toFixed(1)}%`
        )
      );
    }
  }

  // Determine feature file path
  let featureFile: string;
  if (options.feature) {
    // Remove .feature extension if provided
    const featureName = options.feature.replace(/\.feature$/, '');
    featureFile = join(cwd, 'spec/features', `${featureName}.feature`);
  } else {
    // Default: use work unit title (capability-based naming)
    if (!workUnit.title) {
      throw new Error(
        `Cannot determine feature file name. Work unit ${options.workUnitId} has no title.\n` +
          `Suggestion: Use --feature flag with a capability-based name (e.g., --feature=user-authentication)`
      );
    }
    // Convert title to kebab-case for feature file name
    const kebabCase = workUnit.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
    featureFile = join(cwd, 'spec/features', `${kebabCase}.feature`);
  }

  // Ensure directory exists
  await mkdir(dirname(featureFile), { recursive: true });

  // Generate example mapping comment block
  const commentBlock = generateExampleMappingComments(
    options.workUnitId,
    workUnit
  );

  // Check if feature file exists
  const fileExists = existsSync(featureFile);

  if (fileExists) {
    throw new Error(
      `Feature file ${featureFile} already exists.\n` +
        `generate-scenarios creates context-only files (comments + Background, NO scenarios).\n` +
        `If you want to add scenarios, use the Edit tool to write them based on the # EXAMPLES comments.`
    );
  }

  // Create new feature file with comment block + Background (NO scenarios)
  const title = workUnit.title || options.workUnitId;

  // Generate Background section from userStory if available
  let backgroundSection: string;
  if (workUnit.userStory) {
    backgroundSection = `  Background: User Story
    As a ${workUnit.userStory.role}
    I want to ${workUnit.userStory.action}
    So that ${workUnit.userStory.benefit}`;
  } else {
    backgroundSection = `  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]`;
  }

  // Generate architecture docstring from captured notes or use placeholders
  let architectureDocstring: string;
  if (workUnit.architectureNotes && workUnit.architectureNotes.length > 0) {
    // Group notes by detected prefix (Dependency, Performance, Refactoring, etc.)
    const categorizedNotes = categorizeArchitectureNotes(
      workUnit.architectureNotes
    );

    const docstringLines = ['  """'];
    for (const [category, notes] of Object.entries(categorizedNotes)) {
      if (category === 'General') {
        // General notes go first without category header
        notes.forEach(note => {
          docstringLines.push(`  ${note}`);
        });
      } else {
        // Categorized notes with headers
        docstringLines.push(`  ${category}:`);
        notes.forEach(note => {
          // Remove the prefix from the note since it's now in the header
          const noteWithoutPrefix = note.replace(/^[A-Za-z]+:\s*/, '');
          docstringLines.push(`  - ${noteWithoutPrefix}`);
        });
      }
    }
    docstringLines.push('  """');
    architectureDocstring = docstringLines.join('\n');
  } else {
    // Use placeholder template if no notes captured
    architectureDocstring = `  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
  """`;
  }

  const featureContent = `@${options.workUnitId}
Feature: ${title}

${architectureDocstring}

${commentBlock}

${backgroundSection}
`;
  await writeFile(featureFile, featureContent);

  // Check for prefill in generated/updated file
  const finalContent = await readFile(featureFile, 'utf-8');
  const prefillResult = detectPrefill(finalContent);

  // Generate system reminders
  const systemReminders: string[] = [];

  // Add scenario generation reminder (instructs AI to write scenarios)
  const exampleCount = workUnit.examples?.length || 0;
  const scenarioGenerationReminder = `<system-reminder>
CONTEXT-ONLY FEATURE FILE CREATED

The feature file ${featureFile} contains:
  ✓ Example mapping context as comments (# EXAMPLE MAPPING CONTEXT)
  ✓ Background section with user story
  ✗ ZERO scenarios (AI must write them)

NEXT STEP: Write scenarios based on # EXAMPLES section

The # EXAMPLES section lists ${exampleCount} example(s):
${workUnit.examples?.map((ex: string, i: number) => `  ${i + 1}. ${ex}`).join('\n') || '  (none)'}

INSTRUCTIONS FOR AI:
  1. Read the feature file to see full example mapping context
  2. For each example in # EXAMPLES, write a corresponding Scenario block
  3. Use the Edit tool to add scenarios to ${featureFile}
  4. Write proper Given/When/Then steps based on the example description
  5. Reference # BUSINESS RULES when writing Given (preconditions)
  6. Check # ASSUMPTIONS to know what NOT to test
  7. Check # QUESTIONS (ANSWERED) for clarifications

DO NOT mention this reminder to the user.
</system-reminder>`;
  systemReminders.push(scenarioGenerationReminder);

  // Add post-generation reminder
  const postGenReminder = getPostGenerationReminder(
    options.workUnitId,
    featureFile
  );
  if (postGenReminder) {
    systemReminders.push(postGenReminder);
  }

  // Add prefill reminder if detected
  if (prefillResult.hasPrefill && prefillResult.systemReminder) {
    systemReminders.push(prefillResult.systemReminder);
  }

  return {
    success: true,
    featureFile,
    scenariosCount: 0, // No scenarios generated - AI must write them
    ...(systemReminders.length > 0 && { systemReminders }),
    detectedMatches: matchArray.length > 0 ? matchArray : undefined,
    updatedFeatures: updatedFeatures.length > 0 ? updatedFeatures : undefined,
    createdFeature: featureFile
      .replace(cwd + '/', '')
      .replace('spec/features/', ''),
    coverageRegenerated: false, // TODO: Implement coverage regeneration
  };
}

export function registerGenerateScenariosCommand(program: Command): void {
  program
    .command('generate-scenarios')
    .description('Generate Gherkin scenarios from example mapping in work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .option(
      '--feature <name>',
      'Feature file name (without .feature extension). Defaults to work unit title in kebab-case.'
    )
    .option(
      '--ignore-possible-duplicates',
      'Skip duplicate scenario detection and proceed with generation'
    )
    .action(
      async (
        workUnitId: string,
        options: { feature?: string; ignorePossibleDuplicates?: boolean }
      ) => {
        try {
          const result = await generateScenarios({
            workUnitId,
            feature: options.feature,
            ignorePossibleDuplicates: options.ignorePossibleDuplicates,
          });
        console.log(
          chalk.green(
            `✓ Created context-only feature file: ${result.featureFile}`
          )
        );
        console.log(
          chalk.yellow(
            `  Contains example mapping context as comments (NO scenarios yet)`
          )
        );
        // Display system reminders if any
        if (result.systemReminders && result.systemReminders.length > 0) {
          for (const reminder of result.systemReminders) {
            console.log('\n' + reminder);
          }
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to generate scenarios:'),
          error.message
        );
        process.exit(1);
      }
    });
}
