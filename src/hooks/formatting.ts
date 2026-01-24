/**
 * System reminder formatting for AI
 */

import type { HookExecutionResult } from './types';

export function formatHookOutput(
  result: HookExecutionResult,
  isBlocking: boolean
): string {
  const parts: string[] = [];

  // Display stdout if present
  if (result.stdout) {
    parts.push(result.stdout);
  }

  // Non-blocking hook stderr is displayed as-is (no wrapping)
  if (!isBlocking && result.stderr) {
    parts.push(result.stderr);
  }

  // Blocking hook failures are ALWAYS wrapped in <system-reminder> tags
  // This makes failures highly visible to AI, even if no stderr
  if (isBlocking && !result.success) {
    const systemReminder = formatSystemReminder(result);
    parts.push(systemReminder);
  }

  return parts.join('\n');
}

function formatSystemReminder(result: HookExecutionResult): string {
  // System-reminder content includes hook name, exit code, and stderr output
  const lines = [
    '<system-reminder>',
    `Hook: ${result.hookName}`,
    `Exit code: ${result.exitCode}`,
  ];

  // Add stderr if present, otherwise add generic failure message
  if (result.stderr) {
    lines.push('', result.stderr);
  } else {
    lines.push('', '(Hook failed with no error output)');
  }

  lines.push('</system-reminder>');

  return lines.join('\n');
}
