/**
 * Tests for checkpoint-index utilities
 *
 * Coverage:
 * - TUI-016: Checkpoint counts in TUI
 * - GIT-004: Checkpoint viewer functionality
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fsPromises } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  getCheckpointIndexDir,
  checkpointIndexDirExists,
  listCheckpointIndexFiles,
  readCheckpointIndexFile,
  countCheckpoints,
  isAutomaticCheckpoint,
  parseAutomaticCheckpointName,
  AUTO_CHECKPOINT_PATTERN,
} from '../checkpoint-index';

describe('Feature: Checkpoint Index Utilities', () => {
  let testDir: string;
  let indexDir: string;

  beforeEach(async () => {
    // Create unique test directory
    testDir = join(tmpdir(), `fspec-checkpoint-index-test-${Date.now()}`);
    await fsPromises.mkdir(testDir, { recursive: true });
    await fsPromises.mkdir(join(testDir, '.git'), { recursive: true });
    indexDir = join(testDir, '.git', 'fspec-checkpoints-index');
  });

  afterEach(async () => {
    // Cleanup test directory
    try {
      await fsPromises.rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Scenario: getCheckpointIndexDir returns correct path', () => {
    it('should return the checkpoint index directory path', () => {
      // Given a working directory
      const cwd = '/some/project';

      // When I get the checkpoint index directory
      const result = getCheckpointIndexDir(cwd);

      // Then it should return the correct path
      expect(result).toBe('/some/project/.git/fspec-checkpoints-index');
    });
  });

  describe('Scenario: checkpointIndexDirExists for non-existent directory', () => {
    it('should return false when directory does not exist', () => {
      // Given a directory without checkpoints
      // (indexDir not created)

      // When I check if the checkpoint index directory exists
      const result = checkpointIndexDirExists(testDir);

      // Then it should return false
      expect(result).toBe(false);
    });
  });

  describe('Scenario: checkpointIndexDirExists for existing directory', () => {
    it('should return true when directory exists', async () => {
      // Given a directory with checkpoint index
      await fsPromises.mkdir(indexDir, { recursive: true });

      // When I check if the checkpoint index directory exists
      const result = checkpointIndexDirExists(testDir);

      // Then it should return true
      expect(result).toBe(true);
    });
  });

  describe('Scenario: listCheckpointIndexFiles for non-existent directory', () => {
    it('should return empty array without throwing error', async () => {
      // Given a directory without checkpoint index
      // (indexDir not created - simulates new project or test environment)

      // When I list checkpoint index files
      const result = await listCheckpointIndexFiles(testDir);

      // Then it should return empty array (not throw ENOENT)
      expect(result).toEqual([]);
    });
  });

  describe('Scenario: listCheckpointIndexFiles for empty directory', () => {
    it('should return empty array when directory is empty', async () => {
      // Given an empty checkpoint index directory
      await fsPromises.mkdir(indexDir, { recursive: true });

      // When I list checkpoint index files
      const result = await listCheckpointIndexFiles(testDir);

      // Then it should return empty array
      expect(result).toEqual([]);
    });
  });

  describe('Scenario: listCheckpointIndexFiles filters JSON files', () => {
    it('should return only JSON files', async () => {
      // Given a checkpoint index directory with mixed files
      await fsPromises.mkdir(indexDir, { recursive: true });
      await fsPromises.writeFile(join(indexDir, 'WORK-001.json'), '{}');
      await fsPromises.writeFile(join(indexDir, 'BUG-002.json'), '{}');
      await fsPromises.writeFile(join(indexDir, '.gitkeep'), '');
      await fsPromises.writeFile(join(indexDir, 'readme.txt'), '');

      // When I list checkpoint index files
      const result = await listCheckpointIndexFiles(testDir);

      // Then it should return only JSON files
      expect(result).toHaveLength(2);
      expect(result).toContain('WORK-001.json');
      expect(result).toContain('BUG-002.json');
      expect(result).not.toContain('.gitkeep');
      expect(result).not.toContain('readme.txt');
    });
  });

  describe('Scenario: readCheckpointIndexFile for non-existent file', () => {
    it('should return empty checkpoints list', async () => {
      // Given a work unit without checkpoints
      await fsPromises.mkdir(indexDir, { recursive: true });

      // When I read the checkpoint index file
      const result = await readCheckpointIndexFile(testDir, 'WORK-001');

      // Then it should return empty checkpoints
      expect(result).toEqual({ checkpoints: [] });
    });
  });

  describe('Scenario: readCheckpointIndexFile for valid file', () => {
    it('should return parsed checkpoint data', async () => {
      // Given a valid checkpoint index file
      await fsPromises.mkdir(indexDir, { recursive: true });
      const indexData = {
        checkpoints: [
          {
            name: 'checkpoint-1',
            message: 'fspec-checkpoint:WORK-001:checkpoint-1:123456',
          },
          {
            name: 'WORK-001-auto-backlog',
            message: 'fspec-checkpoint:WORK-001:auto:789012',
          },
        ],
      };
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        JSON.stringify(indexData)
      );

      // When I read the checkpoint index file
      const result = await readCheckpointIndexFile(testDir, 'WORK-001');

      // Then it should return the checkpoint data
      expect(result.checkpoints).toHaveLength(2);
      expect(result.checkpoints[0].name).toBe('checkpoint-1');
      expect(result.checkpoints[1].name).toBe('WORK-001-auto-backlog');
    });
  });

  describe('Scenario: readCheckpointIndexFile for corrupted file', () => {
    it('should return empty checkpoints list', async () => {
      // Given a corrupted checkpoint index file
      await fsPromises.mkdir(indexDir, { recursive: true });
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        'not valid json {'
      );

      // When I read the checkpoint index file
      const result = await readCheckpointIndexFile(testDir, 'WORK-001');

      // Then it should return empty checkpoints (graceful degradation)
      expect(result).toEqual({ checkpoints: [] });
    });
  });

  describe('Scenario: countCheckpoints for non-existent directory', () => {
    it('should return zeros without throwing error', async () => {
      // Given a directory without checkpoint index
      // (indexDir not created - simulates new project)

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should return zeros (not throw ENOENT)
      expect(result).toEqual({ manual: 0, auto: 0 });
    });
  });

  describe('Scenario: countCheckpoints for empty directory', () => {
    it('should return zeros when no checkpoints exist', async () => {
      // Given an empty checkpoint index directory
      await fsPromises.mkdir(indexDir, { recursive: true });

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should return zeros
      expect(result).toEqual({ manual: 0, auto: 0 });
    });
  });

  describe('Scenario: countCheckpoints correctly distinguishes manual vs auto', () => {
    it('should count manual and auto checkpoints separately', async () => {
      // Given checkpoint index files with mixed checkpoints
      await fsPromises.mkdir(indexDir, { recursive: true });

      // Work unit 1: 2 manual, 1 auto
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        JSON.stringify({
          checkpoints: [
            { name: 'manual-checkpoint', message: 'msg1' },
            { name: 'WORK-001-auto-backlog', message: 'msg2' },
            { name: 'another-manual', message: 'msg3' },
          ],
        })
      );

      // Work unit 2: 1 manual, 3 auto
      await fsPromises.writeFile(
        join(indexDir, 'BUG-002.json'),
        JSON.stringify({
          checkpoints: [
            { name: 'fix-attempt', message: 'msg4' },
            { name: 'BUG-002-auto-specifying', message: 'msg5' },
            { name: 'BUG-002-auto-testing', message: 'msg6' },
            { name: 'BUG-002-auto-implementing', message: 'msg7' },
          ],
        })
      );

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should correctly count manual vs auto
      expect(result.manual).toBe(3); // 2 + 1
      expect(result.auto).toBe(4); // 1 + 3
    });
  });

  describe('Scenario: countCheckpoints skips corrupted files', () => {
    it('should skip corrupted files and continue counting', async () => {
      // Given checkpoint index with one valid and one corrupted file
      await fsPromises.mkdir(indexDir, { recursive: true });

      // Valid file
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        JSON.stringify({
          checkpoints: [
            { name: 'manual-1', message: 'msg1' },
            { name: 'WORK-001-auto-backlog', message: 'msg2' },
          ],
        })
      );

      // Corrupted file
      await fsPromises.writeFile(
        join(indexDir, 'BUG-002.json'),
        'this is not valid JSON {'
      );

      // Another valid file
      await fsPromises.writeFile(
        join(indexDir, 'FEAT-003.json'),
        JSON.stringify({
          checkpoints: [{ name: 'FEAT-003-auto-testing', message: 'msg3' }],
        })
      );

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should count from valid files only (skip corrupted)
      expect(result.manual).toBe(1); // manual-1
      expect(result.auto).toBe(2); // WORK-001-auto-backlog + FEAT-003-auto-testing
    });
  });

  describe('Scenario: countCheckpoints handles empty checkpoints array', () => {
    it('should handle files with empty checkpoints array', async () => {
      // Given a checkpoint index file with empty checkpoints array
      await fsPromises.mkdir(indexDir, { recursive: true });
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        JSON.stringify({ checkpoints: [] })
      );

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should return zeros
      expect(result).toEqual({ manual: 0, auto: 0 });
    });
  });

  describe('Scenario: countCheckpoints handles missing checkpoints property', () => {
    it('should handle files without checkpoints property', async () => {
      // Given a checkpoint index file without checkpoints property
      await fsPromises.mkdir(indexDir, { recursive: true });
      await fsPromises.writeFile(
        join(indexDir, 'WORK-001.json'),
        JSON.stringify({ other: 'data' })
      );

      // When I count checkpoints
      const result = await countCheckpoints(testDir);

      // Then it should return zeros (graceful handling)
      expect(result).toEqual({ manual: 0, auto: 0 });
    });
  });

  describe('Scenario: AUTO_CHECKPOINT_PATTERN constant is correct', () => {
    it('should export the correct pattern constant', () => {
      // The pattern should be '-auto-' for consistency across the codebase
      expect(AUTO_CHECKPOINT_PATTERN).toBe('-auto-');
    });
  });

  describe('Scenario: isAutomaticCheckpoint identifies automatic checkpoints', () => {
    it('should return true for automatic checkpoint names', () => {
      // Automatic checkpoints follow the pattern {workUnitId}-auto-{state}
      expect(isAutomaticCheckpoint('TUI-001-auto-testing')).toBe(true);
      expect(isAutomaticCheckpoint('BUG-002-auto-specifying')).toBe(true);
      expect(isAutomaticCheckpoint('FEAT-100-auto-implementing')).toBe(true);
      expect(isAutomaticCheckpoint('WORK-001-auto-backlog')).toBe(true);
    });

    it('should return false for manual checkpoint names', () => {
      // Manual checkpoints don't contain '-auto-'
      expect(isAutomaticCheckpoint('manual-checkpoint')).toBe(false);
      expect(isAutomaticCheckpoint('fix-attempt')).toBe(false);
      expect(isAutomaticCheckpoint('baseline-v1')).toBe(false);
      expect(isAutomaticCheckpoint('before-refactor')).toBe(false);
    });

    it('should return false for names that partially match', () => {
      // Ensure we match '-auto-' not just 'auto'
      expect(isAutomaticCheckpoint('automatic-fix')).toBe(false);
      expect(isAutomaticCheckpoint('auto-backup')).toBe(false);
      expect(isAutomaticCheckpoint('my-auto-checkpoint')).toBe(true); // contains '-auto-'
    });
  });

  describe('Scenario: parseAutomaticCheckpointName extracts parts', () => {
    it('should parse automatic checkpoint name into parts', () => {
      // Given an automatic checkpoint name
      const result = parseAutomaticCheckpointName('TUI-001-auto-testing');

      // Then it should extract workUnitId and state
      expect(result).toEqual({
        workUnitId: 'TUI-001',
        state: 'testing',
      });
    });

    it('should handle various work unit ID formats', () => {
      expect(parseAutomaticCheckpointName('BUG-002-auto-specifying')).toEqual({
        workUnitId: 'BUG-002',
        state: 'specifying',
      });

      expect(
        parseAutomaticCheckpointName('FEAT-100-auto-implementing')
      ).toEqual({
        workUnitId: 'FEAT-100',
        state: 'implementing',
      });

      expect(parseAutomaticCheckpointName('ABC-1-auto-backlog')).toEqual({
        workUnitId: 'ABC-1',
        state: 'backlog',
      });
    });

    it('should return null for manual checkpoint names', () => {
      expect(parseAutomaticCheckpointName('manual-checkpoint')).toBeNull();
      expect(parseAutomaticCheckpointName('fix-attempt')).toBeNull();
      expect(parseAutomaticCheckpointName('baseline-v1')).toBeNull();
    });

    it('should return null for empty string', () => {
      expect(parseAutomaticCheckpointName('')).toBeNull();
    });

    it('should return null for edge case with multiple -auto- occurrences', () => {
      // If there are multiple -auto- patterns, the name is malformed
      // and we should return null rather than try to parse it incorrectly
      const result = parseAutomaticCheckpointName('WORK-auto-001-auto-testing');

      // Split gives ['WORK', '001', 'testing'] - 3 parts, not 2, so returns null
      expect(result).toBeNull();
    });
  });
});
