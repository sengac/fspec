import { readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

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

  // Read work units
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

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
