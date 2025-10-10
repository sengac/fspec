import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData, PrefixesData, EpicsData } from '../types';

/**
 * Ensures spec/work-units.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty work units and all Kanban states.
 */
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const filePath = join(cwd, 'spec/work-units.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      await mkdir(join(cwd, 'spec'), { recursive: true });

      const initialData: WorkUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/prefixes.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty prefixes object.
 */
export async function ensurePrefixesFile(cwd: string): Promise<PrefixesData> {
  const filePath = join(cwd, 'spec/prefixes.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      await mkdir(join(cwd, 'spec'), { recursive: true });

      const initialData: PrefixesData = {
        prefixes: {},
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

/**
 * Ensures spec/epics.json exists with proper initial structure.
 * If the file doesn't exist, creates it with empty epics object.
 */
export async function ensureEpicsFile(cwd: string): Promise<EpicsData> {
  const filePath = join(cwd, 'spec/epics.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error: unknown) {
    // File doesn't exist, create it with initial structure
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      await mkdir(join(cwd, 'spec'), { recursive: true });

      const initialData: EpicsData = {
        epics: {},
      };

      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}
