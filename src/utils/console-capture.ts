/**
 * Console Capture Utility for fspec
 *
 * Intercepts all console methods and redirects output to winston logger
 * while preserving original console behavior.
 *
 * Coverage: LOG-003
 */

import { logger } from './logger';

// Store original console methods
const originalConsole = {
  log: console.log,
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  trace: console.trace,
};

// ANSI escape code stripping regex
// eslint-disable-next-line no-control-regex
const ANSI_REGEX = /\x1b\[[0-9;]*m/g;

/**
 * Strip ANSI escape codes (chalk formatting) from a string
 */
function stripAnsi(str: string): string {
  return str.replace(ANSI_REGEX, '');
}

/**
 * Format arguments for logging
 * Handles multiple arguments, objects, and strips ANSI codes
 */
function formatArgs(args: unknown[]): string {
  return stripAnsi(
    args
      .map(arg => {
        if (typeof arg === 'string') return arg;
        if (arg instanceof Error) return arg.stack || arg.message;
        try {
          return JSON.stringify(arg, null, 2);
        } catch {
          return String(arg);
        }
      })
      .join(' ')
  );
}

/**
 * Initialize console capture
 * Call this as early as possible in the application entry point
 */
export function initializeConsoleCapture(): void {
  // Override console.log -> logger.info
  console.log = (...args: unknown[]) => {
    originalConsole.log(...args);
    logger.info(formatArgs(args));
  };

  // Override console.info -> logger.info
  console.info = (...args: unknown[]) => {
    originalConsole.info(...args);
    logger.info(formatArgs(args));
  };

  // Override console.warn -> logger.warn
  console.warn = (...args: unknown[]) => {
    originalConsole.warn(...args);
    logger.warn(formatArgs(args));
  };

  // Override console.error -> logger.error
  console.error = (...args: unknown[]) => {
    originalConsole.error(...args);
    logger.error(formatArgs(args));
  };

  // Override console.debug -> logger.debug
  console.debug = (...args: unknown[]) => {
    originalConsole.debug(...args);
    logger.debug(formatArgs(args));
  };

  // Override console.trace -> logger.debug with stack trace
  console.trace = (...args: unknown[]) => {
    originalConsole.trace(...args);
    const stack = new Error().stack || '';
    logger.debug(`${formatArgs(args)}\n${stack}`);
  };
}

/**
 * Restore original console methods
 * Primarily for testing purposes
 */
export function restoreConsole(): void {
  console.log = originalConsole.log;
  console.error = originalConsole.error;
  console.warn = originalConsole.warn;
  console.info = originalConsole.info;
  console.debug = originalConsole.debug;
  console.trace = originalConsole.trace;
}

/**
 * Get original console methods
 * For cases where direct console access is needed (e.g., test output)
 */
export const originalConsoleMethods = originalConsole;
