/**
 * Shared utilities for Event Storm artifact commands
 * Coverage: EXMAP-007
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { WorkUnitsData, EventStormItem } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddEventStormItemOptions<T extends EventStormItem> {
  workUnitId: string;
  itemData: Omit<T, 'id' | 'deleted' | 'createdAt'>;
  cwd?: string;
}

export interface AddEventStormItemResult {
  success: boolean;
  error?: string;
  itemId?: number;
}

/**
 * Generic helper for adding Event Storm items to work units
 *
 * @param options - Command options including work unit ID and item data
 * @returns Result with success status and item ID
 */
export async function addEventStormItem<T extends EventStormItem>(
  options: AddEventStormItemOptions<T>
): Promise<AddEventStormItemResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Check if work-units.json exists
  if (!existsSync(workUnitsFile)) {
    return {
      success: false,
      error: 'spec/work-units.json not found. Run fspec init first.',
    };
  }

  try {
    // Read work units data
    const workUnitsData = await fileManager.readJSON<WorkUnitsData>(
      workUnitsFile,
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {},
      }
    );

    // Validate work unit exists
    const workUnit = workUnitsData.workUnits[options.workUnitId];
    if (!workUnit) {
      return {
        success: false,
        error: `Work unit ${options.workUnitId} not found`,
      };
    }

    // Validate work unit is not in done/blocked state
    if (workUnit.status === 'done' || workUnit.status === 'blocked') {
      return {
        success: false,
        error: `Cannot add Event Storm items to work unit in ${workUnit.status} state`,
      };
    }

    // Add item to items array and increment ID using transaction
    let itemId: number | undefined;
    await fileManager.transaction(workUnitsFile, (data: WorkUnitsData) => {
      const wu = data.workUnits[options.workUnitId];
      if (!wu.eventStorm) {
        wu.eventStorm = {
          level: 'process_modeling',
          items: [],
          nextItemId: 0,
        };
      }

      itemId = wu.eventStorm.nextItemId;
      const item = {
        ...options.itemData,
        id: itemId,
        deleted: false,
        createdAt: new Date().toISOString(),
      } as T;

      wu.eventStorm.items.push(item);
      wu.eventStorm.nextItemId += 1;
    });

    return {
      success: true,
      itemId,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to add Event Storm item: ${errorMessage}`,
    };
  }
}
