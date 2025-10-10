import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface Epic {
  id: string;
  [key: string]: unknown;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface Prefix {
  prefix: string;
  epicId?: string;
  [key: string]: unknown;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

interface WorkUnit {
  id: string;
  epic?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function deleteEpic(options: {
  epicId: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();
  const epicsFile = join(cwd, 'spec', 'epics.json');
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read and update epics.json
    const epicsContent = await readFile(epicsFile, 'utf-8');
    const epicsData: EpicsData = JSON.parse(epicsContent);

    if (!epicsData.epics[options.epicId]) {
      throw new Error(`Epic ${options.epicId} not found`);
    }

    // Delete the epic
    delete epicsData.epics[options.epicId];
    await writeFile(epicsFile, JSON.stringify(epicsData, null, 2));

    // Update prefixes.json - remove epic references
    try {
      const prefixesContent = await readFile(prefixesFile, 'utf-8');
      const prefixesData: PrefixesData = JSON.parse(prefixesContent);

      for (const prefix of Object.values(prefixesData.prefixes)) {
        if (prefix.epicId === options.epicId) {
          delete prefix.epicId;
        }
      }

      await writeFile(prefixesFile, JSON.stringify(prefixesData, null, 2));
    } catch {
      // No prefixes file yet
    }

    // Update work-units.json - remove epic references
    try {
      const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
      const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

      for (const workUnit of Object.values(workUnitsData.workUnits)) {
        if (workUnit.epic === options.epicId) {
          delete workUnit.epic;
        }
      }

      await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));
    } catch {
      // No work units file yet
    }

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to delete epic: ${error.message}`);
    }
    throw error;
  }
}
