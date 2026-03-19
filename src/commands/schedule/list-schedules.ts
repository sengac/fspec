/**
 * List Schedules Command - SCHED-002
 *
 * Lists all configured schedules from spec/schedules.json.
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { existsSync } from 'fs';
import { fileManager } from '../../utils/file-manager';
import { getSchedulesFilePath } from '../../utils/ensure-schedules-file';
import { output } from '../../utils/output';
import type {
  ListSchedulesResult,
  SchedulesData,
  ScheduleEntry,
} from '../../types/schedule';

interface ListSchedulesOptions {
  cwd?: string;
  format?: 'table' | 'json';
}

/**
 * Lists all configured schedules.
 *
 * @param options - Options for listing
 * @returns Result with schedules array and column names
 */
export async function listSchedules(
  options: ListSchedulesOptions = {}
): Promise<ListSchedulesResult> {
  const cwd = options.cwd || process.cwd();
  const schedulesFile = getSchedulesFilePath(cwd);

  if (!existsSync(schedulesFile)) {
    return {
      schedules: [],
      columns: [
        'name',
        'cron',
        'timezone',
        'type',
        'status',
        'lastRun',
        'nextRun',
      ],
    };
  }

  const defaultData: SchedulesData = { version: '1.0.0', schedules: {} };
  const data = await fileManager.readJSON<SchedulesData>(
    schedulesFile,
    defaultData
  );
  const schedules = Object.values(data.schedules);

  return {
    schedules,
    columns: [
      'name',
      'cron',
      'timezone',
      'type',
      'status',
      'lastRun',
      'nextRun',
    ],
  };
}

/**
 * Formats a schedule entry for table display.
 */
function formatScheduleRow(schedule: ScheduleEntry): string[] {
  const lastRun = schedule.lastRunAt
    ? new Date(schedule.lastRunAt).toLocaleString()
    : '-';

  // Simple next run calculation would require a cron library
  // For now, just show the cron expression
  const nextRun = schedule.status === 'active' ? 'See cron' : 'Paused';

  return [
    schedule.name,
    schedule.cron,
    schedule.timezone,
    schedule.jobType,
    schedule.status,
    lastRun,
    nextRun,
  ];
}

export function registerListSchedulesCommand(program: Command): void {
  program
    .command('list-schedules')
    .description('List all configured scheduled jobs')
    .option('--json', 'Output as JSON')
    .action(async opts => {
      try {
        const result = await listSchedules({
          format: opts.json ? 'json' : 'table',
        });

        if (opts.json) {
          output.log(JSON.stringify(result.schedules, null, 2));
          return;
        }

        if (result.schedules.length === 0) {
          output.log('No schedules configured.');
          output.log('Use `fspec add-schedule` to create a schedule.');
          return;
        }

        // Table header
        const headers = [
          'Name',
          'Cron',
          'Timezone',
          'Type',
          'Status',
          'Last Run',
          'Next Run',
        ];
        const headerRow = headers.map(h => chalk.bold(h)).join('\t');
        output.log(headerRow);
        output.log('-'.repeat(100));

        // Table rows
        for (const schedule of result.schedules) {
          const row = formatScheduleRow(schedule);
          const statusColor =
            schedule.status === 'active' ? chalk.green : chalk.yellow;
          row[4] = statusColor(row[4]);
          output.log(row.join('\t'));
        }

        output.log('');
        output.log(`Total: ${result.schedules.length} schedule(s)`);
      } catch (error: unknown) {
        output.error('✗ Failed to list schedules:', (error as Error).message);
        process.exit(1);
      }
    });
}
