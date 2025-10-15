import type { WorkUnitsData, WorkUnit } from '../types';
import chalk from 'chalk';
import type { Command } from 'commander';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

function calculateMaxChainDepth(workUnits: Record<string, WorkUnit>): number {
  let maxDepth = 0;

  // Calculate depth for each work unit
  for (const workUnitId of Object.keys(workUnits)) {
    const depth = calculateDepth(workUnits, workUnitId, new Set());
    maxDepth = Math.max(maxDepth, depth);
  }

  return maxDepth;
}

function calculateDepth(
  workUnits: Record<string, WorkUnit>,
  workUnitId: string,
  visited: Set<string>
): number {
  if (visited.has(workUnitId)) {
    return 0; // Avoid infinite loops
  }

  visited.add(workUnitId);

  const workUnit = workUnits[workUnitId];
  if (!workUnit) {
    return 0;
  }

  let maxChildDepth = 0;

  // Follow blocks relationships to calculate depth
  if (workUnit.blocks) {
    for (const blockedId of workUnit.blocks) {
      const childDepth = calculateDepth(
        workUnits,
        blockedId,
        new Set(visited)
      );
      maxChildDepth = Math.max(maxChildDepth, childDepth);
    }
  }

  return maxChildDepth + (workUnit.blocks && workUnit.blocks.length > 0 ? 1 : 0);
}

interface QueryDependencyStatsOptions {
  cwd?: string;
}

interface QueryDependencyStatsResult {
  totalBlocks: number;
  totalBlockedBy: number;
  totalDependsOn: number;
  totalRelatesTo: number;
  workUnitsWithDependencies: number;
  workUnitsWithBlockers: number; // Count of work units that have blockedBy
  workUnitsBlockingOthers: number; // Count of work units that have blocks
  workUnitsWithSoftDependencies: number; // Count of work units that have dependsOn
  averageDependenciesPerUnit: number;
  maxDependencyChainDepth: number;
}

export async function queryDependencyStats(
  options: QueryDependencyStatsOptions = {}
): Promise<QueryDependencyStatsResult> {
  const cwd = options.cwd || process.cwd();

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  let totalBlocks = 0;
  let totalBlockedBy = 0;
  let totalDependsOn = 0;
  let totalRelatesTo = 0;
  let workUnitsWithDependencies = 0;
  let workUnitsWithBlockers = 0;
  let workUnitsBlockingOthers = 0;
  let workUnitsWithSoftDependencies = 0;

  const workUnitArray = Object.values(data.workUnits);

  for (const workUnit of workUnitArray) {
    let hasDependencies = false;

    if (workUnit.blocks?.length) {
      totalBlocks += workUnit.blocks.length;
      workUnitsBlockingOthers++;
      hasDependencies = true;
    }

    if (workUnit.blockedBy?.length) {
      totalBlockedBy += workUnit.blockedBy.length;
      workUnitsWithBlockers++;
      hasDependencies = true;
    }

    if (workUnit.dependsOn?.length) {
      totalDependsOn += workUnit.dependsOn.length;
      workUnitsWithSoftDependencies++;
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

  // Calculate average dependencies per unit
  const totalDependencies =
    totalBlocks + totalBlockedBy + totalDependsOn + totalRelatesTo;
  const averageDependenciesPerUnit =
    workUnitArray.length > 0 ? totalDependencies / workUnitArray.length : 0;

  // Calculate max dependency chain depth
  const maxDependencyChainDepth = calculateMaxChainDepth(data.workUnits);

  return {
    totalBlocks,
    totalBlockedBy,
    totalDependsOn,
    totalRelatesTo,
    workUnitsWithDependencies,
    workUnitsWithBlockers,
    workUnitsBlockingOthers,
    workUnitsWithSoftDependencies,
    averageDependenciesPerUnit: Math.round(averageDependenciesPerUnit * 100) / 100,
    maxDependencyChainDepth,
  };
}

export function registerQueryDependencyStatsCommand(program: Command): void {
  program
    .command('query-dependency-stats')
    .description('Show dependency statistics and potential blockers')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (options: { format?: string }) => {
      try {
        const result = await queryDependencyStats({
          format: options.format as 'text' | 'json',
        });
        if (options.format === 'json') {
          console.log(JSON.stringify(result, null, 2));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Query failed:'), error.message);
        process.exit(1);
      }
    });
}
