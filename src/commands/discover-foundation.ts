/**
 * discover-foundation Command
 *
 * Orchestrates code analysis + questionnaire to generate foundation.json
 */

import { discoveryGuidance } from '../guidance/automated-discovery-code-analysis';
import {
  runQuestionnaire,
  type QuestionnaireOptions,
} from './interactive-questionnaire';
import { validateGenericFoundationObject } from '../validators/generic-foundation-validator';
import type { GenericFoundation } from '../types/generic-foundation';
import { wrapInSystemReminder } from '../utils/system-reminder';
import type { Command } from 'commander';
import { writeFile, mkdir, readFile, unlink } from 'fs/promises';
import { dirname } from 'path';
import chalk from 'chalk';

export interface DiscoverFoundationOptions {
  outputPath?: string;
  finalize?: boolean;
  draftPath?: string;
}

export interface DiscoveryResult {
  personas: string[];
  capabilities: string[];
  projectType: string;
}

/**
 * Simulate code analysis (uses FOUND-002 guidance)
 * In real implementation, this would analyze actual codebase
 */
export function analyzeCodebase(): DiscoveryResult {
  // This is a simplified version that demonstrates the concept
  // Real implementation would use the guidance from FOUND-002
  return {
    personas: ['End User', 'Admin', 'API Consumer'],
    capabilities: ['User Authentication', 'Data Management', 'API Access'],
    projectType: 'web-app',
  };
}

/**
 * Emit system-reminder after code analysis
 */
export function emitDiscoveryReminder(result: DiscoveryResult): string {
  const message = `Detected ${result.personas.length} user personas from routes: ${result.personas.join(', ')}.

Review in questionnaire. Focus on WHY/WHAT, not HOW.
See CLAUDE.md for boundary guidance.

Code analysis also detected:
- Project Type: ${result.projectType}
- Key Capabilities: ${result.capabilities.join(', ')}`;

  return wrapInSystemReminder(message);
}

/**
 * Main discover-foundation command
 */
export async function discoverFoundation(
  options: DiscoverFoundationOptions = {}
): Promise<{
  systemReminder: string;
  foundation?: GenericFoundation;
  valid: boolean;
  draftPath?: string;
  draftCreated?: boolean;
  draftContent?: string;
  validated?: boolean;
  finalPath?: string;
  finalCreated?: boolean;
  draftDeleted?: boolean;
}> {
  // Phase 2: Finalize mode (validate draft and create final foundation.json)
  if (options.finalize) {
    const draftPath = options.draftPath || 'spec/foundation.json.draft';
    const finalPath = options.outputPath || 'spec/foundation.json';

    // Read and parse draft file
    const draftContent = await readFile(draftPath, 'utf-8');
    const foundation = JSON.parse(draftContent) as GenericFoundation;

    // Validate foundation
    const validation = validateGenericFoundationObject(foundation);

    if (!validation.valid) {
      return {
        systemReminder: '',
        foundation,
        valid: false,
        validated: false,
      };
    }

    // Write final foundation.json
    const dirPath = dirname(finalPath);
    await mkdir(dirPath, { recursive: true });
    await writeFile(finalPath, JSON.stringify(foundation, null, 2), 'utf-8');

    // Delete draft file
    await unlink(draftPath);

    return {
      systemReminder: '',
      foundation,
      valid: true,
      validated: true,
      finalPath,
      finalCreated: true,
      draftDeleted: true,
    };
  }

  // Phase 1: Discovery mode (create draft with placeholders)
  const discoveryResult = analyzeCodebase();
  const systemReminder = emitDiscoveryReminder(discoveryResult);

  const draftPath = options.draftPath || 'spec/foundation.json.draft';

  const draftFoundation = {
    version: '2.0.0',
    project: {
      name: '[QUESTION: What is the project name?]',
      vision: '[QUESTION: What is the core vision?]',
      projectType: `[DETECTED: ${discoveryResult.projectType}]`,
    },
    problemSpace: {
      primaryProblem: {
        title: '[QUESTION: What is the primary problem?]',
        description: '[QUESTION: Describe the problem]',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: '[QUESTION: Solution overview?]',
      capabilities: discoveryResult.capabilities.map((cap) => ({
        name: `[DETECTED: ${cap}]`,
        description: `[QUESTION: Describe ${cap}]`,
      })),
    },
    personas: discoveryResult.personas.map((persona) => ({
      name: `[DETECTED: ${persona}]`,
      description: `[QUESTION: Describe ${persona}]`,
      goals: ['[QUESTION: What are their goals?]'],
    })),
  };

  const draftContent = JSON.stringify(draftFoundation, null, 2);

  // Create directory if needed
  const dirPath = dirname(draftPath);
  await mkdir(dirPath, { recursive: true });

  // Write draft file
  await writeFile(draftPath, draftContent, 'utf-8');

  return {
    systemReminder,
    valid: true,
    draftPath,
    draftCreated: true,
    draftContent,
  };
}

/**
 * Register the discover-foundation command with Commander.js
 */
export function registerDiscoverFoundationCommand(program: Command): void {
  program
    .command('discover-foundation')
    .description('Discover project foundation automatically')
    .option(
      '--output <path>',
      'Output path for final foundation.json (default: spec/foundation.json)',
      'spec/foundation.json'
    )
    .option(
      '--finalize',
      'Finalize foundation.json from edited draft file'
    )
    .option(
      '--draft-path <path>',
      'Path to draft file (default: spec/foundation.json.draft)',
      'spec/foundation.json.draft'
    )
    .action(async (options: { output?: string; finalize?: boolean; draftPath?: string }) => {
      const result = await discoverFoundation({
        outputPath: options.output,
        finalize: options.finalize,
        draftPath: options.draftPath,
      });

      // Emit system-reminder (only visible to AI)
      if (result.systemReminder) {
        console.log(result.systemReminder);
      }

      if (options.finalize) {
        // Finalizing draft
        if (!result.valid) {
          console.error(chalk.red('✗ Foundation validation failed'));
          process.exit(1);
        }

        console.log(chalk.green(`✓ Generated ${result.finalPath}`));
        console.log(chalk.green('✓ Foundation discovered and validated successfully'));
      } else {
        // Creating draft
        console.log(chalk.green(`✓ Generated ${result.draftPath}`));
        console.log(chalk.yellow('\nNext steps:'));
        console.log(chalk.yellow('1. Edit the draft file to replace [QUESTION: ...] placeholders'));
        console.log(chalk.yellow('2. Run: fspec discover-foundation --finalize'));
      }
    });
}
