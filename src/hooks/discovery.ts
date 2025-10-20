/**
 * Hook discovery and event naming
 */

import type { HookConfig, HookDefinition } from './types.js';

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
