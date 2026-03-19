/**
 * Add Schedule Command - SCHED-002
 *
 * Adds a new schedule (agent or shell) to spec/schedules.json.
 * Validates cron expression, timezone, and schedule name format.
 */

import type { Command } from 'commander';
import { fileManager } from '../../utils/file-manager';
import {
  ensureSchedulesFile,
  getSchedulesFilePath,
} from '../../utils/ensure-schedules-file';
import { validateCronExpression } from '../../utils/validators/cron';
import { validateTimezone } from '../../utils/validators/timezone';
import { output } from '../../utils/output';
import type {
  AddScheduleOptions,
  AddScheduleResult,
  ScheduleEntry,
  SchedulesData,
  AgentScheduleEntry,
  ShellScheduleEntry,
} from '../../types/schedule';

/** Regex for valid schedule names (slug format) */
const SLUG_REGEX = /^[a-z0-9]+(-[a-z0-9]+)*$/;

/**
 * Validates that a schedule name is in slug format.
 *
 * @param name - The schedule name to validate
 * @returns Error message if invalid, undefined if valid
 */
function validateScheduleName(name: string): string | undefined {
  if (!name || typeof name !== 'string') {
    return 'Schedule name is required';
  }

  const trimmed = name.trim();
  if (!SLUG_REGEX.test(trimmed)) {
    return `Invalid schedule name '${name}'. Names must be lowercase, hyphenated slugs (e.g., 'nightly-review', 'daily-sync').`;
  }

  return undefined;
}

/**
 * Adds a new schedule to spec/schedules.json.
 *
 * @param options - Schedule configuration options
 * @returns Result with success status and created schedule
 */
export async function addSchedule(
  options: AddScheduleOptions
): Promise<AddScheduleResult> {
  const cwd = options.cwd || process.cwd();

  // Validate schedule name
  const nameError = validateScheduleName(options.name);
  if (nameError) {
    throw new Error(nameError);
  }

  // Validate cron expression
  const cronResult = validateCronExpression(options.cron);
  if (!cronResult.valid) {
    throw new Error(cronResult.error);
  }

  // Validate timezone
  const tzResult = validateTimezone(options.timezone);
  if (!tzResult.valid) {
    throw new Error(tzResult.error);
  }

  // Validate job-type specific fields
  if (options.jobType === 'agent') {
    if (!options.role || !options.prompt) {
      throw new Error('Agent schedules require both role and prompt');
    }
  } else if (options.jobType === 'shell') {
    if (!options.command) {
      throw new Error('Shell schedules require a command');
    }
  } else {
    throw new Error(
      `Invalid jobType: ${options.jobType}. Must be 'agent' or 'shell'.`
    );
  }

  // Ensure schedules file exists
  await ensureSchedulesFile(cwd);
  const schedulesFile = getSchedulesFilePath(cwd);

  let schedule: ScheduleEntry;

  // Use transaction for atomic write
  await fileManager.transaction<SchedulesData>(schedulesFile, async data => {
    // Check for duplicate
    if (data.schedules[options.name]) {
      throw new Error(`Schedule '${options.name}' already exists`);
    }

    // Create schedule entry
    const baseEntry = {
      name: options.name,
      cron: options.cron,
      timezone: options.timezone,
      overlapPolicy: options.overlapPolicy || 'skip',
      status: 'active' as const,
      lastRunAt: null,
      lastRunStatus: null,
      createdAt: new Date().toISOString(),
    };

    if (options.jobType === 'agent') {
      schedule = {
        ...baseEntry,
        jobType: 'agent',
        role: options.role!,
        prompt: options.prompt!,
      } as AgentScheduleEntry;
    } else {
      schedule = {
        ...baseEntry,
        jobType: 'shell',
        command: options.command!,
      } as ShellScheduleEntry;
    }

    data.schedules[options.name] = schedule;
  });

  return {
    success: true,
    schedule: schedule!,
  };
}

export function registerAddScheduleCommand(program: Command): void {
  program
    .command('add-schedule')
    .description('Add a new scheduled job (agent or shell)')
    .requiredOption('-n, --name <name>', 'Schedule name (slug format)')
    .requiredOption('-c, --cron <expression>', 'Cron expression (5-field)')
    .requiredOption('-z, --timezone <tz>', 'IANA timezone')
    .requiredOption('-t, --type <type>', 'Job type: agent or shell')
    .option('-r, --role <role>', 'Agent role (required for agent type)')
    .option('-p, --prompt <prompt>', 'Agent prompt (required for agent type)')
    .option('--command <command>', 'Shell command (required for shell type)')
    .option('-o, --overlap <policy>', 'Overlap policy: skip or queue', 'skip')
    .action(async opts => {
      try {
        const result = await addSchedule({
          name: opts.name,
          cron: opts.cron,
          timezone: opts.timezone,
          jobType: opts.type,
          role: opts.role,
          prompt: opts.prompt,
          command: opts.command,
          overlapPolicy: opts.overlap,
        });
        output.log(`✓ Schedule '${opts.name}' added successfully`);
        if (result.schedule) {
          output.log(`  Type: ${result.schedule.jobType}`);
          output.log(`  Cron: ${result.schedule.cron}`);
          output.log(`  Timezone: ${result.schedule.timezone}`);
        }
      } catch (error: unknown) {
        output.error('✗ Failed to add schedule:', (error as Error).message);
        process.exit(1);
      }
    });
}
