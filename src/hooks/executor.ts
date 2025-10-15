/**
 * Hook execution engine
 */

import { spawn } from 'node:child_process';
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

  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    let timeoutId: NodeJS.Timeout | null = null;

    const child = spawn(commandPath, [], {
      cwd: projectRoot,
      env: process.env,
    });

    // Set up timeout
    timeoutId = setTimeout(() => {
      timedOut = true;
      child.kill('SIGTERM');
      // Force kill if still running after 1s
      setTimeout(() => {
        if (!child.killed) {
          child.kill('SIGKILL');
        }
      }, 1000);
    }, timeout * 1000);

    // Pass context to stdin (ignore errors if process dies before reading)
    try {
      const contextJson = JSON.stringify(context);
      child.stdin.write(contextJson + '\n');
      child.stdin.end();
    } catch (err) {
      // Ignore stdin write errors
    }

    // Ignore stdin errors (process may die before reading)
    child.stdin.on('error', () => {
      // Ignore
    });

    // Capture stdout
    child.stdout.on('data', (data: Buffer) => {
      stdout += data.toString();
    });

    // Capture stderr
    child.stderr.on('data', (data: Buffer) => {
      stderr += data.toString();
    });

    // Handle process exit
    child.on('close', (code: number | null) => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }

      const duration = Date.now() - startTime;
      const exitCode = timedOut ? null : code;
      const success = !timedOut && exitCode === 0;

      resolve({
        hookName: hook.name,
        success,
        exitCode,
        stdout,
        stderr,
        timedOut,
        duration,
      });
    });

    // Handle process error
    child.on('error', (err: Error) => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }

      resolve({
        hookName: hook.name,
        success: false,
        exitCode: null,
        stdout,
        stderr: stderr + '\n' + err.message,
        timedOut: false,
        duration: Date.now() - startTime,
      });
    });
  });
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
