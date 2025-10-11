import { writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface AddRuleOptions {
  workUnitId: string;
  rule: string;
  cwd?: string;
}

interface AddRuleResult {
  success: boolean;
  ruleCount: number;
}

export async function addRule(options: AddRuleOptions): Promise<AddRuleResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only add rules during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize rules array if it doesn't exist
  if (!workUnit.rules) {
    workUnit.rules = [];
  }

  // Add rule
  workUnit.rules.push(options.rule);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    ruleCount: workUnit.rules.length,
  };
}
