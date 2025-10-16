/**
 * Hook execution engine
 */

import { execa } from 'execa';
import { join } from 'node:path';
import type { HookDefinition, HookContext, HookExecutionResult } from './types.js';

export async function executeHook(
  hook: HookDefinition,
  context: HookContext,
  projectRoot: string
): Promise<HookExecutionResult> {
  const startTime = Date.now();
  const commandPath = join(projectRoot, hook.command);
  const timeout = hook.timeout ?? 60;

  // Create AbortController for custom timeout implementation
  const controller = new AbortController();
  let timedOut = false;
  let timeoutId: NodeJS.Timeout | null = null;

  // Set up timeout with AbortController
  timeoutId = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeout * 1000);

  try {
    // Prepare context JSON for stdin
    const contextJson = JSON.stringify(context);

    // Execute hook using execa with input option for stdin
    const subprocess = execa(commandPath, [], {
      cwd: projectRoot,
      env: process.env,
      input: contextJson + '\n',
      cancelSignal: controller.signal,
      all: true, // Capture interleaved stdout/stderr
    });

    const result = await subprocess;

    // Clear timeout on successful completion
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    const duration = Date.now() - startTime;
    const exitCode = timedOut ? null : result.exitCode;
    const success = !timedOut && exitCode === 0;

    return {
      hookName: hook.name,
      success,
      exitCode,
      stdout: result.stdout,
      stderr: result.stderr,
      timedOut,
      duration,
    };
  } catch (error: any) {
    // Clear timeout
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    const duration = Date.now() - startTime;

    // Handle timeout/abort case
    if (error.isCanceled || error.killed || timedOut) {
      return {
        hookName: hook.name,
        success: false,
        exitCode: null,
        stdout: error.stdout || '',
        stderr: error.stderr || '',
        timedOut: true,
        duration,
      };
    }

    // Handle non-zero exit codes (execa throws on non-zero)
    if (error.exitCode !== undefined) {
      return {
        hookName: hook.name,
        success: false,
        exitCode: error.exitCode,
        stdout: error.stdout || '',
        stderr: error.stderr || '',
        timedOut: false,
        duration,
      };
    }

    // Handle other execution errors (ENOENT, permission denied, etc.)
    return {
      hookName: hook.name,
      success: false,
      exitCode: null,
      stdout: error.stdout || '',
      stderr: (error.stderr || '') + '\n' + error.message,
      timedOut: false,
      duration,
    };
  }
}

export async function executeHooks(
  hooks: HookDefinition[],
  context: HookContext,
  projectRoot: string
): Promise<HookExecutionResult[]> {
  const results: HookExecutionResult[] = [];

  // Execute hooks sequentially
  for (const hook of hooks) {
    const result = await executeHook(hook, context, projectRoot);
    results.push(result);
  }

  return results;
}
