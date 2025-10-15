import { readFile } from 'fs/promises';
import chalk from 'chalk';
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
  cycleTime?: string;
  timePerState?: Record<string, string>;
  aggregateMetrics?: {
    totalWorkUnits: number;
    completedWorkUnits: number;
    averageCycleTime?: string;
    byType?: Record<string, { count: number; averageCycleTime?: string }>;
  };
}

export async function queryMetrics(options: {
  workUnitId?: string;
  type?: 'story' | 'task' | 'bug';
  cwd?: string;
}): Promise<MetricsResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Single work unit metrics
    if (options.workUnitId) {
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
    }

    // Aggregate metrics (when no specific work unit ID)
    let workUnits = Object.values(data.workUnits);

    // Filter by type if specified
    if (options.type) {
      workUnits = workUnits.filter(wu => {
        const type = wu.type || 'story';
        return type === options.type;
      });
    }

    const totalWorkUnits = workUnits.length;
    const completedWorkUnits = workUnits.filter(wu => wu.status === 'done')
      .length;

    // Calculate average cycle time for completed work units
    const completedWithHistory = workUnits.filter(
      wu => wu.status === 'done' && wu.stateHistory && wu.stateHistory.length > 0
    );

    let averageCycleTime: string | undefined;
    if (completedWithHistory.length > 0) {
      const totalCycleTimeMs = completedWithHistory.reduce((sum, wu) => {
        const firstTimestamp = new Date(wu.stateHistory![0].timestamp);
        const lastTimestamp = new Date(
          wu.stateHistory![wu.stateHistory!.length - 1].timestamp
        );
        return sum + (lastTimestamp.getTime() - firstTimestamp.getTime());
      }, 0);
      const avgHours = Math.round(
        totalCycleTimeMs / (1000 * 60 * 60) / completedWithHistory.length
      );
      averageCycleTime = `${avgHours} hour${avgHours !== 1 ? 's' : ''}`;
    }

    // Break down by type if no type filter specified
    let byType: Record<string, { count: number; averageCycleTime?: string }> | undefined;
    if (!options.type) {
      byType = {};
      const types = ['story', 'task', 'bug'] as const;
      for (const type of types) {
        const typeWorkUnits = workUnits.filter(wu => {
          const wuType = wu.type || 'story';
          return wuType === type;
        });
        const typeCompleted = typeWorkUnits.filter(
          wu =>
            wu.status === 'done' && wu.stateHistory && wu.stateHistory.length > 0
        );

        let typeAvgCycleTime: string | undefined;
        if (typeCompleted.length > 0) {
          const typeTotalCycleTimeMs = typeCompleted.reduce((sum, wu) => {
            const firstTimestamp = new Date(wu.stateHistory![0].timestamp);
            const lastTimestamp = new Date(
              wu.stateHistory![wu.stateHistory!.length - 1].timestamp
            );
            return sum + (lastTimestamp.getTime() - firstTimestamp.getTime());
          }, 0);
          const typeAvgHours = Math.round(
            typeTotalCycleTimeMs / (1000 * 60 * 60) / typeCompleted.length
          );
          typeAvgCycleTime = `${typeAvgHours} hour${typeAvgHours !== 1 ? 's' : ''}`;
        }

        byType[type] = {
          count: typeWorkUnits.length,
          averageCycleTime: typeAvgCycleTime,
        };
      }
    }

    return {
      aggregateMetrics: {
        totalWorkUnits,
        completedWorkUnits,
        averageCycleTime,
        byType,
      },
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
    .option('--work-unit-id <id>', 'Specific work unit to query metrics for')
    .option('--type <type>', 'Filter by work item type: story, task, or bug')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(
      async (options: {
        workUnitId?: string;
        type?: 'story' | 'task' | 'bug';
        format?: string;
      }) => {
        try {
          const result = await queryMetrics({
            workUnitId: options.workUnitId,
            type: options.type,
          });
          if (options.format === 'json') {
            console.log(JSON.stringify(result, null, 2));
          } else {
            // Text output for aggregate metrics
            if (result.aggregateMetrics) {
              console.log(chalk.bold('\nProject Metrics'));
              console.log('');
              console.log(
                `Total Work Units: ${result.aggregateMetrics.totalWorkUnits}`
              );
              console.log(
                `Completed Work Units: ${result.aggregateMetrics.completedWorkUnits}`
              );
              if (result.aggregateMetrics.averageCycleTime) {
                console.log(
                  `Average Cycle Time: ${result.aggregateMetrics.averageCycleTime}`
                );
              }
              if (result.aggregateMetrics.byType) {
                console.log('');
                console.log(chalk.bold('By Type:'));
                for (const [type, data] of Object.entries(
                  result.aggregateMetrics.byType
                )) {
                  console.log(
                    `  ${type}: ${data.count} work unit${data.count !== 1 ? 's' : ''}`
                  );
                  if (data.averageCycleTime) {
                    console.log(`    Average Cycle Time: ${data.averageCycleTime}`);
                  }
                }
              }
            } else if (result.cycleTime) {
              // Single work unit output
              console.log(chalk.bold('\nWork Unit Metrics'));
              console.log('');
              console.log(`Cycle Time: ${result.cycleTime}`);
              if (result.timePerState) {
                console.log('');
                console.log(chalk.bold('Time Per State:'));
                for (const [state, time] of Object.entries(result.timePerState)) {
                  console.log(`  ${state}: ${time}`);
                }
              }
            }
          }
        } catch (error: any) {
          console.error(chalk.red('✗ Query failed:'), error.message);
          process.exit(1);
        }
      }
    );
}
