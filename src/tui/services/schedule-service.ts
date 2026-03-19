/**
 * Schedule Service — SCHED-008
 *
 * Thin service layer that bridges /schedule slash commands with
 * the existing schedule CRUD operations from src/commands/schedule/.
 * Parses command strings, calls the pure functions, and returns
 * structured results for TUI display.
 */

import { addSchedule } from '../../commands/schedule/add-schedule';
import { removeSchedule } from '../../commands/schedule/remove-schedule';
import {
  pauseSchedule,
  resumeSchedule,
} from '../../commands/schedule/pause-schedule';
import { listSchedules } from '../../commands/schedule/list-schedules';
import { parseScheduleCommand } from '../utils/scheduleCommandParser';
import type { ScheduleEntry } from '../../types/schedule';

/** Result from handling a schedule slash command */
export interface ScheduleCommandResult {
  success: boolean;
  message: string;
}

const USAGE_TEXT = [
  'Usage: /schedule <subcommand> [options]',
  '',
  'Subcommands:',
  '  add <name> --cron "<expr>" --tz <zone> [--role "<role>" --prompt "<prompt>"] [--command "<cmd>"] [--overlap skip|queue]',
  '  list',
  '  pause <name>',
  '  resume <name>',
  '  remove <name>',
].join('\n');

/**
 * Formats a schedule entry for the list table.
 */
function formatScheduleRow(schedule: ScheduleEntry): string {
  const lastRun = schedule.lastRunAt
    ? new Date(schedule.lastRunAt).toLocaleString()
    : '-';
  const nextRun = schedule.status === 'active' ? 'See cron' : 'Paused';

  return [
    schedule.name.padEnd(20),
    schedule.cron.padEnd(15),
    schedule.timezone.padEnd(20),
    schedule.jobType.padEnd(7),
    schedule.status.padEnd(8),
    lastRun.padEnd(22),
    nextRun,
  ].join(' ');
}

/**
 * Formats the list of schedules as a table string.
 */
function formatScheduleTable(schedules: ScheduleEntry[]): string {
  if (schedules.length === 0) {
    return 'No schedules configured.\nUse /schedule add to create a schedule.';
  }

  const header = [
    'Name'.padEnd(20),
    'Cron'.padEnd(15),
    'Timezone'.padEnd(20),
    'Type'.padEnd(7),
    'Status'.padEnd(8),
    'Last Run'.padEnd(22),
    'Next Run',
  ].join(' ');

  const separator = '-'.repeat(header.length);
  const rows = schedules.map(formatScheduleRow);

  return [
    header,
    separator,
    ...rows,
    '',
    `Total: ${schedules.length} schedule(s)`,
  ].join('\n');
}

/**
 * Infers job type from parsed command flags.
 * If --command is present → shell; if --role or --prompt → agent.
 */
function inferJobType(parsed: {
  command?: string;
  role?: string;
  prompt?: string;
}): 'agent' | 'shell' | undefined {
  if (parsed.command) {
    return 'shell';
  }
  if (parsed.role || parsed.prompt) {
    return 'agent';
  }
  return undefined;
}

/**
 * Handles a /schedule slash command string end-to-end.
 *
 * @param input - The raw slash command (e.g., '/schedule add ...')
 * @param cwd - Working directory for schedule file resolution
 * @returns Result with success status and display message
 */
export async function handleScheduleCommand(
  input: string,
  cwd: string
): Promise<ScheduleCommandResult> {
  const parsed = parseScheduleCommand(input);

  if (parsed.subcommand === 'help') {
    return { success: true, message: USAGE_TEXT };
  }

  try {
    switch (parsed.subcommand) {
      case 'add': {
        if (!parsed.name) {
          return { success: false, message: '✗ Schedule name is required' };
        }
        if (!parsed.cron) {
          return { success: false, message: '✗ --cron flag is required' };
        }
        if (!parsed.timezone) {
          return { success: false, message: '✗ --tz flag is required' };
        }

        const jobType = inferJobType(parsed);
        if (!jobType) {
          return {
            success: false,
            message:
              '✗ Specify --role and --prompt for agent schedules, or --command for shell schedules',
          };
        }

        if (jobType === 'agent' && (!parsed.role || !parsed.prompt)) {
          return {
            success: false,
            message: '✗ Agent schedules require --role and --prompt',
          };
        }

        const result = await addSchedule({
          name: parsed.name,
          cron: parsed.cron,
          timezone: parsed.timezone,
          jobType,
          role: parsed.role,
          prompt: parsed.prompt,
          command: parsed.command,
          overlapPolicy: parsed.overlapPolicy,
          cwd,
        });

        const typeName = result.schedule.jobType;
        return {
          success: true,
          message: `✓ Schedule "${parsed.name}" added (${typeName}, ${parsed.cron}, ${parsed.timezone})`,
        };
      }

      case 'list': {
        const result = await listSchedules({ cwd });
        return {
          success: true,
          message: formatScheduleTable(result.schedules),
        };
      }

      case 'pause': {
        if (!parsed.name) {
          return { success: false, message: '✗ Schedule name is required' };
        }
        await pauseSchedule({ name: parsed.name, cwd });
        return {
          success: true,
          message: `✓ Schedule "${parsed.name}" paused`,
        };
      }

      case 'resume': {
        if (!parsed.name) {
          return { success: false, message: '✗ Schedule name is required' };
        }
        await resumeSchedule({ name: parsed.name, cwd });
        return {
          success: true,
          message: `✓ Schedule "${parsed.name}" resumed`,
        };
      }

      case 'remove': {
        if (!parsed.name) {
          return { success: false, message: '✗ Schedule name is required' };
        }
        await removeSchedule({ name: parsed.name, cwd });
        return {
          success: true,
          message: `✓ Schedule "${parsed.name}" removed`,
        };
      }

      default:
        return { success: true, message: USAGE_TEXT };
    }
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    return { success: false, message: `✗ ${message}` };
  }
}
