/**
 * Schedule Command Parser — SCHED-008
 *
 * Parses /schedule slash command strings into structured arguments.
 * Handles quoted strings, named flags (--cron, --tz, etc.), and
 * positional arguments (subcommand, name).
 */

/** Valid subcommands for the /schedule slash command */
const VALID_SUBCOMMANDS = new Set(['add', 'list', 'pause', 'resume', 'remove']);

/** Parsed result from a /schedule slash command */
export interface ParsedScheduleCommand {
  subcommand: 'add' | 'list' | 'pause' | 'resume' | 'remove' | 'help';
  name?: string;
  cron?: string;
  timezone?: string;
  role?: string;
  prompt?: string;
  command?: string;
  overlapPolicy?: 'skip' | 'queue';
}

/**
 * Tokenizes a command string, respecting quoted strings.
 * Handles both single and double quotes.
 *
 * @param input - Raw command string
 * @returns Array of tokens with quotes stripped
 */
function tokenize(input: string): string[] {
  const tokens: string[] = [];
  let current = '';
  let inQuote: string | null = null;

  for (let i = 0; i < input.length; i++) {
    const ch = input[i];

    if (inQuote) {
      if (ch === inQuote) {
        inQuote = null;
      } else {
        current += ch;
      }
    } else if (ch === '"' || ch === "'") {
      inQuote = ch;
    } else if (ch === ' ' || ch === '\t') {
      if (current.length > 0) {
        tokens.push(current);
        current = '';
      }
    } else {
      current += ch;
    }
  }

  if (current.length > 0) {
    tokens.push(current);
  }

  return tokens;
}

/**
 * Parses a /schedule slash command string into structured arguments.
 *
 * @param input - The raw slash command text (e.g., '/schedule add ...')
 * @returns Parsed command with subcommand and arguments
 */
export function parseScheduleCommand(input: string): ParsedScheduleCommand {
  const trimmed = input.trim();
  const tokens = tokenize(trimmed);

  // Remove leading '/schedule'
  if (tokens.length > 0 && tokens[0] === '/schedule') {
    tokens.shift();
  }

  // No subcommand → help
  if (tokens.length === 0) {
    return { subcommand: 'help' };
  }

  const sub = tokens[0].toLowerCase();
  if (!VALID_SUBCOMMANDS.has(sub)) {
    return { subcommand: 'help' };
  }

  const subcommand = sub as ParsedScheduleCommand['subcommand'];

  // For list, no further args needed
  if (subcommand === 'list') {
    return { subcommand: 'list' };
  }

  // For pause/resume/remove, second token is the name
  if (
    subcommand === 'pause' ||
    subcommand === 'resume' ||
    subcommand === 'remove'
  ) {
    return {
      subcommand,
      name: tokens[1],
    };
  }

  // For add, parse name (positional) and flags
  const result: ParsedScheduleCommand = { subcommand: 'add' };

  // Second token is the name (positional)
  if (tokens.length > 1 && !tokens[1].startsWith('--')) {
    result.name = tokens[1];
  }

  // Parse named flags
  for (let i = 2; i < tokens.length; i++) {
    const flag = tokens[i];
    const value = tokens[i + 1];

    switch (flag) {
      case '--cron':
        result.cron = value;
        i++;
        break;
      case '--tz':
      case '--timezone':
        result.timezone = value;
        i++;
        break;
      case '--role':
        result.role = value;
        i++;
        break;
      case '--prompt':
        result.prompt = value;
        i++;
        break;
      case '--command':
        result.command = value;
        i++;
        break;
      case '--overlap':
        if (value === 'skip' || value === 'queue') {
          result.overlapPolicy = value;
        }
        i++;
        break;
      default:
        // Unknown flag — skip
        break;
    }
  }

  return result;
}
