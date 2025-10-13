import { writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import type { WorkUnitsData, UserStory } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

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

  await writeFile(workUnitsPath, JSON.stringify(data, null, 2));
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
