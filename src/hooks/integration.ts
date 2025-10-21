/**
 * Hook integration with commands
 */

import { loadHookConfig } from './config.js';
import {
  discoverHooks,
  discoverVirtualHooks,
  generateEventNames,
} from './discovery.js';
import { executeHooks } from './executor.js';
import { evaluateHookCondition } from './conditions.js';
import { formatHookOutput } from './formatting.js';
import { getGitContext } from './git-context.js';
import type { HookContext, HookExecutionResult } from './types.js';
import type { WorkUnit } from '../types/work-units.js';
import { readFile } from 'fs/promises';
import { join } from 'path';

export interface CommandContext {
  workUnitId?: string;
  [key: string]: unknown;
}

export interface CommandHookResult {
  preHookResults: HookExecutionResult[];
  postHookResults: HookExecutionResult[];
  commandExecuted: boolean;
  exitCode: number;
  output: string;
  commandResult?: unknown;
}

/**
 * Run a command with pre and post hooks
 */
export async function runCommandWithHooks<T>(
  commandName: string,
  context: CommandContext,
  commandFn: (context: CommandContext) => Promise<T>,
  projectRoot: string = process.cwd()
): Promise<CommandHookResult> {
  const output: string[] = [];
  let commandExecuted = false;
  let commandResult: T | undefined;
  let exitCode = 0;

  // Generate event names
  const { pre: preEvent, post: postEvent } = generateEventNames(commandName);

  // Create hook context
  const hookContext: HookContext = {
    workUnitId: context.workUnitId,
    event: preEvent,
    timestamp: new Date().toISOString(),
  };

  // Load hook configuration (optional - virtual hooks work without it)
  let config;
  try {
    config = await loadHookConfig(projectRoot);
  } catch {
    // No global hooks configured - use empty config
    config = { hooks: {} };
  }

  // Load work unit if workUnitId is provided
  let workUnit: WorkUnit | null = null;
  if (context.workUnitId) {
    try {
      const workUnitsPath = join(projectRoot, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnitsData = JSON.parse(workUnitsContent);
      workUnit = workUnitsData.workUnits[context.workUnitId] || null;
    } catch {
      // Work unit not found, continue without it
    }
  }

  // Discover virtual hooks (run BEFORE global hooks)
  const virtualPreHooks = discoverVirtualHooks(workUnit, preEvent);

  // Discover and filter global pre-hooks
  const globalPreHooks = discoverHooks(config, preEvent).filter(hook =>
    evaluateHookCondition(hook, hookContext, workUnit)
  );

  // Combine: virtual hooks FIRST, then global hooks
  const preHooks = [...virtualPreHooks, ...globalPreHooks];

  // Check if any hooks need git context
  const needsGitContext = preHooks.some(hook => {
    // VirtualHook type has gitContext field
    return 'gitContext' in hook && hook.gitContext === true;
  });

  // Add git context if needed
  if (needsGitContext) {
    const gitContext = await getGitContext(projectRoot);
    hookContext.stagedFiles = gitContext.stagedFiles;
    hookContext.unstagedFiles = gitContext.unstagedFiles;
  }

  // Execute pre-hooks
  hookContext.event = preEvent;
  const preHookResults = await executeHooks(preHooks, hookContext, projectRoot);

  // Check if any blocking pre-hook failed
  const blockingPreHookFailed = preHookResults.some(
    (result, index) => preHooks[index].blocking && !result.success
  );

  if (blockingPreHookFailed) {
    // Format and display blocking hook failures
    preHookResults.forEach((result, index) => {
      if (preHooks[index].blocking && !result.success) {
        const formatted = formatHookOutput(
          result,
          preHooks[index].blocking ?? false
        );
        output.push(formatted);
      }
    });

    return {
      preHookResults,
      postHookResults: [],
      commandExecuted: false,
      exitCode: 1,
      output: output.join('\n'),
    };
  }

  // Display non-blocking pre-hook output
  preHookResults.forEach((result, index) => {
    if (!preHooks[index].blocking) {
      const formatted = formatHookOutput(result, false);
      if (formatted) {
        output.push(formatted);
      }
    }
  });

  // Execute command
  commandResult = await commandFn(context);
  commandExecuted = true;

  // Discover virtual post-hooks (run BEFORE global hooks)
  const virtualPostHooks = discoverVirtualHooks(workUnit, postEvent);

  // Discover and filter global post-hooks
  const globalPostHooks = discoverHooks(config, postEvent).filter(hook =>
    evaluateHookCondition(hook, hookContext, workUnit)
  );

  // Combine: virtual hooks FIRST, then global hooks
  const postHooks = [...virtualPostHooks, ...globalPostHooks];

  // Check if any post-hooks need git context
  const needsGitContextPost = postHooks.some(hook => {
    return 'gitContext' in hook && hook.gitContext === true;
  });

  // Add git context if needed (refresh for post-hooks)
  if (needsGitContextPost) {
    const gitContext = await getGitContext(projectRoot);
    hookContext.stagedFiles = gitContext.stagedFiles;
    hookContext.unstagedFiles = gitContext.unstagedFiles;
  }

  // Execute post-hooks
  hookContext.event = postEvent;
  const postHookResults = await executeHooks(
    postHooks,
    hookContext,
    projectRoot
  );

  // Check if any blocking post-hook failed
  const blockingPostHookFailed = postHookResults.some(
    (result, index) => postHooks[index].blocking && !result.success
  );

  // Format and display hook output
  postHookResults.forEach((result, index) => {
    const formatted = formatHookOutput(
      result,
      postHooks[index].blocking ?? false
    );
    if (formatted) {
      output.push(formatted);
    }
  });

  // Set exit code based on blocking post-hook failures
  if (blockingPostHookFailed) {
    exitCode = 1;
  }

  return {
    preHookResults,
    postHookResults,
    commandExecuted,
    exitCode,
    output: output.join('\n'),
    commandResult,
  };
}
