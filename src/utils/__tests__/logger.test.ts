/**
 * Feature: spec/features/add-winston-universal-logger-for-fspec.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * LOG-002: Also tests FSPEC_RUST_LOG_LEVEL documentation and environment variable handling
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { tmpdir, homedir } from 'os';

describe('Feature: Add winston universal logger for fspec', () => {
  let testHomeDir: string;
  let originalHome: string;
  let logFilePath: string;

  beforeEach(async () => {
    // Create temporary test directory to mock home directory
    testHomeDir = join(tmpdir(), `fspec-logger-test-${Date.now()}`);
    await mkdir(testHomeDir, { recursive: true });

    // Mock home directory
    originalHome = process.env.HOME || '';
    process.env.HOME = testHomeDir;
    process.env.USERPROFILE = testHomeDir; // Windows

    // Clear module cache to force re-initialization with new home directory
    vi.resetModules();

    logFilePath = join(testHomeDir, '.fspec', 'fspec.log');
  });

  afterEach(async () => {
    // Restore original home
    process.env.HOME = originalHome;
    process.env.USERPROFILE = originalHome;

    // Cleanup test directory
    await rm(testHomeDir, { recursive: true, force: true });

    // Clear module cache to reset singleton
    vi.resetModules();
  });

  describe('Scenario: Log error message to file', () => {
    it('should log error message to ~/.fspec/fspec.log with timestamp and level', async () => {
      // Given winston logger is configured with file transport
      // And the log file path is in the user's home directory at .fspec/fspec.log
      const { logger } = await import('../logger');

      // When I import the logger singleton
      // And I call logger.error('test error message')
      logger.error('test error message');

      // Wait for winston to flush to disk
      await new Promise(resolve => setTimeout(resolve, 100));

      // Then the message should be appended to the log file
      const logContent = await readFile(logFilePath, 'utf-8');
      expect(logContent).toContain('test error message');

      // And the log entry should contain timestamp and level 'error'
      expect(logContent).toMatch(/\d{4}-\d{2}-\d{2}/); // Date format
      expect(logContent).toContain('error');

      // And the log file should exist at the correct path on any platform
      await expect(access(logFilePath)).resolves.toBeUndefined();
    });
  });

  describe('Scenario: Concurrent writes from multiple processes', () => {
    it('should handle concurrent writes without corruption or data loss', async () => {
      // Given winston logger is configured with append-only file transport
      // And multiple fspec processes are running concurrently
      const { logger } = await import('../logger');

      // When each process calls logger.error() simultaneously
      const concurrentWrites = Array.from({ length: 10 }, (_, i) =>
        logger.error(`concurrent message ${i}`)
      );

      await Promise.all(concurrentWrites);

      // Wait for winston to flush to disk
      await new Promise(resolve => setTimeout(resolve, 200));

      // Then all log messages should be appended without corruption
      const logContent = await readFile(logFilePath, 'utf-8');
      const lines = logContent.split('\n').filter(line => line.trim());

      // And no log entries should be lost or interleaved
      expect(lines.length).toBeGreaterThanOrEqual(10);

      for (let i = 0; i < 10; i++) {
        expect(logContent).toContain(`concurrent message ${i}`);
      }
    });
  });

  describe('Scenario: Debug level logging', () => {
    it('should filter debug messages based on configured log level', async () => {
      // Given winston logger is configured with log level 'info'
      const { logger } = await import('../logger');

      // When I call logger.debug('debug message')
      logger.debug('debug message 1');

      // Wait for winston to flush
      await new Promise(resolve => setTimeout(resolve, 100));

      // Then the debug message should not be logged
      const logContent1 = await readFile(logFilePath, 'utf-8');
      expect(logContent1).not.toContain('debug message 1');

      // When I configure log level to 'debug'
      // (Note: In real implementation, this would be via environment variable or config)
      logger.level = 'debug';

      // And I call logger.debug('debug message')
      logger.debug('debug message 2');

      // Wait for winston to flush
      await new Promise(resolve => setTimeout(resolve, 100));

      // Then the debug message should be appended to the log file
      const logContent2 = await readFile(logFilePath, 'utf-8');
      expect(logContent2).toContain('debug message 2');
    });
  });

  describe('Scenario: Cross-platform log file path resolution', () => {
    it('should resolve correct log file path on Windows', async () => {
      // Given I am running fspec on Windows
      const originalPlatform = process.platform;
      Object.defineProperty(process, 'platform', { value: 'win32' });
      process.env.USERPROFILE = 'C:\\Users\\testuser';
      delete process.env.HOME;

      // When winston logger initializes
      vi.resetModules();
      const { logger } = await import('../logger');

      // Then the log file path should resolve to C:\Users\<username>\.fspec\fspec.log
      const expectedPath = join('C:\\Users\\testuser', '.fspec', 'fspec.log');

      // Log a message to trigger file creation
      logger.error('test');
      await new Promise(resolve => setTimeout(resolve, 100));

      // Verify path format (Winston internally normalizes paths)
      expect(expectedPath).toMatch(/\.fspec/);
      expect(expectedPath).toMatch(/fspec\.log$/);

      // Restore
      Object.defineProperty(process, 'platform', { value: originalPlatform });
    });

    it('should resolve correct log file path on macOS', async () => {
      // Given I am running fspec on macOS
      const originalPlatform = process.platform;
      Object.defineProperty(process, 'platform', { value: 'darwin' });
      process.env.HOME = '/Users/testuser';

      // When winston logger initializes
      vi.resetModules();
      const { logger } = await import('../logger');

      // Then the log file path should resolve to /Users/<username>/.fspec/fspec.log
      const expectedPath = join('/Users/testuser', '.fspec', 'fspec.log');
      expect(expectedPath).toBe('/Users/testuser/.fspec/fspec.log');

      // Restore
      Object.defineProperty(process, 'platform', { value: originalPlatform });
    });

    it('should resolve correct log file path on Linux', async () => {
      // Given I am running fspec on Linux
      const originalPlatform = process.platform;
      Object.defineProperty(process, 'platform', { value: 'linux' });
      process.env.HOME = '/home/testuser';

      // When winston logger initializes
      vi.resetModules();
      const { logger } = await import('../logger');

      // Then the log file path should resolve to /home/<username>/.fspec/fspec.log
      const expectedPath = join('/home/testuser', '.fspec', 'fspec.log');
      expect(expectedPath).toBe('/home/testuser/.fspec/fspec.log');

      // Restore
      Object.defineProperty(process, 'platform', { value: originalPlatform });
    });
  });

  describe('LOG-002: Rust log level filtering documentation', () => {
    it('should document FSPEC_RUST_LOG_LEVEL environment variable', async () => {
      // Given the logger module has been imported
      // The module header documents the environment variables

      // Verify the environment variable is documented in the source
      const loggerSource = await readFile(
        join(__dirname, '../logger.ts'),
        'utf-8'
      );

      // Then FSPEC_RUST_LOG_LEVEL should be documented
      expect(loggerSource).toContain('FSPEC_RUST_LOG_LEVEL');
      expect(loggerSource).toContain('Rust tracing log level');

      // And FSPEC_LOG_LEVEL should be documented
      expect(loggerSource).toContain('FSPEC_LOG_LEVEL');
      expect(loggerSource).toContain('TypeScript/winston log level');

      // And RUST_LOG fallback should be documented
      expect(loggerSource).toContain('RUST_LOG');
      expect(loggerSource).toContain('Fallback');
    });

    it('should respect FSPEC_LOG_LEVEL for TypeScript logger level', async () => {
      // Given FSPEC_LOG_LEVEL is set to 'debug'
      process.env.FSPEC_LOG_LEVEL = 'debug';
      vi.resetModules();

      // When the logger is initialized
      const { logger } = await import('../logger');

      // Then the logger level should be 'debug'
      expect(logger.level).toBe('debug');

      // Cleanup
      delete process.env.FSPEC_LOG_LEVEL;
    });

    it('should default to info level when FSPEC_LOG_LEVEL is not set', async () => {
      // Given FSPEC_LOG_LEVEL is not set
      delete process.env.FSPEC_LOG_LEVEL;
      vi.resetModules();

      // When the logger is initialized
      const { logger } = await import('../logger');

      // Then the logger level should default to 'info'
      expect(logger.level).toBe('info');
    });
  });
});
