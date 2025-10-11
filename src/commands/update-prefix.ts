import { writeFile } from 'fs/promises';
import { join } from 'path';
import { ensurePrefixesFile, ensureEpicsFile } from '../utils/ensure-files';

interface Prefix {
  prefix: string;
  description?: string;
  epicId?: string;
  createdAt?: string;
  updatedAt?: string;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

interface EpicsData {
  epics: Record<string, { id: string }>;
}

export async function updatePrefix(options: {
  prefix: string;
  epicId?: string;
  description?: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();

  try {
    // Read existing prefixes
    const data: PrefixesData = await ensurePrefixesFile(cwd);
    const prefixesFile = join(cwd, 'spec', 'prefixes.json');

    // Check if prefix exists
    if (!data.prefixes[options.prefix]) {
      throw new Error(`Prefix ${options.prefix} not found`);
    }

    // If epicId is provided, verify it exists
    if (options.epicId) {
      const epicsData: EpicsData = await ensureEpicsFile(cwd);
      if (!epicsData.epics[options.epicId]) {
        throw new Error(`Epic ${options.epicId} not found`);
      }
    }

    // Update prefix
    if (options.epicId !== undefined) {
      data.prefixes[options.prefix].epicId = options.epicId;
    }

    if (options.description !== undefined) {
      data.prefixes[options.prefix].description = options.description;
    }

    data.prefixes[options.prefix].updatedAt = new Date().toISOString();

    // Write back to file
    await writeFile(prefixesFile, JSON.stringify(data, null, 2));

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to update prefix: ${error.message}`);
    }
    throw error;
  }
}
