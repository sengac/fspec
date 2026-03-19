/**
 * Remove Schedule Command - SCHED-002
 *
 * Removes a schedule from spec/schedules.json.
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
 * Removes a schedule from spec/schedules.json.
 *
 * @param options - Options containing the schedule name
 * @returns Result with success status
 */
export async function removeSchedule(
  options: ScheduleNameOptions
): Promise<ScheduleOperationResult> {
  const cwd = options.cwd || process.cwd();
  const schedulesFile = getSchedulesFilePath(cwd);

  await fileManager.transaction<SchedulesData>(schedulesFile, async data => {
    if (!data.schedules[options.name]) {
      throw new Error(`Schedule '${options.name}' does not exist`);
    }

    delete data.schedules[options.name];
  });

  return { success: true };
}

export function registerRemoveScheduleCommand(program: Command): void {
  program
    .command('remove-schedule')
    .description('Remove a scheduled job')
    .argument('<name>', 'Schedule name to remove')
    .action(async (name: string) => {
      try {
        await removeSchedule({ name });
        output.log(`✓ Schedule '${name}' removed successfully`);
      } catch (error: unknown) {
        output.error('✗ Failed to remove schedule:', (error as Error).message);
        process.exit(1);
      }
    });
}
