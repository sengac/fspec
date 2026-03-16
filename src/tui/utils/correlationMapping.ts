/**
 * Correlation Mapping Utilities (WATCH-011)
 *
 * Builds bi-directional maps between subordinate and supervisor turns based on correlation IDs.
 * Used by SplitSessionView for cross-pane highlighting.
 *
 * SOLID: Single Responsibility - Only handles correlation mapping logic
 * DRY: Extracted from SplitSessionView for reuse in tests
 */

import type { ConversationLine } from '../types/conversation';

/**
 * Result of building correlation maps
 */
export interface CorrelationMaps {
  /**
   * Map from subordinate messageIndex to Set of supervisor messageIndices that observed it
   * Used when user selects a subordinate turn to highlight correlated supervisor turns
   */
  subordinateToSupervisorTurns: Map<number, Set<number>>;

  /**
   * Map from supervisor messageIndex to Set of subordinate messageIndices it was observing
   * Used when user selects a supervisor turn to highlight correlated subordinate turns
   */
  supervisorToSubordinateTurns: Map<number, Set<number>>;
}

/**
 * Build correlation maps between subordinate and supervisor turns (WATCH-011)
 *
 * The correlation is established through:
 * - Subordinate chunks have `correlationId` assigned by handle_output()
 * - Supervisor response chunks have `observedCorrelationIds` listing subordinate IDs that triggered the response
 *
 * This function creates bidirectional maps for efficient lookup in either direction.
 *
 * @param subordinateConversation - Conversation lines from the subordinate session
 * @param supervisorConversation - Conversation lines from the supervisor session
 * @returns Maps for subordinate→supervisor and supervisor→subordinate turn correlation
 *
 * @example
 * const { subordinateToSupervisorTurns, supervisorToSubordinateTurns } = buildCorrelationMaps(subordinate, supervisor);
 *
 * // User selects subordinate turn 3, find correlated supervisor turns:
 * const supervisorTurns = subordinateToSupervisorTurns.get(3); // Set { 5, 6 }
 *
 * // User selects supervisor turn 5, find correlated subordinate turns:
 * const subordinateTurns = supervisorToSubordinateTurns.get(5); // Set { 2, 3 }
 */
export function buildCorrelationMaps(
  subordinateConversation: ConversationLine[],
  supervisorConversation: ConversationLine[]
): CorrelationMaps {
  const subordinateToSupervisorTurns = new Map<number, Set<number>>();
  const supervisorToSubordinateTurns = new Map<number, Set<number>>();

  // Build a map from subordinate correlationId to subordinate messageIndex
  // Multiple lines can share the same correlationId (same turn), but we only need
  // one entry per correlationId since we're mapping to messageIndex (turn level)
  const subordinateCorrelationToTurn = new Map<string, number>();
  for (const line of subordinateConversation) {
    if (line.correlationId && !line.isSeparator) {
      subordinateCorrelationToTurn.set(line.correlationId, line.messageIndex);
    }
  }

  // For each supervisor turn, find which subordinate turns it was observing
  // Supervisor lines with observedCorrelationIds were part of an observation response
  for (const line of supervisorConversation) {
    if (line.isSeparator || !line.observedCorrelationIds) continue;

    const supervisorTurn = line.messageIndex;
    for (const observedId of line.observedCorrelationIds) {
      const subordinateTurn = subordinateCorrelationToTurn.get(observedId);
      if (subordinateTurn !== undefined) {
        // Add to supervisorToSubordinateTurns (supervisor → subordinates it observed)
        if (!supervisorToSubordinateTurns.has(supervisorTurn)) {
          supervisorToSubordinateTurns.set(supervisorTurn, new Set());
        }
        supervisorToSubordinateTurns.get(supervisorTurn)!.add(subordinateTurn);

        // Add to subordinateToSupervisorTurns (subordinate → supervisors that observed it)
        if (!subordinateToSupervisorTurns.has(subordinateTurn)) {
          subordinateToSupervisorTurns.set(subordinateTurn, new Set());
        }
        subordinateToSupervisorTurns.get(subordinateTurn)!.add(supervisorTurn);
      }
    }
  }

  return { subordinateToSupervisorTurns, supervisorToSubordinateTurns };
}

/**
 * Get highlighted turns for cross-pane display (WATCH-011)
 *
 * Computes which turns in the inactive pane should be highlighted based on
 * the current selection in the active pane.
 *
 * @param activePane - Which pane is currently active
 * @param isSelectMode - Whether turn selection mode is active in either pane
 * @param selectedSubordinateTurn - Currently selected turn in subordinate pane (messageIndex)
 * @param selectedSupervisorTurn - Currently selected turn in supervisor pane (messageIndex)
 * @param correlationMaps - Maps from buildCorrelationMaps
 * @returns Set of messageIndices to highlight in the inactive pane
 */
export function getHighlightedTurns(
  activePane: 'subordinate' | 'supervisor',
  isSelectMode: boolean,
  selectedSubordinateTurn: number | null,
  selectedSupervisorTurn: number | null,
  correlationMaps: CorrelationMaps
): Set<number> {
  const highlighted = new Set<number>();

  if (!isSelectMode) {
    return highlighted;
  }

  if (activePane === 'subordinate' && selectedSubordinateTurn !== null) {
    // Subordinate pane is active - highlight correlated supervisor turns
    const supervisorTurns = correlationMaps.subordinateToSupervisorTurns.get(
      selectedSubordinateTurn
    );
    if (supervisorTurns) {
      supervisorTurns.forEach(t => highlighted.add(t));
    }
  } else if (activePane === 'supervisor' && selectedSupervisorTurn !== null) {
    // Supervisor pane is active - highlight correlated subordinate turns
    const subordinateTurns = correlationMaps.supervisorToSubordinateTurns.get(
      selectedSupervisorTurn
    );
    if (subordinateTurns) {
      subordinateTurns.forEach(t => highlighted.add(t));
    }
  }

  return highlighted;
}
