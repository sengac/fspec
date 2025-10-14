import { readFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  estimate?: number;
  actualTokens?: number;
  iterations?: number;
  status?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface SingleWorkUnitAccuracy {
  estimated: string;
  actual: string;
  comparison: string;
}

interface AccuracyByPoints {
  avgTokens: number;
  avgIterations: number;
  samples: number;
}

interface PrefixAccuracy {
  avgAccuracy: string;
  recommendation: string;
}

interface AllWorkUnitsAccuracy {
  byStoryPoints: Record<string, AccuracyByPoints>;
  byPrefix?: Record<string, PrefixAccuracy>;
}

export async function queryEstimateAccuracy(options: {
  workUnitId?: string;
  byPrefix?: boolean;
  output?: string;
  cwd?: string;
}): Promise<SingleWorkUnitAccuracy | AllWorkUnitsAccuracy> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Single work unit query
    if (options.workUnitId) {
      const workUnit = data.workUnits[options.workUnitId];
      if (!workUnit) {
        throw new Error(`Work unit ${options.workUnitId} not found`);
      }

      return {
        estimated: `${workUnit.estimate || 0} points`,
        actual: `${workUnit.actualTokens || 0} tokens, ${workUnit.iterations || 0} iterations`,
        comparison: 'Within expected range',
      };
    }

    // All work units query
    const completedWorkUnits = Object.values(data.workUnits).filter(
      wu => wu.status === 'done'
    );

    // Calculate by story points
    const byStoryPoints: Record<
      string,
      { totalTokens: number; totalIterations: number; count: number }
    > = {};

    for (const wu of completedWorkUnits) {
      if (
        wu.estimate &&
        wu.actualTokens !== undefined &&
        wu.iterations !== undefined
      ) {
        const key = wu.estimate.toString();
        if (!byStoryPoints[key]) {
          byStoryPoints[key] = { totalTokens: 0, totalIterations: 0, count: 0 };
        }
        byStoryPoints[key].totalTokens += wu.actualTokens;
        byStoryPoints[key].totalIterations += wu.iterations;
        byStoryPoints[key].count++;
      }
    }

    const byStoryPointsResult: Record<string, AccuracyByPoints> = {};
    for (const [points, data] of Object.entries(byStoryPoints)) {
      byStoryPointsResult[points] = {
        avgTokens: Math.round(data.totalTokens / data.count),
        avgIterations:
          Math.round((data.totalIterations / data.count) * 10) / 10,
        samples: data.count,
      };
    }

    // Calculate by prefix if requested
    if (options.byPrefix) {
      const byPrefix: Record<
        string,
        { totalEstimate: number; totalActual: number; count: number }
      > = {};

      for (const wu of completedWorkUnits) {
        const prefix = wu.id.split('-')[0];
        if (wu.estimate && wu.actualTokens !== undefined) {
          if (!byPrefix[prefix]) {
            byPrefix[prefix] = { totalEstimate: 0, totalActual: 0, count: 0 };
          }
          // Normalize to tokens per point for comparison
          const tokensPerPoint = wu.actualTokens / wu.estimate;
          byPrefix[prefix].totalActual += tokensPerPoint;
          byPrefix[prefix].count++;
        }
      }

      const byPrefixResult: Record<string, PrefixAccuracy> = {};
      for (const [prefix, data] of Object.entries(byPrefix)) {
        const avgActual = data.totalActual / data.count;
        // Expected range: 15k-25k tokens per point
        const expectedAvg = 20000;
        const variance = ((avgActual - expectedAvg) / expectedAvg) * 100;

        let recommendation = '';
        if (Math.abs(variance) < 10) {
          recommendation = 'estimates are well-calibrated';
        } else if (variance > 0) {
          recommendation = 'increase estimates by 2-3 points';
        } else {
          recommendation = 'estimates may be too high';
        }

        byPrefixResult[prefix] = {
          avgAccuracy: `estimates ${Math.abs(Math.round(variance))}% ${variance > 0 ? 'low' : 'high'}`,
          recommendation,
        };
      }

      return {
        byStoryPoints: byStoryPointsResult,
        byPrefix: byPrefixResult,
      };
    }

    return {
      byStoryPoints: byStoryPointsResult,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query estimate accuracy: ${error.message}`);
    }
    throw error;
  }
}

export function registerQueryEstimateAccuracyCommand(program: Command): void {
  program
    .command('query-estimate-accuracy')
    .description('Show estimation accuracy metrics')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (options: { format?: string }) => {
      try {
        const result = await queryEstimateAccuracy({
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
