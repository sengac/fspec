import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';

interface StateHistoryEntry {
  state: string;
  timestamp: string;
}

interface WorkUnit {
  id: string;
  stateHistory?: StateHistoryEntry[];
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface MetricsResult {
  cycleTime: string;
  timePerState: Record<string, string>;
}

export async function queryMetrics(options: {
  workUnitId: string;
  cwd?: string;
}): Promise<MetricsResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    const workUnit = data.workUnits[options.workUnitId];

    if (!workUnit.stateHistory || workUnit.stateHistory.length === 0) {
      throw new Error(`Work unit ${options.workUnitId} has no state history`);
    }

    // Calculate cycle time (first to last state)
    const firstTimestamp = new Date(workUnit.stateHistory[0].timestamp);
    const lastTimestamp = new Date(
      workUnit.stateHistory[workUnit.stateHistory.length - 1].timestamp
    );
    const cycleTimeMs = lastTimestamp.getTime() - firstTimestamp.getTime();
    const cycleTimeHours = Math.round(cycleTimeMs / (1000 * 60 * 60));

    // Calculate time per state
    const timePerState: Record<string, string> = {};
    for (let i = 0; i < workUnit.stateHistory.length - 1; i++) {
      const currentState = workUnit.stateHistory[i];
      const nextState = workUnit.stateHistory[i + 1];
      const currentTime = new Date(currentState.timestamp);
      const nextTime = new Date(nextState.timestamp);
      const durationMs = nextTime.getTime() - currentTime.getTime();
      const durationHours = Math.round(durationMs / (1000 * 60 * 60));
      timePerState[currentState.state] =
        `${durationHours} hour${durationHours !== 1 ? 's' : ''}`;
    }

    return {
      cycleTime: `${cycleTimeHours} hour${cycleTimeHours !== 1 ? 's' : ''}`,
      timePerState,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query metrics: ${error.message}`);
    }
    throw error;
  }
}

export function registerQueryMetricsCommand(program: Command): void {
  program
    .command('query-metrics')
    .description('Query project metrics and statistics')
    .option('--metric <metric>', 'Specific metric to query')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(async (options: { metric?: string; format?: string }) => {
      try {
        const result = await queryMetrics({
          metric: options.metric,
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
