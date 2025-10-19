import { stat } from 'fs/promises';
import { glob } from 'tinyglobby';
import type { WorkUnit } from '../types';

/**
 * Find when a work unit entered a specific state
 */
export function findStateHistoryEntry(
  workUnit: WorkUnit,
  targetState: string
): { state: string; timestamp: string; reason?: string } | null {
  if (!workUnit.stateHistory || workUnit.stateHistory.length === 0) {
    return null;
  }

  // Find the FIRST time the work unit entered this state
  // (in case it moved backward and forward multiple times)
  const entry = workUnit.stateHistory.find(h => h.state === targetState);
  return entry || null;
}

/**
 * Check if files related to a work unit were created/modified AFTER a timestamp
 * @param workUnitId - Work unit ID to check
 * @param afterTimestamp - ISO timestamp - files must be modified after this
 * @param fileType - 'feature' or 'test'
 * @param cwd - Current working directory
 * @throws Error if files exist but were modified BEFORE the timestamp
 */
export async function checkFileCreatedAfter(
  workUnitId: string,
  afterTimestamp: string,
  fileType: 'feature' | 'test',
  cwd: string
): Promise<void> {
  const files = await findWorkUnitFiles(workUnitId, fileType, cwd);

  if (files.length === 0) {
    // No files found - this is OK, other validation will catch if they're required
    return;
  }

  const afterDate = new Date(afterTimestamp);
  const violations: Array<{ file: string; fileTime: Date; stateName: string }> =
    [];

  for (const file of files) {
    try {
      const fileStat = await stat(file);
      const fileModTime = fileStat.mtime;

      // If file was modified BEFORE entering the state, that's a violation
      if (fileModTime < afterDate) {
        const stateName = fileType === 'feature' ? 'specifying' : 'testing';
        violations.push({ file, fileTime: fileModTime, stateName });
      }
    } catch (error) {
      // File doesn't exist - ignore, other validation will catch
      continue;
    }
  }

  if (violations.length > 0) {
    const violationDetails = violations
      .map(v => {
        const fileTimeStr = v.fileTime.toISOString();
        const stateTimeStr = afterTimestamp;
        return `  - ${v.file}\n    File modified: ${fileTimeStr}\n    Entered ${v.stateName}: ${stateTimeStr}\n    Gap: ${Math.round((afterDate.getTime() - v.fileTime.getTime()) / 1000 / 60)} minutes BEFORE state entry`;
      })
      .join('\n\n');

    const stateName = fileType === 'feature' ? 'specifying' : 'testing';

    throw new Error(
      `ACDD temporal ordering violation detected!\n\n` +
        `${fileType === 'feature' ? 'Feature files' : 'Test files'} were created/modified BEFORE entering ${stateName} state.\n` +
        `This indicates retroactive completion (doing work first, then walking through states as theater).\n\n` +
        `Violations:\n${violationDetails}\n\n` +
        `ACDD requires work to be done IN each state, not BEFORE entering it:\n` +
        `  - ${fileType === 'feature' ? 'Feature files must be created AFTER entering specifying state' : 'Test files must be created AFTER entering testing state'}\n` +
        `  - Timestamps prove when work was actually done\n\n` +
        `To fix:\n` +
        `  1. If this is reverse ACDD or importing existing work: Use --skip-temporal-validation flag\n` +
        `  2. If this is a mistake: Delete ${workUnitId} and restart with proper ACDD workflow\n` +
        `  3. If recovering from error: Move work unit back to ${stateName} state and update files\n\n` +
        `For more info: See FEAT-011 "Prevent retroactive state walking"`
    );
  }
}

/**
 * Find files associated with a work unit
 * @param workUnitId - Work unit ID
 * @param fileType - 'feature' or 'test'
 * @param cwd - Current working directory
 * @returns Array of absolute file paths
 */
export async function findWorkUnitFiles(
  workUnitId: string,
  fileType: 'feature' | 'test',
  cwd: string
): Promise<string[]> {
  if (fileType === 'feature') {
    // Find feature files tagged with @WORK-UNIT-ID
    const featureFiles = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: true,
    });

    const matchingFiles: string[] = [];
    const { readFile } = await import('fs/promises');

    for (const file of featureFiles) {
      try {
        const content = await readFile(file, 'utf-8');
        if (content.includes(`@${workUnitId}`)) {
          matchingFiles.push(file);
        }
      } catch {
        // Ignore unreadable files
        continue;
      }
    }

    return matchingFiles;
  } else {
    // Find test files that reference the work unit ID or feature files
    // Test files typically live in src/**/__tests__/**/*.test.ts
    const testFiles = await glob(['src/**/__tests__/**/*.test.ts'], {
      cwd,
      absolute: true,
    });

    const matchingFiles: string[] = [];
    const { readFile } = await import('fs/promises');

    for (const file of testFiles) {
      try {
        const content = await readFile(file, 'utf-8');
        // Check if test file mentions the work unit ID in comments or describes
        if (
          content.includes(workUnitId) ||
          content.includes(`@${workUnitId}`)
        ) {
          matchingFiles.push(file);
        }
      } catch {
        // Ignore unreadable files
        continue;
      }
    }

    return matchingFiles;
  }
}
