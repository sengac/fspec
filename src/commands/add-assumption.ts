import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface AddAssumptionOptions {
  workUnitId: string;
  assumption: string;
  cwd?: string;
}

interface AddAssumptionResult {
  success: boolean;
  assumptionCount: number;
}

export async function addAssumption(options: AddAssumptionOptions): Promise<AddAssumptionResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units
  const content = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(content);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only add assumptions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize assumptions array if it doesn't exist
  if (!workUnit.assumptions) {
    workUnit.assumptions = [];
  }

  // Add assumption
  workUnit.assumptions.push(options.assumption);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    assumptionCount: workUnit.assumptions.length,
  };
}
