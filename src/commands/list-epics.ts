import { readFile } from 'fs/promises';
import { join } from 'path';

interface Epic {
  id: string;
  title?: string;
  description?: string;
  [key: string]: unknown;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface WorkUnit {
  id: string;
  status?: string;
  epic?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface EpicWithProgress {
  id: string;
  title?: string;
  description?: string;
  totalWorkUnits: number;
  completedWorkUnits: number;
  completionPercentage: number;
}

export async function listEpics(options: {
  cwd?: string;
}): Promise<{ epics: EpicWithProgress[] }> {
  const cwd = options.cwd || process.cwd();
  const epicsFile = join(cwd, 'spec', 'epics.json');
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read epics
    const epicsContent = await readFile(epicsFile, 'utf-8');
    const epicsData: EpicsData = JSON.parse(epicsContent);

    // Read work units to calculate progress
    let workUnitsData: WorkUnitsData | undefined;
    try {
      const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
      workUnitsData = JSON.parse(workUnitsContent);
    } catch {
      // No work units file yet
    }

    // Build epic list with progress
    const epics: EpicWithProgress[] = [];

    for (const epic of Object.values(epicsData.epics)) {
      let totalWorkUnits = 0;
      let completedWorkUnits = 0;

      if (workUnitsData) {
        for (const workUnit of Object.values(workUnitsData.workUnits)) {
          if (workUnit.epic === epic.id) {
            totalWorkUnits++;
            if (workUnit.status === 'done') {
              completedWorkUnits++;
            }
          }
        }
      }

      const completionPercentage = totalWorkUnits > 0
        ? Math.round((completedWorkUnits / totalWorkUnits) * 100)
        : 0;

      epics.push({
        id: epic.id,
        title: epic.title,
        description: epic.description,
        totalWorkUnits,
        completedWorkUnits,
        completionPercentage,
      });
    }

    return { epics };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to list epics: ${error.message}`);
    }
    throw error;
  }
}

export async function listEpicsCommand(): Promise<void> {
  try {
    const result = await listEpics();

    if (result.epics.length === 0) {
      console.log(chalk.yellow('No epics found'));
      process.exit(0);
    }

    console.log(chalk.bold(`\nEpics (${result.epics.length})`));
    console.log('');

    for (const epic of result.epics) {
      console.log(chalk.cyan(epic.id));
      console.log(`  ${epic.title}`);
      if (epic.description) {
        console.log(chalk.gray(`  ${epic.description}`));
      }
      if (epic.workUnitCount !== undefined && epic.workUnitCount > 0) {
        console.log(chalk.gray(`  Work Units: ${epic.workUnitCount}`));
      }
      console.log('');
    }

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
