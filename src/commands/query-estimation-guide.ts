import { readFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  estimate?: number;
  iterations?: number;
  status?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface EstimationPattern {
  points: number;
  expectedIterations: string;
  confidence: string;
}

interface EstimationGuideResult {
  patterns: EstimationPattern[];
}

export async function queryEstimationGuide(options: {
  cwd?: string;
}): Promise<EstimationGuideResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Get completed work units
    const completedWorkUnits = Object.values(data.workUnits).filter(
      wu => wu.status === 'done'
    );

    // Group by story points
    const byPoints: Record<number, { iterations: number[] }> = {};

    for (const wu of completedWorkUnits) {
      if (wu.estimate && wu.iterations !== undefined) {
        if (!byPoints[wu.estimate]) {
          byPoints[wu.estimate] = { iterations: [] };
        }
        byPoints[wu.estimate].iterations.push(wu.iterations);
      }
    }

    // Calculate patterns
    const patterns: EstimationPattern[] = [];

    for (const [points, data] of Object.entries(byPoints)) {
      const pointsNum = parseInt(points);
      const minIterations = Math.min(...data.iterations);
      const maxIterations = Math.max(...data.iterations);

      // Determine confidence based on sample size
      let confidence = 'low';
      if (data.iterations.length >= 4) {
        confidence = 'high';
      } else if (data.iterations.length >= 2) {
        confidence = 'medium';
      }

      patterns.push({
        points: pointsNum,
        expectedIterations: `${minIterations}-${maxIterations}`,
        confidence,
      });
    }

    // Sort by points
    patterns.sort((a, b) => a.points - b.points);

    return { patterns };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query estimation guide: ${error.message}`);
    }
    throw error;
  }
}

export function registerQueryEstimationGuideCommand(program: Command): void {
  program
    .command('query-estimation-guide')
    .description('Get estimation guidance based on historical data')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (workUnitId: string, options: { format?: string }) => {
      try {
        const result = await queryEstimationGuide({
          workUnitId,
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
