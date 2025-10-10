import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface AddExampleOptions {
  workUnitId: string;
  example: string;
  cwd?: string;
}

interface AddExampleResult {
  success: boolean;
  exampleCount: number;
}

export async function addExample(options: AddExampleOptions): Promise<AddExampleResult> {
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
      `Can only add examples during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize examples array if it doesn't exist
  if (!workUnit.examples) {
    workUnit.examples = [];
  }

  // Add example
  workUnit.examples.push(options.example);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    exampleCount: workUnit.examples.length,
  };
}
