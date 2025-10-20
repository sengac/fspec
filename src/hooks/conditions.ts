/**
 * Hook condition evaluation
 */

import type { HookDefinition, HookContext } from './types.js';
import type { WorkUnit } from '../types/index.js';

export function evaluateHookCondition(
  hook: HookDefinition,
  context: HookContext,
  workUnit: WorkUnit | null
): boolean {
  // Hooks with no condition always match (condition is optional)
  if (!hook.condition) {
    return true;
  }

  // If context has no workUnitId, only hooks without conditions run
  if (!context.workUnitId || !workUnit) {
    return false;
  }

  // Multiple condition fields use AND logic (all must match)

  // Condition with tags: hook runs if work unit has ANY of the specified tags (OR logic)
  if (hook.condition.tags && hook.condition.tags.length > 0) {
    const workUnitTags = workUnit.tags || [];
    const hasMatchingTag = hook.condition.tags.some(conditionTag =>
      workUnitTags.includes(conditionTag)
    );
    if (!hasMatchingTag) {
      return false;
    }
  }

  // Condition with prefix: hook runs if work unit ID starts with ANY of the specified prefixes (OR logic)
  if (hook.condition.prefix && hook.condition.prefix.length > 0) {
    const hasMatchingPrefix = hook.condition.prefix.some(prefix =>
      workUnit.id.startsWith(prefix)
    );
    if (!hasMatchingPrefix) {
      return false;
    }
  }

  // Condition with epic: hook runs if work unit belongs to specified epic
  if (hook.condition.epic) {
    if (workUnit.epic !== hook.condition.epic) {
      return false;
    }
  }

  // Condition with estimateMin/estimateMax: hook runs if work unit estimate is within range
  if (
    hook.condition.estimateMin !== undefined ||
    hook.condition.estimateMax !== undefined
  ) {
    const estimate = workUnit.estimate;
    if (estimate === undefined) {
      return false;
    }

    if (
      hook.condition.estimateMin !== undefined &&
      estimate < hook.condition.estimateMin
    ) {
      return false;
    }

    if (
      hook.condition.estimateMax !== undefined &&
      estimate > hook.condition.estimateMax
    ) {
      return false;
    }
  }

  // All conditions matched
  return true;
}
