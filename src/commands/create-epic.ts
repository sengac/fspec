import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

interface Epic {
  id: string;
  title: string;
  description?: string;
  createdAt: string;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

const EPIC_ID_REGEX = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;

export async function createEpic(options: {
  epicId: string;
  title: string;
  description?: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();
  const epicsFile = join(cwd, 'spec', 'epics.json');

  // Validate epic ID format: lowercase-with-hyphens
  if (!EPIC_ID_REGEX.test(options.epicId)) {
    throw new Error('Epic ID must be lowercase-with-hyphens format (e.g., epic-user-management)');
  }

  try {
    // Ensure spec directory exists
    await mkdir(join(cwd, 'spec'), { recursive: true });

    // Read existing epics or initialize
    let data: EpicsData;
    try {
      const content = await readFile(epicsFile, 'utf-8');
      data = JSON.parse(content);
    } catch {
      data = { epics: {} };
    }

    // Check if epic already exists
    if (data.epics[options.epicId]) {
      throw new Error(`Epic ${options.epicId} already exists`);
    }

    // Create new epic
    const newEpic: Epic = {
      id: options.epicId,
      title: options.title,
      createdAt: new Date().toISOString(),
    };

    if (options.description) {
      newEpic.description = options.description;
    }

    data.epics[options.epicId] = newEpic;

    // Write back to file
    await writeFile(epicsFile, JSON.stringify(data, null, 2));

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to create epic: ${error.message}`);
    }
    throw error;
  }
}

export async function createEpicCommand(
  epicId: string,
  title: string,
  options: { description?: string }
): Promise<void> {
  try {
    const result = await createEpic({
      epicId,
      title,
      description: options.description,
    });

    if (result.success) {
      console.log(chalk.green(`✓ Created epic ${epicId}`));
      console.log(chalk.gray(`  Title: ${title}`));
      if (options.description) {
        console.log(chalk.gray(`  Description: ${options.description}`));
      }
      process.exit(0);
    } else {
      console.error(chalk.red('✗ Failed to create epic'));
      process.exit(1);
    }
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
