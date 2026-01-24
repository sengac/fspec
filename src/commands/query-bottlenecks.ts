import type { WorkUnitsData, WorkUnit } from '../types/work-unit';
import chalk from 'chalk';
import type { Command } from 'commander';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface Bottleneck {
  id: string;
  title: string;
  status: string;
  score: number;
  directBlocks: string[];
  transitiveBlocks: string[];
}

interface QueryBottlenecksOptions {
  cwd?: string;
  output?: 'text' | 'json';
}

interface QueryBottlenecksResult {
  bottlenecks: Bottleneck[];
}

/**
 * Calculate all work units blocked by a given work unit (direct + transitive)
 */
function calculateBlockedWorkUnits(
  workUnits: Record<string, WorkUnit>,
  workUnitId: string,
  visited: Set<string> = new Set()
): Set<string> {
  if (visited.has(workUnitId)) {
    return new Set(); // Avoid infinite loops
  }

  visited.add(workUnitId);

  const workUnit = workUnits[workUnitId];
  if (!workUnit || !workUnit.blocks) {
    return new Set();
  }

  const blocked = new Set<string>();

  // Add direct blocks
  for (const blockedId of workUnit.blocks) {
    blocked.add(blockedId);

    // Add transitive blocks
    const transitiveBlocked = calculateBlockedWorkUnits(
      workUnits,
      blockedId,
      new Set(visited)
    );
    for (const id of transitiveBlocked) {
      blocked.add(id);
    }
  }

  return blocked;
}

export async function queryBottlenecks(
  options: QueryBottlenecksOptions = {}
): Promise<QueryBottlenecksResult> {
  const cwd = options.cwd || process.cwd();

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  const bottlenecks: Bottleneck[] = [];

  // Calculate bottleneck score for each work unit
  for (const workUnit of Object.values(data.workUnits)) {
    // Rule 2: Only work units NOT in 'done' status are bottlenecks
    if (workUnit.status === 'done') {
      continue;
    }

    // Rule 7: Work units in 'blocked' status cannot be progressed
    if (workUnit.status === 'blocked') {
      continue;
    }

    // Skip if no blocks relationships
    if (!workUnit.blocks || workUnit.blocks.length === 0) {
      continue;
    }

    // Calculate all blocked work units (direct + transitive)
    const blockedWorkUnits = calculateBlockedWorkUnits(
      data.workUnits,
      workUnit.id
    );

    const directBlocks = Array.from(workUnit.blocks);
    const transitiveBlocks = Array.from(blockedWorkUnits).filter(
      id => !workUnit.blocks?.includes(id)
    );

    // Rule 3: Bottleneck score = total work units blocked
    const score = blockedWorkUnits.size;

    // Rule 6: Only include bottlenecks with score >= 2
    if (score >= 2) {
      bottlenecks.push({
        id: workUnit.id,
        title: workUnit.title,
        status: workUnit.status,
        score,
        directBlocks,
        transitiveBlocks,
      });
    }
  }

  // Rule 4: Rank bottlenecks by score (highest to lowest)
  bottlenecks.sort((a, b) => b.score - a.score);

  return { bottlenecks };
}

export function registerQueryBottlenecksCommand(program: Command): void {
  program
    .command('query-bottlenecks')
    .description('Identify bottleneck work units blocking the most work')
    .option('--output <format>', 'Output format: text or json', 'text')
    .action(async (options: { output?: string }) => {
      try {
        const result = await queryBottlenecks({
          output: options.output as 'text' | 'json',
        });

        if (options.output === 'json') {
          console.log(JSON.stringify(result, null, 2));
        } else {
          // Text output
          if (result.bottlenecks.length === 0) {
            console.log(chalk.green('✓ No bottlenecks found'));
            return;
          }

          console.log(
            chalk.bold('Bottleneck Work Units (blocking 2+ work units):\n')
          );

          for (const bottleneck of result.bottlenecks) {
            console.log(
              chalk.bold(`${bottleneck.id}`) +
                chalk.gray(` (${bottleneck.status})`) +
                ` - ${bottleneck.title}`
            );
            console.log(
              chalk.yellow(`  Bottleneck Score: ${bottleneck.score}`)
            );
            console.log(
              chalk.gray(
                `  Direct Blocks: ${bottleneck.directBlocks.join(', ')}`
              )
            );
            if (bottleneck.transitiveBlocks.length > 0) {
              console.log(
                chalk.gray(
                  `  Transitive Blocks: ${bottleneck.transitiveBlocks.join(', ')}`
                )
              );
            }
            console.log();
          }

          console.log(
            chalk.bold(`\nTotal bottlenecks: ${result.bottlenecks.length}`)
          );
        }
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(chalk.red('✗ Query failed:'), errorMessage);
        process.exit(1);
      }
    });
}
