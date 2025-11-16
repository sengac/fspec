/**
 * Suggest tags from Event Storm artifacts
 * Coverage: EXMAP-008
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormItem } from '../types';
import { fileManager } from '../utils/file-manager';
import { logger } from '../utils/logger.js';

export interface SuggestTagsOptions {
  workUnitId: string;
  cwd?: string;
}

export interface TagSuggestion {
  category: 'component' | 'feature-group' | 'technical';
  tagName: string;
  source: string;
  confidence: 'high' | 'medium' | 'low';
}

export interface SuggestTagsResult {
  success: boolean;
  error?: string;
  suggestions?: TagSuggestion[];
}

/**
 * Convert text to kebab-case format
 */
function toKebabCase(text: string): string {
  return text
    .replace(/([a-z])([A-Z])/g, '$1-$2') // CamelCase -> Camel-Case
    .replace(/[\s_]+/g, '-') // spaces/underscores -> hyphens
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, ''); // remove non-alphanumeric except hyphens
}

/**
 * Extract common prefix from event names (e.g., "User" from UserRegistered, UserLoggedIn)
 */
function extractCommonPrefix(events: string[]): string | null {
  if (events.length === 0) return null;
  if (events.length === 1) {
    // Extract base word (remove suffixes like Created, Updated, Deleted, etc.)
    const match = events[0].match(/^([A-Z][a-z]+)/);
    return match ? match[1] : null;
  }

  // Find common prefix across multiple events
  const sortedEvents = events.slice().sort();
  const first = sortedEvents[0];
  const last = sortedEvents[sortedEvents.length - 1];
  let i = 0;

  while (
    i < first.length &&
    i < last.length &&
    first.charAt(i) === last.charAt(i)
  ) {
    i++;
  }

  const prefix = first.substring(0, i);

  // Must be at least 3 characters and end at a word boundary
  if (prefix.length >= 3) {
    // Find last capital letter boundary
    const lastCapital = prefix.match(/[A-Z][a-z]*/g);
    if (lastCapital && lastCapital.length > 0) {
      return lastCapital.join('');
    }
  }

  return null;
}

/**
 * Analyze Event Storm items and suggest tags
 */
export async function suggestTagsFromEvents(
  options: SuggestTagsOptions
): Promise<SuggestTagsResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Check if work-units.json exists
  if (!existsSync(workUnitsFile)) {
    return {
      success: false,
      error: 'spec/work-units.json not found. Run fspec init first.',
    };
  }

  try {
    // Read work units data
    const workUnitsData = await fileManager.readJSON<WorkUnitsData>(
      workUnitsFile,
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {},
      }
    );

    // Validate work unit exists
    const workUnit = workUnitsData.workUnits[options.workUnitId];
    if (!workUnit) {
      return {
        success: false,
        error: `Work unit ${options.workUnitId} not found`,
      };
    }

    // Check if eventStorm section exists and has items
    if (
      !workUnit.eventStorm ||
      !workUnit.eventStorm.items ||
      workUnit.eventStorm.items.length === 0
    ) {
      return {
        success: false,
        error: `No Event Storm artifacts found for ${options.workUnitId}`,
      };
    }

    const suggestions: TagSuggestion[] = [];
    const items = workUnit.eventStorm.items.filter(item => !item.deleted);

    // Extract bounded contexts -> component tags
    const boundedContexts = items.filter(
      item => item.type === 'bounded_context'
    );
    for (const context of boundedContexts) {
      const tagName = '@' + toKebabCase(context.text);
      suggestions.push({
        category: 'component',
        tagName,
        source: `bounded_context: ${context.text}`,
        confidence: 'high',
      });
    }

    // Extract aggregates -> component tags
    const aggregates = items.filter(item => item.type === 'aggregate');
    for (const aggregate of aggregates) {
      const tagName = '@' + toKebabCase(aggregate.text);
      suggestions.push({
        category: 'component',
        tagName,
        source: `aggregate: ${aggregate.text}`,
        confidence: 'high',
      });
    }

    // Extract domain events -> feature-group tags
    const events = items.filter(item => item.type === 'event');
    if (events.length > 0) {
      const eventNames = events.map(e => e.text);
      let featureGroup: string | null = null;

      // Check for authentication-related events first
      if (
        eventNames.some(
          name =>
            name.includes('Login') ||
            name.includes('Register') ||
            name.includes('Password') ||
            name.includes('Auth')
        )
      ) {
        featureGroup = 'authentication';
      }
      // Check for checkpoint-related events
      else if (
        eventNames.some(
          name =>
            name.includes('Checkpoint') ||
            name.includes('Restore') ||
            name.includes('Snapshot')
        )
      ) {
        featureGroup = 'checkpoint-management';
      }
      // Fallback to common prefix extraction
      else {
        const commonPrefix = extractCommonPrefix(eventNames);
        if (commonPrefix) {
          featureGroup = commonPrefix.toLowerCase();
        }
      }

      if (featureGroup) {
        const tagName = '@' + toKebabCase(featureGroup);
        suggestions.push({
          category: 'feature-group',
          tagName,
          source: `events: ${eventNames.join(', ')}`,
          confidence: 'medium',
        });
      }
    }

    // Extract external systems -> technical tags
    const externalSystems = items.filter(
      item => item.type === 'external_system'
    );
    for (const system of externalSystems) {
      // Suggest tag based on system name
      const systemTag = '@' + toKebabCase(system.text);

      // Check for OAuth
      if (system.text.toLowerCase().includes('oauth')) {
        suggestions.push({
          category: 'technical',
          tagName: '@oauth',
          source: `external_system: ${system.text}`,
          confidence: 'high',
        });
      }

      // Check for integration type
      if ('integrationType' in system && system.integrationType) {
        const integrationType = system.integrationType as string;
        if (integrationType === 'REST_API') {
          suggestions.push({
            category: 'technical',
            tagName: '@rest-api',
            source: `external_system: ${system.text}`,
            confidence: 'high',
          });
        } else if (integrationType === 'MESSAGE_QUEUE') {
          suggestions.push({
            category: 'technical',
            tagName: '@message-queue',
            source: `external_system: ${system.text}`,
            confidence: 'high',
          });
        } else if (integrationType === 'DATABASE') {
          suggestions.push({
            category: 'technical',
            tagName: '@database',
            source: `external_system: ${system.text}`,
            confidence: 'high',
          });
        }
      }
    }

    return {
      success: true,
      suggestions,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to analyze Event Storm artifacts: ${errorMessage}`,
    };
  }
}

export function registerSuggestTagsFromEventsCommand(program: Command): void {
  program
    .command('suggest-tags-from-events')
    .description('Suggest tags based on Event Storm artifacts in work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await suggestTagsFromEvents({
          workUnitId,
        });

        if (!result.success) {
          logger.error(result.error || 'Failed to suggest tags');
          process.exit(1);
        }

        if (!result.suggestions || result.suggestions.length === 0) {
          logger.info('No tag suggestions found');
          return;
        }

        // Group suggestions by category
        const byCategory: Record<string, TagSuggestion[]> = {
          component: [],
          'feature-group': [],
          technical: [],
        };

        for (const suggestion of result.suggestions) {
          byCategory[suggestion.category].push(suggestion);
        }

        // Display suggestions
        console.log(chalk.bold('\nSuggested Tags:\n'));

        for (const category of ['component', 'feature-group', 'technical']) {
          const suggestions = byCategory[category];
          if (suggestions.length === 0) continue;

          console.log(chalk.cyan(`${category.toUpperCase()}:`));
          for (const suggestion of suggestions) {
            const confidenceBadge =
              suggestion.confidence === 'high'
                ? chalk.green('[HIGH]')
                : suggestion.confidence === 'medium'
                  ? chalk.yellow('[MED]')
                  : chalk.gray('[LOW]');

            console.log(
              `  ${suggestion.tagName} ${confidenceBadge} - ${chalk.gray(suggestion.source)}`
            );
          }
          console.log();
        }

        console.log(
          chalk.gray(
            'Use: fspec register-tag <tag> <category> <description> to register these tags'
          )
        );
      } catch (error: any) {
        logger.error(`Error: ${error.message}`);
        process.exit(1);
      }
    });
}
