/**
 * System reminder formatting for AI
 */

import type { HookExecutionResult } from './types.js';

export function formatHookOutput(result: HookExecutionResult, isBlocking: boolean): string {
  const parts: string[] = [];

  // Display stdout if present
  if (result.stdout) {
    parts.push(result.stdout);
  }

  // Non-blocking hook stderr is displayed as-is (no wrapping)
  if (!isBlocking && result.stderr) {
    parts.push(result.stderr);
  }

  // Blocking hook stderr is wrapped in <system-reminder> tags
  // Empty stderr produces no system-reminder (only if stderr has content)
  if (isBlocking && result.stderr && !result.success) {
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
    '',
    result.stderr,
    '</system-reminder>',
  ];

  return lines.join('\n');
}
