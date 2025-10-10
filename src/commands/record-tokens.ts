import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  actualTokens?: number;
  updatedAt: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function recordTokens(options: {
  workUnitId: string;
  tokens: number;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Load work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    // Add tokens
    const workUnit = data.workUnits[options.workUnitId];
    workUnit.actualTokens = (workUnit.actualTokens || 0) + options.tokens;
    workUnit.updatedAt = new Date().toISOString();

    // Save work units
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return {
      success: true,
      totalTokens: workUnit.actualTokens,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to record tokens: ${error.message}`);
    }
    throw error;
  }
}
