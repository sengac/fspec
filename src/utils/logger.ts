/**
 * Winston Universal Logger for fspec
 *
 * Singleton logger instance that logs to ~/.fspec/fspec.log
 * Uses os.homedir() + path.join() for cross-platform compatibility
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
import { getFspecUserDir } from './config';

// Lazy initialization to respect mocked environment in tests
let loggerInstance: winston.Logger | null = null;
let logFilePathCache: string | null = null;

function initializeLogger(): winston.Logger {
  if (loggerInstance) {
    return loggerInstance;
  }

  // Platform-agnostic log file path
  const logDir = getFspecUserDir();
  logFilePathCache = join(logDir, 'fspec.log');

  // Ensure .fspec directory exists
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
