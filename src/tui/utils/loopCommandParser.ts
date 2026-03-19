/**
 * Parser for /loop slash command — deterministic interval extraction.
 * Converts natural language into cron expressions without LLM round-trip.
 *
 * Intervals are stored in seconds for sub-minute granularity.
 * The scheduler engine ticks every 30 seconds, so that is the
 * effective minimum resolution.
 *
 * SCHED-011: Loop Shorthand — Natural Language Schedule Creation
 */

interface ParsedLoopAddCommand {
  subcommand: 'add';
  prompt: string;
  intervalSeconds: number;
  cron: string;
}

interface ParsedLoopCancelCommand {
  subcommand: 'cancel';
  jobId: string;
}

interface ParsedLoopListCommand {
  subcommand: 'list';
}

interface ParsedLoopHelpCommand {
  subcommand: 'help';
}

export type ParsedLoopCommand =
  | ParsedLoopAddCommand
  | ParsedLoopCancelCommand
  | ParsedLoopListCommand
  | ParsedLoopHelpCommand;

const LEADING_INTERVAL_RE = /^(\d+)([smhd])$/;
const TRAILING_INTERVAL_RE =
  /\s+every\s+(\d+)\s*(s|sec|seconds?|m|min|minutes?|h|hrs?|hours?|d|days?)$/i;

function unitToSeconds(value: number, unit: string): number {
  const u = unit.toLowerCase();
  if (u === 's' || u === 'sec' || u === 'second' || u === 'seconds') {
    return Math.max(1, value);
  }
  if (u === 'm' || u === 'min' || u === 'minute' || u === 'minutes') {
    return value * 60;
  }
  if (u === 'h' || u === 'hr' || u === 'hrs' || u === 'hour' || u === 'hours') {
    return value * 3600;
  }
  if (u === 'd' || u === 'day' || u === 'days') {
    return value * 86400;
  }
  return value;
}

function secondsToCron(seconds: number): string {
  // Cron only supports minute-level granularity at best.
  // For sub-minute intervals, use the fastest cron: every 1 minute.
  const minutes = Math.max(1, Math.ceil(seconds / 60));
  if (minutes < 60) {
    return `*/${minutes} * * * *`;
  }
  const hours = minutes / 60;
  if (hours < 24 && Number.isInteger(hours)) {
    return `0 */${hours} * * *`;
  }
  const days = minutes / 1440;
  if (Number.isInteger(days)) {
    return `0 0 */${days} * *`;
  }
  // Fallback for non-standard intervals: use minute cron
  return `*/${minutes} * * * *`;
}

export function parseLoopCommand(input: string): ParsedLoopCommand {
  // Strip leading "/loop"
  const body = input.replace(/^\/loop\s*/, '').trim();

  // Empty → help
  if (!body) {
    return { subcommand: 'help' };
  }

  // Subcommands: cancel, list
  if (body === 'list') {
    return { subcommand: 'list' };
  }

  if (body.startsWith('cancel ')) {
    const jobId = body.slice('cancel '.length).trim();
    return { subcommand: 'cancel', jobId };
  }

  // Parse interval + prompt
  const tokens = body.split(/\s+/);
  const firstToken = tokens[0];
  const leadingMatch = LEADING_INTERVAL_RE.exec(firstToken);

  if (leadingMatch) {
    // Leading interval: "/loop 5m check deployment status"
    const rawValue = parseInt(leadingMatch[1], 10);
    const rawUnit = leadingMatch[2];
    const intervalSeconds = unitToSeconds(rawValue, rawUnit);
    const prompt = tokens.slice(1).join(' ');

    return {
      subcommand: 'add',
      prompt,
      intervalSeconds,
      cron: secondsToCron(intervalSeconds),
    };
  }

  // Check for trailing "every N unit" clause
  const trailingMatch = TRAILING_INTERVAL_RE.exec(body);
  if (trailingMatch) {
    const rawValue = parseInt(trailingMatch[1], 10);
    const rawUnit = trailingMatch[2];
    const intervalSeconds = unitToSeconds(rawValue, rawUnit);
    const prompt = body.replace(TRAILING_INTERVAL_RE, '').trim();

    return {
      subcommand: 'add',
      prompt,
      intervalSeconds,
      cron: secondsToCron(intervalSeconds),
    };
  }

  // No interval found → default to 10 minutes (600 seconds)
  return {
    subcommand: 'add',
    prompt: body,
    intervalSeconds: 600,
    cron: '*/10 * * * *',
  };
}
