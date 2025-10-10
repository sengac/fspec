import { readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile, ensurePrefixesFile } from '../utils/ensure-files';

interface ListWorkUnitsOptions {
  status?: string;
  prefix?: string;
  epic?: string;
  cwd?: string;
}

interface WorkUnitSummary {
  id: string;
  title: string;
  status: string;
  epic?: string;
  [key: string]: unknown;
}

interface ListWorkUnitsResult {
  workUnits: WorkUnitSummary[];
}

export async function listWorkUnits(options: ListWorkUnitsOptions = {}): Promise<ListWorkUnitsResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units (auto-create if missing)
  const workUnitsData = await ensureWorkUnitsFile(cwd);

  // Ensure prefixes file exists too (for consistency)
  await ensurePrefixesFile(cwd);

  // Get all work units
  let workUnits = Object.values(workUnitsData.workUnits);

  // Apply filters
  if (options.status) {
    workUnits = workUnits.filter(wu => wu.status === options.status);
  }

  if (options.prefix) {
    workUnits = workUnits.filter(wu => wu.id.startsWith(`${options.prefix}-`));
  }

  if (options.epic) {
    workUnits = workUnits.filter(wu => wu.epic === options.epic);
  }

  // Map to summary format
  const summaries: WorkUnitSummary[] = workUnits.map(wu => ({
    id: wu.id,
    title: wu.title,
    status: wu.status,
    ...(wu.epic && { epic: wu.epic }),
  }));

  return {
    workUnits: summaries,
  };
}
