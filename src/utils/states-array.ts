/**
 * Shared utility functions for manipulating work unit states arrays.
 *
 * Used by:
 * - update-work-unit-status.ts (sorted insertion into done column)
 * - fspecStore.ts (priority reordering with bracket keys)
 *
 * Architecture: BOARD-016
 * - States arrays determine display order in TUI
 * - Done column sorted by 'updated' timestamp (most recent first)
 * - All other columns maintain manual order
 */

import type { WorkUnitsData } from '../types';

/**
 * Move work unit within the same states array (swap positions).
 * Used for priority reordering with bracket keys.
 *
 * @param workUnitsData - The work units data containing states arrays
 * @param workUnitId - ID of work unit to move
 * @param statusKey - Column name (e.g., 'backlog', 'implementing')
 * @param direction - 'up' (towards index 0) or 'down' (towards end)
 * @returns Updated workUnitsData with modified states array, or original if no change
 */
export function moveWorkUnitInArray(
  workUnitsData: WorkUnitsData,
  workUnitId: string,
  statusKey: string,
  direction: 'up' | 'down'
): WorkUnitsData {
  const statesArray = workUnitsData.states[statusKey] || [];
  const currentIndex = statesArray.indexOf(workUnitId);

  // Cannot move if not found or already at boundary
  if (currentIndex < 0) {
    return workUnitsData;
  }

  if (direction === 'up' && currentIndex <= 0) {
    return workUnitsData; // Already at top
  }

  if (direction === 'down' && currentIndex >= statesArray.length - 1) {
    return workUnitsData; // Already at bottom
  }

  // Perform swap
  const newStatesArray = [...statesArray];
  const swapIndex = direction === 'up' ? currentIndex - 1 : currentIndex + 1;

  [newStatesArray[currentIndex], newStatesArray[swapIndex]] = [
    newStatesArray[swapIndex],
    newStatesArray[currentIndex],
  ];

  // Update states array
  return {
    ...workUnitsData,
    states: {
      ...workUnitsData.states,
      [statusKey]: newStatesArray,
    },
  };
}

/**
 * Insert work unit into states array at sorted position based on comparator.
 * Used for inserting into done column sorted by 'updated' timestamp.
 *
 * @param workUnitsData - The work units data containing states arrays
 * @param workUnitId - ID of work unit to insert
 * @param oldStatusKey - Previous column (to remove from)
 * @param newStatusKey - Target column (to insert into)
 * @param compareFn - Optional comparator function for sorting (a, b) => number
 *                    Negative = a before b, Positive = a after b, 0 = equal
 *                    If not provided, appends to end
 * @returns Updated workUnitsData with work unit moved to new position
 */
export function insertWorkUnitSorted(
  workUnitsData: WorkUnitsData,
  workUnitId: string,
  oldStatusKey: string,
  newStatusKey: string,
  compareFn?: (
    aId: string,
    bId: string,
    workUnits: Record<string, any>
  ) => number
): WorkUnitsData {
  // BUG-064 FIX: Remove work unit ID from ALL state arrays first
  // This prevents duplicates when moving backward or to the same state
  const cleanedStates: Record<string, string[]> = {};
  for (const [stateKey, stateArray] of Object.entries(workUnitsData.states)) {
    cleanedStates[stateKey] = stateArray.filter(id => id !== workUnitId);
  }

  // Get target states array (after cleaning)
  const targetStatesArray = cleanedStates[newStatusKey] || [];

  let newTargetStatesArray: string[];

  if (!compareFn) {
    // No comparator - append to end
    newTargetStatesArray = [...targetStatesArray, workUnitId];
  } else {
    // Find insertion position using comparator
    let insertIndex = targetStatesArray.length; // Default to end

    for (let i = 0; i < targetStatesArray.length; i++) {
      const existingId = targetStatesArray[i];
      const compareResult = compareFn(
        workUnitId,
        existingId,
        workUnitsData.workUnits
      );

      if (compareResult < 0) {
        // workUnitId should come before existingId
        insertIndex = i;
        break;
      }
    }

    // Insert at calculated position
    newTargetStatesArray = [
      ...targetStatesArray.slice(0, insertIndex),
      workUnitId,
      ...targetStatesArray.slice(insertIndex),
    ];
  }

  // Return updated data with cleaned states
  return {
    ...workUnitsData,
    states: {
      ...cleanedStates,
      [newStatusKey]: newTargetStatesArray,
    },
  };
}

/**
 * Comparator function for sorting by 'updated' timestamp (most recent first).
 * Used when inserting work units into done column.
 *
 * @param aId - First work unit ID
 * @param bId - Second work unit ID
 * @param workUnits - Work units lookup object
 * @returns Negative if a is more recent, positive if b is more recent, 0 if equal
 */
export function compareByUpdatedDescending(
  aId: string,
  bId: string,
  workUnits: Record<string, any>
): number {
  const aWorkUnit = workUnits[aId];
  const bWorkUnit = workUnits[bId];

  // BOARD-016: Use 'updated' if present, otherwise fall back to 'createdAt'
  const aTimestamp = aWorkUnit?.updated || aWorkUnit?.createdAt;
  const bTimestamp = bWorkUnit?.updated || bWorkUnit?.createdAt;

  // Handle missing timestamps (should not happen, but be defensive)
  if (!aTimestamp && !bTimestamp) {
    return 0;
  }
  if (!aTimestamp) {
    return 1; // a goes after b
  }
  if (!bTimestamp) {
    return -1; // a goes before b
  }

  // Parse timestamps and compare (descending = most recent first)
  const aTime = new Date(aTimestamp).getTime();
  const bTime = new Date(bTimestamp).getTime();

  return bTime - aTime; // Descending order (most recent first)
}
