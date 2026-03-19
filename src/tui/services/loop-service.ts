/**
 * Service layer for /loop slash command — bridges TUI input parsing
 * to the Rust-side LoopStore for execution by the scheduler engine.
 *
 * Parsing stays in TypeScript (loopCommandParser.ts).
 * Storage + execution lives in Rust (scheduler/loop_store.rs + engine.rs).
 *
 * SCHED-011: Loop Shorthand — Natural Language Schedule Creation
 */

import { randomBytes } from 'crypto';
import { parseLoopCommand } from '../utils/loopCommandParser';

export interface LoopCommandResult {
  success: boolean;
  message: string;
}

function generateJobId(): string {
  return randomBytes(4).toString('hex');
}

function formatInterval(seconds: number): string {
  if (seconds < 60) {
    return `${seconds} second${seconds === 1 ? '' : 's'}`;
  }
  const minutes = seconds / 60;
  if (Number.isInteger(minutes) && minutes < 60) {
    return `${minutes} minute${minutes === 1 ? '' : 's'}`;
  }
  if (!Number.isInteger(minutes) && seconds < 3600) {
    return `${seconds} seconds`;
  }
  const hours = seconds / 3600;
  if (hours < 24 && Number.isInteger(hours)) {
    return `${hours} hour${hours === 1 ? '' : 's'}`;
  }
  const days = seconds / 86400;
  if (Number.isInteger(days)) {
    return `${days} day${days === 1 ? '' : 's'}`;
  }
  if (Number.isInteger(minutes)) {
    return `${minutes} minutes`;
  }
  return `${seconds} seconds`;
}

/**
 * Handle a /loop command by parsing it and bridging to the Rust LoopStore.
 *
 * @param input - The raw user input (e.g., "/loop 5m check status")
 * @param sessionId - The active session ID (required for add/list/cancel)
 */
export async function handleLoopCommand(
  input: string,
  sessionId: string | null
): Promise<LoopCommandResult> {
  try {
    const parsed = parseLoopCommand(input);

    if (parsed.subcommand === 'help') {
      return {
        success: true,
        message:
          'Usage: /loop [interval] <prompt> | /loop cancel <id> | /loop list\n' +
          'Intervals: Ns (seconds), Nm (minutes), Nh (hours), Nd (days)\n' +
          'Examples:\n' +
          '  /loop 30s check health              (every 30 seconds)\n' +
          '  /loop 5m check deployment status    (every 5 minutes)\n' +
          '  /loop 2h check build                (every 2 hours)\n' +
          '  /loop check the build               (defaults to 10m)\n' +
          '  /loop check status every 2 hours\n' +
          '  /loop cancel a1b2c3d4\n' +
          '  /loop list',
      };
    }

    // All non-help commands require an active session
    if (!sessionId) {
      return {
        success: false,
        message: '✗ No active session — start a session first',
      };
    }

    // Lazy-import NAPI bindings (avoids circular deps and test issues)
    const napi = await import('@sengac/codelet-napi');

    if (parsed.subcommand === 'list') {
      const jsonStr = await napi.loopList(sessionId);
      const entries = JSON.parse(jsonStr) as Array<{
        id: string;
        prompt: string;
        intervalSeconds: number;
      }>;

      if (entries.length === 0) {
        return { success: true, message: 'No active loops.' };
      }

      const header = `${'ID'.padEnd(10)}${'Prompt'.padEnd(30)}${'Interval'.padEnd(15)}`;
      const separator = '-'.repeat(55);
      const rows = entries.map(entry => {
        const truncatedPrompt =
          entry.prompt.length > 28
            ? entry.prompt.slice(0, 25) + '...'
            : entry.prompt;
        return `${entry.id.padEnd(10)}${truncatedPrompt.padEnd(30)}${formatInterval(entry.intervalSeconds).padEnd(15)}`;
      });
      return {
        success: true,
        message: `Active loops:\n${header}\n${separator}\n${rows.join('\n')}`,
      };
    }

    if (parsed.subcommand === 'cancel') {
      const removed = await napi.loopCancel(parsed.jobId);
      if (!removed) {
        return {
          success: false,
          message: `✗ Loop "${parsed.jobId}" not found`,
        };
      }
      return {
        success: true,
        message: `✓ Cancelled loop ${parsed.jobId}`,
      };
    }

    // subcommand === 'add'
    const jobId = generateJobId();

    // Register with Rust scheduler — it will fire the prompt on schedule
    await napi.loopRegister(
      sessionId,
      jobId,
      parsed.prompt,
      parsed.intervalSeconds
    );

    const intervalStr = formatInterval(parsed.intervalSeconds);

    return {
      success: true,
      message: `✓ Scheduled every ${intervalStr} [job: ${jobId}]`,
    };
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    return { success: false, message: `✗ ${msg}` };
  }
}
