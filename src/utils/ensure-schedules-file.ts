/**
 * Ensure Schedules File - SCHED-002
 *
 * Creates spec/schedules.json if it doesn't exist with default structure.
 * Uses LockedFileManager for thread-safe file operations.
 */

import { join } from 'path';
import { existsSync } from 'fs';
import { mkdir } from 'fs/promises';
import { fileManager } from './file-manager';
import type { SchedulesData } from '../types/schedule';

const DEFAULT_SCHEDULES_DATA: SchedulesData = {
  version: '1.0.0',
  schedules: {},
};

/**
 * Ensures spec/schedules.json exists, creating it with default structure if missing.
 *
 * @param cwd - Project root directory
 * @returns The schedules data
 */
export async function ensureSchedulesFile(cwd: string): Promise<SchedulesData> {
  const specDir = join(cwd, 'spec');
  const schedulesFile = join(specDir, 'schedules.json');

  // Ensure spec directory exists
  if (!existsSync(specDir)) {
    await mkdir(specDir, { recursive: true });
  }

  // Use fileManager.readJSON which auto-creates if missing
  return await fileManager.readJSON<SchedulesData>(
    schedulesFile,
    DEFAULT_SCHEDULES_DATA
  );
}

/**
 * Gets the path to the schedules file for a project.
 *
 * @param cwd - Project root directory
 * @returns Path to spec/schedules.json
 */
export function getSchedulesFilePath(cwd: string): string {
  return join(cwd, 'spec', 'schedules.json');
}
