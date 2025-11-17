/**
 * LockedFileManager - Three-layer file locking architecture
 *
 * Layer 1: Inter-process coordination via proper-lockfile
 * Layer 2: In-process readers-writer pattern
 * Layer 3: Atomic write-replace pattern
 *
 * Prevents JSON file corruption when multiple fspec instances run concurrently.
 * Coverage: LOCK-001
 */

import { readFile, writeFile, rename, unlink } from 'fs/promises';
import { existsSync } from 'fs';
import lockfile from 'proper-lockfile';
import { randomUUID } from 'crypto';
import { logger } from './logger';

interface LockMetrics {
  lockType: 'READ' | 'WRITE';
  filePath: string;
  waitTimeMs: number;
  holdDurationMs: number;
  retries: number;
}

/**
 * LockedFileManager - Singleton for coordinating file access with three-layer locking
 */
class LockedFileManager {
  private static instance: LockedFileManager;

  // Layer 2: In-process locks (readers-writer pattern)
  private readCounts: Map<string, number> = new Map(); // file → number of active readers
  private writeLocks: Set<string> = new Set(); // files with active write locks
  private waitingReaders: Map<string, Array<() => void>> = new Map(); // queued readers
  private waitingWriters: Map<string, Array<() => void>> = new Map(); // queued writers

  private constructor() {}

  public static getInstance(): LockedFileManager {
    if (!LockedFileManager.instance) {
      LockedFileManager.instance = new LockedFileManager();
    }
    return LockedFileManager.instance;
  }

  /**
   * Reset internal lock state (for testing only)
   * CRITICAL: Only call this when no file operations are in progress
   */
  public resetLockState(): void {
    this.readCounts.clear();
    this.writeLocks.clear();
    this.waitingReaders.clear();
    this.waitingWriters.clear();
  }

  /**
   * Acquire in-process read lock (multiple readers allowed)
   */
  private async acquireReadLock(filePath: string): Promise<void> {
    // If write lock active, wait
    if (this.writeLocks.has(filePath)) {
      await new Promise<void>(resolve => {
        if (!this.waitingReaders.has(filePath)) {
          this.waitingReaders.set(filePath, []);
        }
        this.waitingReaders.get(filePath)!.push(resolve);
      });
    }

    // Increment read count
    const currentCount = this.readCounts.get(filePath) || 0;
    this.readCounts.set(filePath, currentCount + 1);
  }

  /**
   * Release in-process read lock
   */
  private async releaseReadLock(filePath: string): Promise<void> {
    const currentCount = this.readCounts.get(filePath) || 0;
    const newCount = Math.max(0, currentCount - 1);

    if (newCount === 0) {
      this.readCounts.delete(filePath);

      // Wake up waiting writers (only one writer at a time)
      const waitingWriters = this.waitingWriters.get(filePath);
      if (waitingWriters && waitingWriters.length > 0) {
        const nextWriter = waitingWriters.shift();
        if (nextWriter) {
          nextWriter();
        }
      }
    } else {
      this.readCounts.set(filePath, newCount);
    }
  }

  /**
   * Acquire in-process write lock (exclusive - blocks all readers and writers)
   */
  private async acquireWriteLock(filePath: string): Promise<void> {
    // Use a loop to handle spurious wakeups
    while (true) {
      // Check if lock is available
      if (!this.readCounts.get(filePath) && !this.writeLocks.has(filePath)) {
        // Lock is available, acquire it
        this.writeLocks.add(filePath);
        return;
      }

      // Lock is not available, wait
      await new Promise<void>(resolve => {
        if (!this.waitingWriters.has(filePath)) {
          this.waitingWriters.set(filePath, []);
        }
        this.waitingWriters.get(filePath)!.push(resolve);
      });
      // After being woken up, loop back to re-check the condition
    }
  }

  /**
   * Release in-process write lock
   */
  private async releaseWriteLock(filePath: string): Promise<void> {
    this.writeLocks.delete(filePath);

    // Wake up waiting readers (all of them - concurrent reads allowed)
    const waitingReaders = this.waitingReaders.get(filePath);
    if (waitingReaders && waitingReaders.length > 0) {
      const readers = [...waitingReaders];
      this.waitingReaders.delete(filePath);
      readers.forEach(resolve => resolve());
    } else {
      // No waiting readers, wake up next writer
      const waitingWriters = this.waitingWriters.get(filePath);
      if (waitingWriters && waitingWriters.length > 0) {
        const nextWriter = waitingWriters.shift();
        if (nextWriter) {
          nextWriter();
        }
      }
    }
  }

  /**
   * Log debug metrics if FSPEC_DEBUG_LOCKS is enabled
   */
  private logMetrics(metrics: LockMetrics): void {
    if (process.env.FSPEC_DEBUG_LOCKS) {
      logger.debug(
        `[LOCK] Acquired ${metrics.lockType} lock on ${metrics.filePath} ` +
          `(waited ${metrics.waitTimeMs}ms, held ${metrics.holdDurationMs}ms, retries ${metrics.retries})`
      );
    }
  }

  /**
   * Read JSON file with READ lock
   *
   * @param filePath - Absolute path to JSON file
   * @param defaultData - Default data if file doesn't exist
   * @returns Parsed JSON data
   *
   * Uses read-lock-first pattern:
   * 1. Try to read with READ lock
   * 2. On ENOENT, upgrade to WRITE lock and create file with double-check
   */
  async readJSON<T>(filePath: string, defaultData: T): Promise<T> {
    const startTime = Date.now();
    let release: (() => Promise<void>) | undefined;
    let retries = 0;

    try {
      // Layer 1: Acquire inter-process lock
      release = await lockfile.lock(filePath, {
        stale: 10000,
        retries: {
          retries: 10,
          minTimeout: 50,
          maxTimeout: 500,
        },
        realpath: false,
        onCompromised: () => {
          throw new Error('Lock compromised');
        },
      });

      // Layer 2: Acquire in-process READ lock
      await this.acquireReadLock(filePath);

      const holdStartTime = Date.now();

      try {
        // Try to read file
        const content = await readFile(filePath, 'utf-8');
        const data = JSON.parse(content);

        // Log metrics
        this.logMetrics({
          lockType: 'READ',
          filePath,
          waitTimeMs: holdStartTime - startTime,
          holdDurationMs: Date.now() - holdStartTime,
          retries,
        });

        return data;
      } catch (error: any) {
        // File doesn't exist - create it
        if (error.code === 'ENOENT') {
          // Release READ lock first
          await this.releaseReadLock(filePath);
          if (release) {
            await release();
            release = undefined;
          }

          // Acquire WRITE lock and create file
          return await this.createFileWithWriteLock(filePath, defaultData);
        }
        throw error;
      } finally {
        // Release in-process READ lock
        await this.releaseReadLock(filePath);
      }
    } finally {
      // Release inter-process lock
      if (release) {
        await release();
      }
    }
  }

  /**
   * Create file with WRITE lock (helper for readJSON ENOENT case)
   */
  private async createFileWithWriteLock<T>(
    filePath: string,
    defaultData: T
  ): Promise<T> {
    const startTime = Date.now();
    let release: (() => Promise<void>) | undefined;

    try {
      // Acquire inter-process WRITE lock
      release = await lockfile.lock(filePath, {
        stale: 10000,
        retries: {
          retries: 10,
          minTimeout: 50,
          maxTimeout: 500,
        },
        realpath: false,
      });

      // Acquire in-process WRITE lock
      await this.acquireWriteLock(filePath);

      const holdStartTime = Date.now();

      try {
        // Double-check: another process may have created it
        if (existsSync(filePath)) {
          const content = await readFile(filePath, 'utf-8');
          return JSON.parse(content);
        }

        // Still missing, create it
        await writeFile(filePath, JSON.stringify(defaultData, null, 2));

        this.logMetrics({
          lockType: 'WRITE',
          filePath,
          waitTimeMs: holdStartTime - startTime,
          holdDurationMs: Date.now() - holdStartTime,
          retries: 0,
        });

        return defaultData;
      } finally {
        await this.releaseWriteLock(filePath);
      }
    } finally {
      if (release) {
        await release();
      }
    }
  }

  /**
   * Execute read-modify-write transaction with exclusive WRITE lock
   *
   * @param filePath - Absolute path to JSON file
   * @param fn - Callback that mutates data in place
   *
   * @remarks
   * Mutation-based API: callback receives data, mutates in place, returns void.
   * On error: rollback (no write occurs, file unchanged, error propagates).
   *
   * Multi-file operations use sequential transactions (not atomic).
   * See: spec/attachments/LOCK-001/multi-file-consistency-strategy.md
   *
   * @example
   * ```typescript
   * // Single file transaction
   * await fileManager.transaction(workUnitsFile, async (data) => {
   *   data.workUnits[id].status = 'done';
   * });
   *
   * // Multi-file (sequential, not atomic)
   * await fileManager.transaction(workUnitsFile, async (data) => {
   *   data.workUnits[id] = newWorkUnit;
   * });
   * await fileManager.transaction(epicsFile, async (data) => {
   *   data.epics[epicId].workUnits.push(id);
   * });
   * ```
   */
  async transaction<T>(
    filePath: string,
    fn: (data: T) => Promise<void> | void
  ): Promise<void> {
    const startTime = Date.now();
    let release: (() => Promise<void>) | undefined;

    try {
      // Layer 1: Acquire inter-process WRITE lock
      release = await lockfile.lock(filePath, {
        stale: 10000,
        retries: {
          retries: 10,
          minTimeout: 50,
          maxTimeout: 500,
        },
        realpath: false,
      });

      // Layer 2: Acquire in-process WRITE lock
      await this.acquireWriteLock(filePath);

      const holdStartTime = Date.now();

      try {
        // Read current data
        let data: T;
        if (existsSync(filePath)) {
          const content = await readFile(filePath, 'utf-8');
          data = JSON.parse(content);
        } else {
          data = {} as T;
        }

        // Execute user callback (mutates data)
        await fn(data);

        // Layer 3: Atomic write-replace
        const tempFile = `${filePath}.tmp.${randomUUID()}`;
        await writeFile(tempFile, JSON.stringify(data, null, 2));
        await rename(tempFile, filePath);

        this.logMetrics({
          lockType: 'WRITE',
          filePath,
          waitTimeMs: holdStartTime - startTime,
          holdDurationMs: Date.now() - holdStartTime,
          retries: 0,
        });
      } catch (error) {
        // Rollback: error occurred, don't write
        // Clean up temp file if it exists
        const potentialTempFiles = filePath + '.tmp.';
        // Note: In production, would need to clean up temp files
        // For now, temp files are cleaned up by OS or manual cleanup

        throw error;
      } finally {
        await this.releaseWriteLock(filePath);
      }
    } finally {
      if (release) {
        await release();
      }
    }
  }
}

// Export singleton instance
export const fileManager = LockedFileManager.getInstance();
