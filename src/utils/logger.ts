/**
 * Winston Universal Logger for fspec
 *
 * Singleton logger instance that logs to ~/.fspec/fspec.log
 * Uses os.homedir() + path.join() for cross-platform compatibility
 *
 * In test mode (VITEST env var set), logs are written to a temp directory
 * to prevent polluting the user's log file with test output.
 *
 * Coverage: LOG-001, LOG-002
 *
 * Environment Variables:
 * - FSPEC_LOG_LEVEL: Controls TypeScript/winston log level (default: 'info')
 *   Values: 'error', 'warn', 'info', 'debug'
 *
 * - FSPEC_RUST_LOG_LEVEL: Controls Rust tracing log level (default: 'warn')
 *   Values: 'error', 'warn', 'info', 'debug', 'trace'
 *   Supports complex directives like 'info,rig::completions=off'
 *
 * - RUST_LOG: Fallback for Rust log level if FSPEC_RUST_LOG_LEVEL not set
 */

import winston from 'winston';
import { join } from 'path';
import { mkdirSync, existsSync } from 'fs';
import { tmpdir } from 'os';
import { getFspecUserDir } from './config';

// Lazy initialization to respect mocked environment in tests
let loggerInstance: winston.Logger | null = null;
let logFilePathCache: string | null = null;

/**
 * Check if we're running in test mode
 */
function isTestMode(): boolean {
  return !!(process.env.VITEST || process.env.NODE_ENV === 'test');
}

/**
 * Get the log directory, using temp dir in test mode unless HOME is explicitly set
 *
 * This allows tests to either:
 * 1. Use the automatic test temp directory (default for most tests)
 * 2. Set HOME to a custom test directory (for tests that need to test home dir behavior)
 */
function getLogDir(): string {
  // If running in test mode and HOME hasn't been changed to a temp directory,
  // use a shared test temp directory to avoid polluting user's log file
  if (isTestMode()) {
    const home = process.env.HOME || process.env.USERPROFILE || '';
    // If HOME points to a temp directory, the test wants to control the path
    const isTempHome =
      home.includes(tmpdir()) || home.includes('fspec-logger-test');
    if (!isTempHome) {
      // Use shared temp directory for test logs
      return join(tmpdir(), 'fspec-test-logs');
    }
  }
  return getFspecUserDir();
}

function initializeLogger(): winston.Logger {
  if (loggerInstance) {
    return loggerInstance;
  }

  // Platform-agnostic log file path
  const logDir = getLogDir();
  logFilePathCache = join(logDir, 'fspec.log');

  // Ensure log directory exists
  if (!existsSync(logDir)) {
    mkdirSync(logDir, { recursive: true });
  }

  // Create winston logger with file transport
  loggerInstance = winston.createLogger({
    level: process.env.FSPEC_LOG_LEVEL || 'info',
    format: winston.format.combine(
      winston.format.timestamp(),
      winston.format.printf(({ timestamp, level, message }) => {
        return `${timestamp} [${level}]: ${message}`;
      })
    ),
    transports: [
      new winston.transports.File({
        filename: logFilePathCache,
        flags: 'a', // append mode (safe for concurrent writes)
      }),
    ],
  });

  return loggerInstance;
}

// Export lazy-initialized logger with support for level changes
export const logger = new Proxy({} as winston.Logger, {
  get(_target, prop) {
    const instance = initializeLogger();
    return instance[prop as keyof winston.Logger];
  },
  set(_target, prop, value) {
    const instance = initializeLogger();
    (instance as any)[prop] = value;
    return true;
  },
});

// Export log file path getter for testing
export const logFilePath = (): string => {
  if (!logFilePathCache) {
    initializeLogger();
  }
  return logFilePathCache!;
};
