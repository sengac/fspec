/**
 * Feature: spec/features/implement-foundation-event-storm-commands-for-aggregates-events-and-commands.feature
 * Scenario: Add command to existing bounded context
 */

import chalk from 'chalk';
import { Command } from 'commander';
import { fileManager } from '../utils/file-manager';
import { generateFoundationMdCommand } from './generate-foundation-md';
import type { GenericFoundation } from '../types/generic-foundation';
import type { EventStormItem } from '../types';

export interface AddCommandToFoundationOptions {
  cwd?: string;
  description?: string;
}

/**
 * Add command to foundation Event Storm
 * Links command to a specific bounded context
 */
export async function addCommandToFoundation(
  contextName: string,
  commandName: string,
  options: AddCommandToFoundationOptions = {}
): Promise<{ success: boolean; message?: string }> {
  const cwd = options.cwd || process.cwd();
  const foundationPath = `${cwd}/spec/foundation.json`;

  // Read foundation.json with defaults (validates file exists)
  await fileManager.readJSON<GenericFoundation>(foundationPath, {
    version: '2.0.0',
    project: {
      name: '',
      vision: '',
      projectType: 'other' as const,
    },
    problemSpace: {
      primaryProblem: {
        title: '',
        description: '',
        impact: 'medium' as const,
      },
    },
    solutionSpace: {
      overview: '',
      capabilities: [],
    },
  });

  // Use transaction for atomic update
  await fileManager.transaction<GenericFoundation>(
    foundationPath,
    async data => {
      // Initialize eventStorm section if missing
      if (!data.eventStorm) {
        data.eventStorm = {
          level: 'big_picture',
          items: [],
          nextItemId: 1,
        };
      }

      // Find bounded context by name
      const boundedContext = data.eventStorm.items.find(
        item => item.type === 'bounded_context' && item.text === contextName
      );

      if (!boundedContext) {
        throw new Error(`Bounded context '${contextName}' not found`);
      }

      // Create command item
      const command: EventStormItem & {
        boundedContextId: number;
        description?: string;
      } = {
        id: data.eventStorm.nextItemId,
        type: 'command' as const,
        text: commandName,
        boundedContextId: boundedContext.id,
        color: 'blue',
        deleted: false,
        createdAt: new Date().toISOString(),
        ...(options.description && { description: options.description }),
      };

      // Add item and increment counter
      data.eventStorm.items.push(command);
      data.eventStorm.nextItemId++;
    }
  );

  // Auto-regenerate FOUNDATION.md after updating foundation.json
  await generateFoundationMdCommand({ cwd });

  return {
    success: true,
    message: `Added command "${commandName}" to "${contextName}" bounded context`,
  };
}

/**
 * CLI command handler
 */
export async function addCommandToFoundationCommand(
  contextName: string,
  commandName: string,
  options: AddCommandToFoundationOptions
): Promise<void> {
  try {
    const result = await addCommandToFoundation(
      contextName,
      commandName,
      options
    );

    if (!result.success) {
      console.error(chalk.red('Error:'), result.message);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    process.exit(0);
  } catch (error: unknown) {
    console.error(
      chalk.red('Error:'),
      error instanceof Error ? error.message : 'Unknown error'
    );
    process.exit(1);
  }
}

/**
 * Register command with Commander
 */
export function registerAddCommandToFoundationCommand(program: Command): void {
  program
    .command('add-command-to-foundation')
    .description(
      'Add a command to a foundation bounded context in Big Picture Event Storm'
    )
    .argument('<context-name>', 'Bounded context name')
    .argument('<command-name>', 'Command name')
    .option('-d, --description <text>', 'Optional description')
    .action(
      async (
        contextName: string,
        commandName: string,
        options: AddCommandToFoundationOptions
      ) => {
        await addCommandToFoundationCommand(contextName, commandName, options);
      }
    );
}
