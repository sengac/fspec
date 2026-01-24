/**
 * Rust Log Capture Utility for fspec
 *
 * Wires up Rust tracing logs to the TypeScript winston logger.
 * This MUST be called early in the application lifecycle to capture
 * all Rust logs (including session navigation, etc).
 *
 * Coverage: LOG-004
 */

import { logger } from './logger';
import { setRustLogCallback } from '@sengac/codelet-napi';

let initialized = false;

/**
 * Initialize Rust log capture
 * Call this as early as possible in the application entry point,
 * after console capture is initialized.
 */
export function initializeRustLogCapture(): void {
  if (initialized) {
    return;
  }

  try {
    setRustLogCallback((msg: string) => {
      // Route Rust logs through TypeScript logger at appropriate levels
      // The logger's configured level (FSPEC_LOG_LEVEL) controls what gets written
      if (msg.includes('[RUST:ERROR]')) {
        logger.error(msg);
      } else if (msg.includes('[RUST:WARN]')) {
        logger.warn(msg);
      } else if (msg.includes('[RUST:INFO]')) {
        logger.info(msg);
      } else if (msg.includes('[RUST:DEBUG]')) {
        logger.debug(msg);
      } else if (msg.includes('[RUST:TRACE]')) {
        // TRACE is very verbose - route to debug level
        // Will only be written if FSPEC_LOG_LEVEL=debug
        logger.debug(msg);
      }
    });

    initialized = true;
    logger.debug('Rust log capture initialized');
  } catch (err) {
    // Log but don't fail - codelet-napi might not be available in all contexts
    logger.warn(`Failed to initialize Rust log capture: ${err}`);
  }
}

/**
 * Check if Rust log capture is initialized
 */
export function isRustLogCaptureInitialized(): boolean {
  return initialized;
}
