import { readFile } from 'fs/promises';
import { join } from 'path';

interface Prefix {
  prefix: string;
  description: string;
  createdAt: string;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

interface WorkUnit {
  id: string;
  status?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface PrefixWithProgress {
  prefix: string;
  description: string;
  totalWorkUnits: number;
  completedWorkUnits: number;
  completionPercentage: number;
}

export async function listPrefixes(options: {
  cwd?: string;
}): Promise<{ prefixes: PrefixWithProgress[] }> {
  const cwd = options.cwd || process.cwd();
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Read prefixes
  let prefixesData: PrefixesData;
  try {
    const prefixesContent = await readFile(prefixesFile, 'utf-8');
    prefixesData = JSON.parse(prefixesContent);
  } catch (error: unknown) {
    // No prefixes file yet - return empty list
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
      return { prefixes: [] };
    }
    throw error;
  }

  // Read work units to calculate progress
  let workUnitsData: WorkUnitsData | undefined;
  try {
    const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
    workUnitsData = JSON.parse(workUnitsContent);
  } catch {
    // No work units file yet
  }

  // Build prefix list with progress
  const prefixes: PrefixWithProgress[] = [];

  for (const prefix of Object.values(prefixesData.prefixes)) {
    let totalWorkUnits = 0;
    let completedWorkUnits = 0;

    if (workUnitsData) {
      for (const workUnit of Object.values(workUnitsData.workUnits)) {
        // Work unit ID starts with prefix (e.g., "AUTH-001" starts with "AUTH")
        if (workUnit.id.startsWith(prefix.prefix + '-')) {
          totalWorkUnits++;
          if (workUnit.status === 'done') {
            completedWorkUnits++;
          }
        }
      }
    }

    const completionPercentage =
      totalWorkUnits > 0
        ? Math.round((completedWorkUnits / totalWorkUnits) * 100)
        : 0;

    prefixes.push({
      prefix: prefix.prefix,
      description: prefix.description,
      totalWorkUnits,
      completedWorkUnits,
      completionPercentage,
    });
  }

  return { prefixes };
}
