/**
 * audit-scenarios command
 *
 * Finds and reports duplicate scenarios across all feature files
 */

import { glob } from 'tinyglobby';
import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import {
  calculateScenarioSimilarity,
  extractKeywords,
} from '../utils/scenario-similarity.js';

interface DuplicateGroup {
  files: string[];
  scenarios: string[];
  similarityScore: number;
  keywords: string[];
}

interface AuditResult {
  duplicates: DuplicateGroup[];
  mergeable: boolean;
  totalScenarios: number;
  duplicateCount: number;
}

interface AuditOptions {
  cwd?: string;
  threshold?: number;
  interactive?: boolean;
}

/**
 * Audit all feature files for duplicate scenarios
 */
export async function auditScenarios(
  options: AuditOptions = {}
): Promise<AuditResult> {
  const cwd = options.cwd || process.cwd();
  const threshold = options.threshold || 0.75;

  // Find all feature files
  const featureFiles = await glob('spec/features/**/*.feature', {
    cwd,
    absolute: true,
    onlyFiles: true,
  });

  // Parse all features and extract scenarios
  const featureScenarios: Array<{
    file: string;
    name: string;
    scenarios: Array<{ name: string; steps: string[] }>;
  }> = [];

  let totalScenarios = 0;

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
        featureScenarios.push({
          file: featureFile.replace(cwd + '/', ''),
          name: doc.feature.name,
          scenarios,
        });
        totalScenarios += scenarios.length;
      }
    } catch (error) {
      // Skip invalid feature files
      console.error(
        chalk.yellow(`⚠ Skipping ${featureFile}: ${(error as Error).message}`)
      );
    }
  }

  // Find duplicates
  const duplicates: DuplicateGroup[] = [];
  const processed = new Set<string>();

  for (let i = 0; i < featureScenarios.length; i++) {
    const feature1 = featureScenarios[i];

    for (const scenario1 of feature1.scenarios) {
      const key1 = `${feature1.file}:${scenario1.name}`;

      if (processed.has(key1)) {
        continue;
      }

      const group: DuplicateGroup = {
        files: [feature1.file],
        scenarios: [scenario1.name],
        similarityScore: 1.0,
        keywords: extractKeywords(scenario1),
      };

      // Compare with scenarios in other features
      for (let j = i + 1; j < featureScenarios.length; j++) {
        const feature2 = featureScenarios[j];

        for (const scenario2 of feature2.scenarios) {
          const key2 = `${feature2.file}:${scenario2.name}`;

          if (processed.has(key2)) {
            continue;
          }

          const similarity = calculateScenarioSimilarity(scenario1, scenario2);

          if (similarity >= threshold) {
            group.files.push(feature2.file);
            group.scenarios.push(scenario2.name);
            group.similarityScore = Math.min(group.similarityScore, similarity);
            processed.add(key2);
          }
        }
      }

      // Only add groups with duplicates
      if (group.files.length > 1) {
        duplicates.push(group);
        processed.add(key1);
      }
    }
  }

  return {
    duplicates,
    mergeable: duplicates.length > 0,
    totalScenarios,
    duplicateCount: duplicates.reduce(
      (sum, group) => sum + group.files.length,
      0
    ),
  };
}

/**
 * Display audit results
 */
export function displayAuditResults(result: AuditResult): void {
  console.log(chalk.bold(`\n📊 Scenario Audit Report\n${'='.repeat(50)}\n`));

  console.log(`Total scenarios scanned: ${chalk.cyan(result.totalScenarios)}`);
  console.log(
    `Duplicate groups found: ${chalk.yellow(result.duplicates.length)}`
  );
  console.log(
    `Total duplicate scenarios: ${chalk.yellow(result.duplicateCount)}\n`
  );

  if (result.duplicates.length === 0) {
    console.log(
      chalk.green('✓ No duplicates found! All scenarios are unique.\n')
    );
    return;
  }

  console.log(chalk.bold('Duplicate Groups:\n'));

  result.duplicates.forEach((group, index) => {
    console.log(
      chalk.bold(
        `${index + 1}. Similarity: ${chalk.yellow((group.similarityScore * 100).toFixed(1) + '%')}`
      )
    );
    console.log(
      `   Keywords: ${chalk.cyan(group.keywords.slice(0, 5).join(', '))}`
    );

    group.files.forEach((file, i) => {
      console.log(
        `   ${chalk.gray('→')} ${chalk.white(file)}: "${chalk.yellow(group.scenarios[i])}"`
      );
    });

    console.log();
  });

  if (result.mergeable) {
    console.log(
      chalk.cyan(
        '💡 Tip: Review these duplicates and consider merging them to maintain consistency.\n'
      )
    );
  }
}

/**
 * Command handler
 */
export async function command(options: AuditOptions = {}): Promise<void> {
  try {
    const result = await auditScenarios(options);
    displayAuditResults(result);

    if (result.duplicates.length > 0) {
      process.exit(1);
    }
  } catch (error) {
    console.error(
      chalk.red('Error auditing scenarios:'),
      (error as Error).message
    );
    process.exit(1);
  }
}
