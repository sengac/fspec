import { readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface ShowWorkUnitOptions {
  workUnitId: string;
  output?: 'json' | 'text';
  cwd?: string;
}

interface WorkUnitDetails {
  id: string;
  title: string;
  status: string;
  description?: string;
  estimate?: number;
  epic?: string;
  parent?: string;
  children?: string[];
  blockedBy?: string[];
  rules?: string[];
  examples?: string[];
  questions?: string[];
  assumptions?: string[];
  createdAt: string;
  updatedAt: string;
  [key: string]: unknown;
}

export async function showWorkUnit(options: ShowWorkUnitOptions): Promise<WorkUnitDetails> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];

  return {
    id: workUnit.id,
    title: workUnit.title,
    status: workUnit.status,
    ...(workUnit.description && { description: workUnit.description }),
    ...(workUnit.estimate !== undefined && { estimate: workUnit.estimate }),
    ...(workUnit.epic && { epic: workUnit.epic }),
    ...(workUnit.parent && { parent: workUnit.parent }),
    ...(workUnit.children && { children: workUnit.children }),
    ...(workUnit.blockedBy && { blockedBy: workUnit.blockedBy }),
    ...(workUnit.rules && { rules: workUnit.rules }),
    ...(workUnit.examples && { examples: workUnit.examples }),
    ...(workUnit.questions && { questions: workUnit.questions }),
    ...(workUnit.assumptions && { assumptions: workUnit.assumptions }),
    createdAt: workUnit.createdAt,
    updatedAt: workUnit.updatedAt,
  };
}
