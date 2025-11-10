/**
 * Hook execution engine
 */

import { execa } from 'execa';
import { join } from 'path';
import type {
  HookDefinition,
  HookContext,
  HookExecutionResult,
} from './types.js';
import { isShellCommand } from './command-utils.js';

export async function executeHook(
  hook: HookDefinition,
  context: HookContext,
  projectRoot: string
): Promise<HookExecutionResult> {
  const startTime = Date.now();
  const commandPath = join(projectRoot, hook.command);
  const timeout = hook.timeout ?? 60;

  // Create AbortController for custom timeout implementation
  const controller = new globalThis.AbortController();
  let timedOut = false;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  // Set up timeout with AbortController
  timeoutId = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeout * 1000);

  try {
    // Prepare context JSON for stdin
    const contextJson = JSON.stringify(context);

    // Determine if this is a shell command or script path
    const isShell = await isShellCommand(hook.command, projectRoot);

    // Execute hook using execa
    let subprocess;
    if (isShell) {
      // Execute as shell command via sh -c
      subprocess = execa('sh', ['-c', hook.command], {
        cwd: projectRoot,
        env: process.env,
        input: contextJson + '\n',
        cancelSignal: controller.signal,
        all: true, // Capture interleaved stdout/stderr
      });
    } else {
      // Execute as script path
      subprocess = execa(commandPath, [], {
        cwd: projectRoot,
        env: process.env,
        input: contextJson + '\n',
        cancelSignal: controller.signal,
        all: true, // Capture interleaved stdout/stderr
      });
    }

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
  } catch (error: unknown) {
    // Clear timeout
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    const duration = Date.now() - startTime;

    // Type guard for execa error
    const execaError = error as {
      isCanceled?: boolean;
      killed?: boolean;
      stdout?: string;
      stderr?: string;
      exitCode?: number;
      message?: string;
    };

    // Handle timeout/abort case
    if (execaError.isCanceled || execaError.killed || timedOut) {
      return {
        hookName: hook.name,
        success: false,
        exitCode: null,
        stdout: execaError.stdout || '',
        stderr: execaError.stderr || '',
        timedOut: true,
        duration,
      };
    }

    // Handle non-zero exit codes (execa throws on non-zero)
    if (execaError.exitCode !== undefined) {
      return {
        hookName: hook.name,
        success: false,
        exitCode: execaError.exitCode,
        stdout: execaError.stdout || '',
        stderr: execaError.stderr || '',
        timedOut: false,
        duration,
      };
    }

    // Handle other execution errors (ENOENT, permission denied, etc.)
    return {
      hookName: hook.name,
      success: false,
      exitCode: null,
      stdout: execaError.stdout || '',
      stderr:
        (execaError.stderr || '') +
        '\n' +
        (execaError.message || 'Unknown error'),
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
