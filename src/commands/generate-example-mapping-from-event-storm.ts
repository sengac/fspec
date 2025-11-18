/**
 * Generate Example Mapping from Event Storm command
 * Automatically derives rules, examples, and questions from Event Storm artifacts
 */

import { join } from 'path';
import { existsSync } from 'fs';
import type { Command } from 'commander';
import { logger } from '../utils/logger';
import { fileManager } from '../utils/file-manager';
import { pascalCaseToSentence } from '../utils/text-formatting';
import type {
  WorkUnitsData,
  EventStormPolicy,
  EventStormEvent,
  EventStormHotspot,
} from '../types';

export interface GenerateExampleMappingOptions {
  workUnitId: string;
  cwd?: string;
}

export interface GenerateExampleMappingResult {
  success: boolean;
  error?: string;
  rulesAdded?: number;
  examplesAdded?: number;
  questionsAdded?: number;
}

/**
 * Generate Example Mapping entries from Event Storm artifacts
 */
export async function generateExampleMappingFromEventStorm(
  options: GenerateExampleMappingOptions
): Promise<GenerateExampleMappingResult> {
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
    // Read work units data using transaction for safety
    let rulesAdded = 0;
    let examplesAdded = 0;
    let questionsAdded = 0;

    await fileManager.transaction(workUnitsFile, async data => {
      const workUnitsData = data as WorkUnitsData;

      // Validate work unit exists
      const workUnit = workUnitsData.workUnits[options.workUnitId];
      if (!workUnit) {
        throw new Error(`Work unit ${options.workUnitId} not found`);
      }

      // Validate Event Storm exists
      if (!workUnit.eventStorm || !workUnit.eventStorm.items) {
        throw new Error(
          `Work unit ${options.workUnitId} has no Event Storm data`
        );
      }

      // Initialize Example Mapping arrays if not present
      if (!workUnit.rules) workUnit.rules = [];
      if (!workUnit.examples) workUnit.examples = [];
      if (!workUnit.questions) workUnit.questions = [];

      // Get next IDs
      const nextRuleId = workUnit.rules.length;
      const nextExampleId = workUnit.examples.length;
      const nextQuestionId = workUnit.questions.length;

      // Process Event Storm items
      for (const item of workUnit.eventStorm.items) {
        // Skip deleted items
        if (item.deleted) continue;

        // Derive rules from policies
        if (item.type === 'policy') {
          const policy = item as EventStormPolicy;
          if (policy.when && policy.then) {
            const whenText = pascalCaseToSentence(policy.when);
            const thenText = pascalCaseToSentence(policy.then);
            const ruleText = `System must ${thenText} after ${whenText}`;
            workUnit.rules.push({
              id: nextRuleId + rulesAdded,
              text: ruleText,
              deleted: false,
              createdAt: new Date().toISOString(),
            });
            rulesAdded++;
          }
        }

        // Derive examples from events
        if (item.type === 'event') {
          const event = item as EventStormEvent;
          // Convert PascalCase event name to sentence
          const eventSentence = pascalCaseToSentence(event.text);
          // Create more natural example text based on common patterns
          const exampleText =
            eventSentence.includes('authenticated') ||
            eventSentence.includes('logged in')
              ? `User enters valid credentials and is ${eventSentence}`
              : `User ${eventSentence} and is logged in`;
          workUnit.examples.push({
            id: nextExampleId + examplesAdded,
            text: exampleText,
            deleted: false,
            createdAt: new Date().toISOString(),
          });
          examplesAdded++;
        }

        // Derive questions from hotspots
        if (item.type === 'hotspot') {
          const hotspot = item as EventStormHotspot;
          if (hotspot.concern) {
            // BUG-088: Preserve concern text as-is, just ensure it ends with '?'
            let concernText = hotspot.concern.trim();
            if (!concernText.endsWith('?')) {
              concernText += '?';
            }
            const questionText = `@human: ${concernText}`;
            workUnit.questions.push({
              id: nextQuestionId + questionsAdded,
              text: questionText,
              deleted: false,
              answer: undefined,
              createdAt: new Date().toISOString(),
            });
            questionsAdded++;
          }
        }
      }

      // Update work unit timestamp
      workUnit.updatedAt = new Date().toISOString();

      // Update meta
      if (workUnitsData.meta) {
        workUnitsData.meta.lastUpdated = new Date().toISOString();
      }
    });

    return {
      success: true,
      rulesAdded,
      examplesAdded,
      questionsAdded,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message || 'Unknown error occurred',
    };
  }
}

/**
 * Register command with Commander
 */
export function registerGenerateExampleMappingFromEventStormCommand(
  program: Command
): void {
  program
    .command('generate-example-mapping-from-event-storm')
    .description(
      'Generate Example Mapping entries (rules, examples, questions) from Event Storm artifacts'
    )
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await generateExampleMappingFromEventStorm({
          workUnitId,
        });

        if (!result.success) {
          logger.error(
            result.error ||
              'Failed to generate Example Mapping from Event Storm'
          );
          process.exit(1);
        }

        logger.success(
          `Generated Example Mapping for ${workUnitId}:\n` +
            `  Rules added: ${result.rulesAdded}\n` +
            `  Examples added: ${result.examplesAdded}\n` +
            `  Questions added: ${result.questionsAdded}`
        );
      } catch (error: any) {
        logger.error(`Error: ${error.message}`);
        process.exit(1);
      }
    });
}
