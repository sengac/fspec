/**
 * Schedule Types - SCHED-002
 *
 * TypeScript interfaces for schedule entries and the schedules.json file format.
 * Corresponds to spec/features/schedule-persistence.feature
 */

/** Job type discriminator */
export type JobType = 'agent' | 'shell';

/** Overlap policy - what to do if previous run is still active */
export type OverlapPolicy = 'skip' | 'queue';

/** Schedule status */
export type ScheduleStatus = 'active' | 'paused';

/** Last run status */
export type LastRunStatus = 'completed' | 'failed' | 'skipped' | null;

/**
 * Base schedule entry fields shared by all job types
 */
export interface ScheduleEntryBase {
  /** Unique schedule identifier (slug format) */
  name: string;
  /** Cron expression (5-field standard syntax) */
  cron: string;
  /** IANA timezone string */
  timezone: string;
  /** Job type discriminator */
  jobType: JobType;
  /** What to do if previous run is still active */
  overlapPolicy: OverlapPolicy;
  /** Whether the schedule triggers */
  status: ScheduleStatus;
  /** Timestamp of last completed run */
  lastRunAt: string | null;
  /** Status of last run */
  lastRunStatus: LastRunStatus;
  /** Creation timestamp (ISO8601) */
  createdAt: string;
}

/**
 * Agent schedule entry - spawns an AI agent session
 */
export interface AgentScheduleEntry extends ScheduleEntryBase {
  jobType: 'agent';
  /** Agent session role */
  role: string;
  /** Initial prompt sent to agent */
  prompt: string;
}

/**
 * Shell schedule entry - executes a shell command
 */
export interface ShellScheduleEntry extends ScheduleEntryBase {
  jobType: 'shell';
  /** Shell command to execute */
  command: string;
}

/**
 * Union type for all schedule entry types
 */
export type ScheduleEntry = AgentScheduleEntry | ShellScheduleEntry;

/**
 * The schedules.json file format
 */
export interface SchedulesData {
  /** Schema version for migrations */
  version: string;
  /** Map of schedule name to schedule entry */
  schedules: Record<string, ScheduleEntry>;
}

/**
 * Options for adding a schedule
 */
export interface AddScheduleOptions {
  name: string;
  cron: string;
  timezone: string;
  jobType: JobType;
  overlapPolicy?: OverlapPolicy;
  // Agent-specific
  role?: string;
  prompt?: string;
  // Shell-specific
  command?: string;
  cwd?: string;
}

/**
 * Result from adding a schedule
 */
export interface AddScheduleResult {
  success: boolean;
  schedule?: ScheduleEntry;
}

/**
 * Options for schedule commands that take a name
 */
export interface ScheduleNameOptions {
  name: string;
  cwd?: string;
}

/**
 * Generic schedule operation result
 */
export interface ScheduleOperationResult {
  success: boolean;
}

/**
 * Result from listing schedules
 */
export interface ListSchedulesResult {
  schedules: ScheduleEntry[];
  columns: string[];
}
