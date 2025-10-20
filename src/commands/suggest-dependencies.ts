import chalk from 'chalk';
import type { Command } from 'commander';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import type { WorkUnitsData } from '../types';

interface DependencySuggestion {
  from: string;
  to: string;
  type: 'dependsOn' | 'relatesTo';
  reason: string;
  confidence: 'high' | 'medium';
}

interface SuggestDependenciesOptions {
  cwd?: string;
  output?: 'json' | 'text';
}

interface SuggestDependenciesResult {
  suggestions: DependencySuggestion[];
}

/**
 * Auto-suggest dependency relationships based on work unit metadata
 *
 * Rules:
 * 1. Sequential IDs in same prefix → dependsOn (AUTH-001 → AUTH-002)
 * 2. Build/Test pairs → dependsOn ('Test X' depends on 'Build X')
 * 3. Infrastructure keywords → dependsOn (schema/migration before data/features)
 * 4. Same epic → relatesTo (for context, low confidence)
 * 5. Avoid circular dependencies
 */
export async function suggestDependencies(
  options: SuggestDependenciesOptions = {}
): Promise<SuggestDependenciesResult> {
  const cwd = options.cwd || process.cwd();
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const suggestions: DependencySuggestion[] = [];
  const workUnits = Object.values(data.workUnits);

  // Group work units by prefix for sequential analysis
  const workUnitsByPrefix = new Map<string, typeof workUnits>();
  for (const wu of workUnits) {
    const prefix = wu.id.split('-')[0];
    if (!workUnitsByPrefix.has(prefix)) {
      workUnitsByPrefix.set(prefix, []);
    }
    workUnitsByPrefix.get(prefix)!.push(wu);
  }

  // Track IDs that have specific pattern matches to avoid generic sequential suggestions
  const specificMatches = new Set<string>();

  // Rule 3: Build/Test pairs (higher priority than sequential)
  for (const wu of workUnits) {
    const title = wu.title.toLowerCase();

    // Check if this is a "Test X" work unit
    if (title.startsWith('test ')) {
      const testTarget = title.replace(/^test\s+/, '');

      // Find corresponding "Build X" work unit
      for (const candidate of workUnits) {
        if (candidate.id === wu.id) continue;
        const candidateTitle = candidate.title.toLowerCase();

        if (
          candidateTitle.startsWith('build ') &&
          candidateTitle.includes(testTarget)
        ) {
          // Skip if dependency already exists
          if (
            wu.dependsOn?.includes(candidate.id) ||
            wu.blockedBy?.includes(candidate.id)
          ) {
            continue;
          }

          suggestions.push({
            from: wu.id,
            to: candidate.id,
            type: 'dependsOn',
            reason: `test work depends on build work: "${wu.title}" depends on "${candidate.title}"`,
            confidence: 'high',
          });

          // Mark this pair as having a specific match
          specificMatches.add(`${wu.id}->${candidate.id}`);
        }
      }
    }
  }

  // Rule 4: Infrastructure before features (higher priority than sequential)
  const infrastructureKeywords = [
    'schema',
    'migration',
    'database schema',
    'setup',
    'infrastructure',
  ];
  const featureKeywords = ['add', 'create', 'implement', 'build'];

  for (const featureWu of workUnits) {
    const featureTitle = featureWu.title.toLowerCase();
    const hasFeatureKeyword = featureKeywords.some(kw =>
      featureTitle.startsWith(kw + ' ')
    );

    if (!hasFeatureKeyword) continue;

    for (const infraWu of workUnits) {
      if (infraWu.id === featureWu.id) continue;
      const infraTitle = infraWu.title.toLowerCase();
      const hasInfraKeyword = infrastructureKeywords.some(kw =>
        infraTitle.includes(kw)
      );

      if (!hasInfraKeyword) continue;

      // Check if they're in the same prefix (domain)
      const featurePrefix = featureWu.id.split('-')[0];
      const infraPrefix = infraWu.id.split('-')[0];

      if (featurePrefix !== infraPrefix) continue;

      // Skip if dependency already exists
      if (
        featureWu.dependsOn?.includes(infraWu.id) ||
        featureWu.blockedBy?.includes(infraWu.id)
      ) {
        continue;
      }

      suggestions.push({
        from: featureWu.id,
        to: infraWu.id,
        type: 'dependsOn',
        reason: `infrastructure work (schema/migration) should complete before feature work: "${featureWu.title}" depends on "${infraWu.title}"`,
        confidence: 'high',
      });

      // Mark this pair as having a specific match
      specificMatches.add(`${featureWu.id}->${infraWu.id}`);
    }
  }

  // Rule 2: Sequential IDs in same prefix (fallback when no specific pattern)
  for (const [prefix, units] of workUnitsByPrefix) {
    // Sort by numeric part of ID
    const sorted = units.sort((a, b) => {
      const numA = parseInt(a.id.split('-')[1] || '0', 10);
      const numB = parseInt(b.id.split('-')[1] || '0', 10);
      return numA - numB;
    });

    // Suggest sequential dependencies
    for (let i = 1; i < sorted.length; i++) {
      const from = sorted[i];
      const to = sorted[i - 1];

      // Skip if dependency already exists
      if (from.dependsOn?.includes(to.id) || from.blockedBy?.includes(to.id)) {
        continue;
      }

      // Skip if a more specific pattern already matched this pair
      if (specificMatches.has(`${from.id}->${to.id}`)) {
        continue;
      }

      suggestions.push({
        from: from.id,
        to: to.id,
        type: 'dependsOn',
        reason: `sequential IDs in ${prefix} prefix suggest ${from.id} depends on ${to.id}`,
        confidence: 'medium',
      });
    }
  }

  // Rule 5: Remove circular dependencies
  const filteredSuggestions = suggestions.filter((suggestion, index) => {
    // Check if there's a reverse suggestion
    const hasReverse = suggestions.some(
      (s, i) =>
        i !== index && // Different suggestion
        s.from === suggestion.to &&
        s.to === suggestion.from
    );

    // If reverse exists, keep only one (deterministic: keep lower ID)
    if (hasReverse) {
      return suggestion.from < suggestion.to;
    }

    return true;
  });

  return {
    suggestions: filteredSuggestions,
  };
}

export function registerSuggestDependenciesCommand(program: Command): void {
  program
    .command('suggest-dependencies')
    .description(
      'Auto-suggest dependency relationships based on work unit metadata'
    )
    .option(
      '--output <format>',
      'Output format: json or text (default: text)',
      'text'
    )
    .action(async (options: { output?: 'json' | 'text' }) => {
      try {
        const result = await suggestDependencies({ output: options.output });

        if (options.output === 'json') {
          console.log(JSON.stringify(result, null, 2));
        } else {
          // Text output
          if (result.suggestions.length === 0) {
            console.log(chalk.yellow('No dependency suggestions found.'));
            console.log(
              chalk.dim(
                'Suggestions are based on sequential IDs, build/test pairs, and infrastructure patterns.'
              )
            );
            return;
          }

          console.log(
            chalk.bold(
              `\nFound ${result.suggestions.length} dependency suggestion(s):\n`
            )
          );

          result.suggestions.forEach((suggestion, index) => {
            const confidenceColor =
              suggestion.confidence === 'high' ? chalk.green : chalk.yellow;
            console.log(
              `${index + 1}. ${chalk.cyan(suggestion.from)} → ${chalk.cyan(suggestion.to)} (${suggestion.type})`
            );
            console.log(`   ${confidenceColor('●')} ${suggestion.reason}`);
            console.log(
              `   Confidence: ${confidenceColor(suggestion.confidence.toUpperCase())}\n`
            );
          });

          console.log(
            chalk.dim(
              'To apply a suggestion: fspec add-dependency <from-id> --depends-on=<to-id>'
            )
          );
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to suggest dependencies:'),
          error.message
        );
        process.exit(1);
      }
    });
}
