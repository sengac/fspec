import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { WorkUnitsData, UserStory } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface SetUserStoryOptions {
  role: string;
  action: string;
  benefit: string;
  cwd?: string;
}

export async function setUserStory(
  workUnitId: string,
  options: SetUserStoryOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  if (!data.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const userStory: UserStory = {
    role: options.role,
    action: options.action,
    benefit: options.benefit,
  };

  data.workUnits[workUnitId].userStory = userStory;
  data.workUnits[workUnitId].updatedAt = new Date().toISOString();

  if (data.meta) {
    data.meta.lastUpdated = new Date().toISOString();
  }

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsPath, async fileData => {
    Object.assign(fileData, data);
  });
}

export async function setUserStoryCommand(
  workUnitId: string,
  options: SetUserStoryOptions
): Promise<void> {
  try {
    await setUserStory(workUnitId, options);
    console.log(chalk.green(`✓ User story set for ${workUnitId}`));
    console.log(chalk.gray(`  As a ${options.role}`));
    console.log(chalk.gray(`  I want to ${options.action}`));
    console.log(chalk.gray(`  So that ${options.benefit}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerSetUserStoryCommand(program: Command): void {
  program
    .command('set-user-story')
    .description('Set user story fields for work unit')
    .argument('<work-unit-id>', 'Work unit ID')
    .requiredOption('--role <role>', 'User role (As a...)')
    .requiredOption('--action <action>', 'User action (I want to...)')
    .requiredOption('--benefit <benefit>', 'User benefit (So that...)')
    .action(
      async (
        workUnitId: string,
        options: { role: string; action: string; benefit: string }
      ) => {
        await setUserStoryCommand(workUnitId, options);
      }
    );
}
