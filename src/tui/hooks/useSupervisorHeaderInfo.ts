/**
 * useSupervisorHeaderInfo - Hook to get supervisor info for session header
 *
 * Computes supervisor slug and instance number from session ID.
 * Returns null if session is not a supervisor.
 *
 * AMGR-008: Simplified after removing supervisor template infrastructure.
 * Uses role string directly instead of SupervisorRole struct.
 */

import { useMemo } from 'react';
import {
  sessionGetSubordinate,
  sessionGetRole,
  sessionGetSupervisors,
} from '@sengac/codelet-napi';

export interface SupervisorHeaderInfo {
  /** Template slug (e.g., "security-reviewer") */
  slug: string;
  /** Instance number (1-based) */
  instanceNumber: number;
  /** Role name (e.g., "Security Reviewer") */
  roleName: string;
  /** Subordinate session ID */
  subordinateId: string;
}

/**
 * Generate a URL-friendly slug from a name string
 */
function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Get supervisor header info for a session
 * @param sessionId - The current session ID
 * @returns SupervisorHeaderInfo if session is a supervisor, null otherwise
 */
export function useSupervisorHeaderInfo(
  sessionId: string | null
): SupervisorHeaderInfo | null {
  return useMemo(() => {
    if (!sessionId) return null;

    try {
      const subordinateId = sessionGetSubordinate(sessionId);
      if (!subordinateId) return null;

      // This is a supervisor session - get role info
      const role = sessionGetRole(sessionId);
      const roleName = role?.name || 'Supervisor';
      const slug = generateSlug(roleName);

      // Calculate instance number by counting supervisors with same slug
      const allSupervisors = sessionGetSupervisors(subordinateId);
      let instanceNumber = 1;
      for (const supervisorId of allSupervisors) {
        if (supervisorId === sessionId) break;
        const supervisorRole = sessionGetRole(supervisorId);
        if (supervisorRole && generateSlug(supervisorRole.name) === slug) {
          instanceNumber++;
        }
      }

      return {
        slug,
        instanceNumber,
        roleName,
        subordinateId,
      };
    } catch {
      // Error checking subordinate - not a supervisor
      return null;
    }
  }, [sessionId]);
}
