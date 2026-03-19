/**
 * Pause/Resume Schedule Commands - SCHED-002
 *
 * Pauses or resumes a schedule in spec/schedules.json.
 */

import type { Command } from 'commander';
import { fileManager } from '../../utils/file-manager';
import { getSchedulesFilePath } from '../../utils/ensure-schedules-file';
import { output } from '../../utils/output';
import type {
  ScheduleNameOptions,
  ScheduleOperationResult,
  SchedulesData,
} from '../../types/schedule';

/**
 * Pauses a schedule by setting its status to 'paused'.
 *
 * @param options - Options containing the schedule name
 * @returns Result with success status
 */
export async function pauseSchedule(
  options: ScheduleNameOptions
): Promise<ScheduleOperationResult> {
  const cwd = options.cwd || process.cwd();
  const schedulesFile = getSchedulesFilePath(cwd);

  await fileManager.transaction<SchedulesData>(schedulesFile, async data => {
    const schedule = data.schedules[options.name];
    if (!schedule) {
      throw new Error(`Schedule '${options.name}' does not exist`);
    }

    if (schedule.status === 'paused') {
      throw new Error(`Schedule '${options.name}' is already paused`);
    }

    schedule.status = 'paused';
  });

  return { success: true };
}

/**
 * Resumes a schedule by setting its status to 'active'.
 *
 * @param options - Options containing the schedule name
 * @returns Result with success status
 */
export async function resumeSchedule(
  options: ScheduleNameOptions
): Promise<ScheduleOperationResult> {
  const cwd = options.cwd || process.cwd();
  const schedulesFile = getSchedulesFilePath(cwd);

  await fileManager.transaction<SchedulesData>(schedulesFile, async data => {
    const schedule = data.schedules[options.name];
    if (!schedule) {
      throw new Error(`Schedule '${options.name}' does not exist`);
    }

    if (schedule.status === 'active') {
      throw new Error(`Schedule '${options.name}' is already active`);
    }

    schedule.status = 'active';
  });

  return { success: true };
}

export function registerPauseScheduleCommand(program: Command): void {
  program
    .command('pause-schedule')
    .description('Pause a scheduled job')
    .argument('<name>', 'Schedule name to pause')
    .action(async (name: string) => {
      try {
        await pauseSchedule({ name });
        output.log(`✓ Schedule '${name}' paused successfully`);
      } catch (error: unknown) {
        output.error('✗ Failed to pause schedule:', (error as Error).message);
        process.exit(1);
      }
    });
}

export function registerResumeScheduleCommand(program: Command): void {
  program
    .command('resume-schedule')
    .description('Resume a paused scheduled job')
    .argument('<name>', 'Schedule name to resume')
    .action(async (name: string) => {
      try {
        await resumeSchedule({ name });
        output.log(`✓ Schedule '${name}' resumed successfully`);
      } catch (error: unknown) {
        output.error('✗ Failed to resume schedule:', (error as Error).message);
        process.exit(1);
      }
    });
}
