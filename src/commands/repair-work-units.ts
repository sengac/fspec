import { writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface RepairWorkUnitsOptions {
  cwd?: string;
}

interface RepairWorkUnitsResult {
  success: boolean;
  repairs: string[];
  repaired: number;
}

export async function repairWorkUnits(options: RepairWorkUnitsOptions = {}): Promise<RepairWorkUnitsResult> {
  const cwd = options.cwd || process.cwd();
  const repairs: string[] = [];

  // Read work units
  const workUnitsData: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Rebuild states index from scratch
  const newStates: Record<string, string[]> = {
    backlog: [],
    specifying: [],
    testing: [],
    implementing: [],
    validating: [],
    done: [],
    blocked: [],
  };

  // Place each work unit in correct state array
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    const status = workUnit.status;

    if (newStates[status]) {
      newStates[status].push(id);

      // Check if it was in wrong state before
      for (const [stateName, ids] of Object.entries(workUnitsData.states)) {
        if (stateName !== status && ids.includes(id)) {
          repairs.push(`Moved ${id} from ${stateName} to ${status}`);
        }
      }
    }
  }

  // Update states
  workUnitsData.states = newStates;

  // Repair bidirectional dependency relationships
  for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
    // Repair blocks/blockedBy relationships
    if (workUnit.blocks) {
      for (const targetId of workUnit.blocks) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.blockedBy) {
            target.blockedBy = [];
          }
          if (!target.blockedBy.includes(id)) {
            target.blockedBy.push(id);
            repairs.push(`Repaired bidirectional link: ${id} blocks ${targetId}`);
          }
        }
      }
    }

    // Repair blockedBy/blocks relationships
    if (workUnit.blockedBy) {
      for (const targetId of workUnit.blockedBy) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.blocks) {
            target.blocks = [];
          }
          if (!target.blocks.includes(id)) {
            target.blocks.push(id);
            repairs.push(`Repaired bidirectional link: ${targetId} blocks ${id}`);
          }
        }
      }
    }

    // Repair relatesTo relationships (bidirectional)
    if (workUnit.relatesTo) {
      for (const targetId of workUnit.relatesTo) {
        if (workUnitsData.workUnits[targetId]) {
          const target = workUnitsData.workUnits[targetId];
          if (!target.relatesTo) {
            target.relatesTo = [];
          }
          if (!target.relatesTo.includes(id)) {
            target.relatesTo.push(id);
            repairs.push(`Repaired bidirectional link: ${id} relates to ${targetId}`);
          }
        }
      }
    }
  }

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

  return {
    success: true,
    repairs,
    repaired: repairs.length,
  };
}
