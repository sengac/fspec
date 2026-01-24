/**
 * useWatcherHeaderInfo - Hook to get watcher info for session header
 *
 * Computes watcher slug and instance number from session ID.
 * Returns null if session is not a watcher.
 *
 * SOLID: Single Responsibility - only watcher header info computation
 * DRY: Reusable across any component that needs watcher info
 */

import { useMemo } from 'react';
import {
  sessionGetParent,
  sessionGetRole,
  sessionGetWatchers,
} from '@sengac/codelet-napi';
import { generateSlug } from '../utils/watcherTemplateStorage';

export interface WatcherHeaderInfo {
  /** Template slug (e.g., "security-reviewer") */
  slug: string;
  /** Instance number (1-based) */
  instanceNumber: number;
  /** Role name (e.g., "Security Reviewer") */
  roleName: string;
  /** Parent session ID */
  parentId: string;
}

/**
 * Get watcher header info for a session
 * @param sessionId - The current session ID
 * @returns WatcherHeaderInfo if session is a watcher, null otherwise
 */
export function useWatcherHeaderInfo(
  sessionId: string | null
): WatcherHeaderInfo | null {
  return useMemo(() => {
    if (!sessionId) return null;

    try {
      const parentId = sessionGetParent(sessionId);
      if (!parentId) return null;

      // This is a watcher session - get role info
      const role = sessionGetRole(sessionId);
      const roleName = role?.name || 'Watcher';
      const slug = generateSlug(roleName);

      // Calculate instance number by counting watchers with same slug
      const allWatchers = sessionGetWatchers(parentId);
      let instanceNumber = 1;
      for (const watcherId of allWatchers) {
        if (watcherId === sessionId) break;
        const watcherRole = sessionGetRole(watcherId);
        if (watcherRole && generateSlug(watcherRole.name) === slug) {
          instanceNumber++;
        }
      }

      return {
        slug,
        instanceNumber,
        roleName,
        parentId,
      };
    } catch {
      // Error checking parent - not a watcher
      return null;
    }
  }, [sessionId]);
}
