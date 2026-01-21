/**
 * Correlation Mapping Utilities (WATCH-011)
 *
 * Builds bi-directional maps between parent and watcher turns based on correlation IDs.
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
   * Map from parent messageIndex to Set of watcher messageIndices that observed it
   * Used when user selects a parent turn to highlight correlated watcher turns
   */
  parentToWatcherTurns: Map<number, Set<number>>;

  /**
   * Map from watcher messageIndex to Set of parent messageIndices it was observing
   * Used when user selects a watcher turn to highlight correlated parent turns
   */
  watcherToParentTurns: Map<number, Set<number>>;
}

/**
 * Build correlation maps between parent and watcher turns (WATCH-011)
 *
 * The correlation is established through:
 * - Parent chunks have `correlationId` assigned by handle_output()
 * - Watcher response chunks have `observedCorrelationIds` listing parent IDs that triggered the response
 *
 * This function creates bidirectional maps for efficient lookup in either direction.
 *
 * @param parentConversation - Conversation lines from the parent session
 * @param watcherConversation - Conversation lines from the watcher session
 * @returns Maps for parent→watcher and watcher→parent turn correlation
 *
 * @example
 * const { parentToWatcherTurns, watcherToParentTurns } = buildCorrelationMaps(parent, watcher);
 *
 * // User selects parent turn 3, find correlated watcher turns:
 * const watcherTurns = parentToWatcherTurns.get(3); // Set { 5, 6 }
 *
 * // User selects watcher turn 5, find correlated parent turns:
 * const parentTurns = watcherToParentTurns.get(5); // Set { 2, 3 }
 */
export function buildCorrelationMaps(
  parentConversation: ConversationLine[],
  watcherConversation: ConversationLine[]
): CorrelationMaps {
  const parentToWatcherTurns = new Map<number, Set<number>>();
  const watcherToParentTurns = new Map<number, Set<number>>();

  // Build a map from parent correlationId to parent messageIndex
  // Multiple lines can share the same correlationId (same turn), but we only need
  // one entry per correlationId since we're mapping to messageIndex (turn level)
  const parentCorrelationToTurn = new Map<string, number>();
  for (const line of parentConversation) {
    if (line.correlationId && !line.isSeparator) {
      parentCorrelationToTurn.set(line.correlationId, line.messageIndex);
    }
  }

  // For each watcher turn, find which parent turns it was observing
  // Watcher lines with observedCorrelationIds were part of an observation response
  for (const line of watcherConversation) {
    if (line.isSeparator || !line.observedCorrelationIds) continue;

    const watcherTurn = line.messageIndex;
    for (const observedId of line.observedCorrelationIds) {
      const parentTurn = parentCorrelationToTurn.get(observedId);
      if (parentTurn !== undefined) {
        // Add to watcherToParentTurns (watcher → parents it observed)
        if (!watcherToParentTurns.has(watcherTurn)) {
          watcherToParentTurns.set(watcherTurn, new Set());
        }
        watcherToParentTurns.get(watcherTurn)!.add(parentTurn);

        // Add to parentToWatcherTurns (parent → watchers that observed it)
        if (!parentToWatcherTurns.has(parentTurn)) {
          parentToWatcherTurns.set(parentTurn, new Set());
        }
        parentToWatcherTurns.get(parentTurn)!.add(watcherTurn);
      }
    }
  }

  return { parentToWatcherTurns, watcherToParentTurns };
}

/**
 * Get highlighted turns for cross-pane display (WATCH-011)
 *
 * Computes which turns in the inactive pane should be highlighted based on
 * the current selection in the active pane.
 *
 * @param activePane - Which pane is currently active
 * @param isSelectMode - Whether turn selection mode is active in either pane
 * @param selectedParentTurn - Currently selected turn in parent pane (messageIndex)
 * @param selectedWatcherTurn - Currently selected turn in watcher pane (messageIndex)
 * @param correlationMaps - Maps from buildCorrelationMaps
 * @returns Set of messageIndices to highlight in the inactive pane
 */
export function getHighlightedTurns(
  activePane: 'parent' | 'watcher',
  isSelectMode: boolean,
  selectedParentTurn: number | null,
  selectedWatcherTurn: number | null,
  correlationMaps: CorrelationMaps
): Set<number> {
  const highlighted = new Set<number>();

  if (!isSelectMode) {
    return highlighted;
  }

  if (activePane === 'parent' && selectedParentTurn !== null) {
    // Parent pane is active - highlight correlated watcher turns
    const watcherTurns =
      correlationMaps.parentToWatcherTurns.get(selectedParentTurn);
    if (watcherTurns) {
      watcherTurns.forEach(t => highlighted.add(t));
    }
  } else if (activePane === 'watcher' && selectedWatcherTurn !== null) {
    // Watcher pane is active - highlight correlated parent turns
    const parentTurns =
      correlationMaps.watcherToParentTurns.get(selectedWatcherTurn);
    if (parentTurns) {
      parentTurns.forEach(t => highlighted.add(t));
    }
  }

  return highlighted;
}
