import { readFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface QueryDependencyStatsOptions {
  cwd?: string;
}

interface QueryDependencyStatsResult {
  totalBlocks: number;
  totalBlockedBy: number;
  totalDependsOn: number;
  totalRelatesTo: number;
  workUnitsWithDependencies: number;
}

export async function queryDependencyStats(
  options: QueryDependencyStatsOptions = {}
): Promise<QueryDependencyStatsResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  const content = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(content);

  let totalBlocks = 0;
  let totalBlockedBy = 0;
  let totalDependsOn = 0;
  let totalRelatesTo = 0;
  let workUnitsWithDependencies = 0;

  for (const workUnit of Object.values(data.workUnits)) {
    let hasDependencies = false;

    if (workUnit.blocks?.length) {
      totalBlocks += workUnit.blocks.length;
      hasDependencies = true;
    }

    if (workUnit.blockedBy?.length) {
      totalBlockedBy += workUnit.blockedBy.length;
      hasDependencies = true;
    }

    if (workUnit.dependsOn?.length) {
      totalDependsOn += workUnit.dependsOn.length;
      hasDependencies = true;
    }

    if (workUnit.relatesTo?.length) {
      totalRelatesTo += workUnit.relatesTo.length;
      hasDependencies = true;
    }

    if (hasDependencies) {
      workUnitsWithDependencies++;
    }
  }

  return {
    totalBlocks,
    totalBlockedBy,
    totalDependsOn,
    totalRelatesTo,
    workUnitsWithDependencies,
  };
}
