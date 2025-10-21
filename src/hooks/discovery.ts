/**
 * Hook discovery and event naming
 */

import type { HookConfig, HookDefinition } from './types.js';
import type { WorkUnit, VirtualHook } from '../types/index.js';

export function discoverHooks(
  config: HookConfig,
  eventName: string
): HookDefinition[] {
  // Search config.hooks for matching event name
  const hooks = config.hooks[eventName];

  // If no hooks match event name, return empty array (no error)
  if (!hooks) {
    return [];
  }

  // Return matching hooks in config order
  return hooks;
}

export function discoverVirtualHooks(
  workUnit: WorkUnit | null,
  eventName: string
): HookDefinition[] {
  // If no work unit or no virtual hooks, return empty array
  if (
    !workUnit ||
    !workUnit.virtualHooks ||
    workUnit.virtualHooks.length === 0
  ) {
    return [];
  }

  // Filter virtual hooks for matching event
  const matchingHooks = workUnit.virtualHooks.filter(
    (hook: VirtualHook) => hook.event === eventName
  );

  // Convert VirtualHook to HookDefinition format
  return matchingHooks.map((hook: VirtualHook) => ({
    name: hook.name,
    command: hook.command,
    blocking: hook.blocking,
  }));
}

export function generateEventNames(commandName: string): {
  pre: string;
  post: string;
} {
  // Event names follow pre-/post- convention: pre-<command-name>, post-<command-name>
  return {
    pre: `pre-${commandName}`,
    post: `post-${commandName}`,
  };
}
