/**
 * Cross-platform IPC utility for fspec
 * Uses Unix domain sockets (Linux/Mac) and Named Pipes (Windows)
 */

/* eslint-env node */

import net from 'net';
import fs from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';
import { logger } from './logger.js';

export interface IPCMessage {
  type: string;
  payload?: Record<string, unknown>;
}

/**
 * Get cross-platform IPC path
 * - Windows: Named pipe (\\.\pipe\fspec-tui)
 * - Unix: Socket file (/tmp/fspec-tui.sock)
 */
export function getIPCPath(name = 'fspec-tui'): string {
  if (process.platform === 'win32') {
    return `\\\\.\\pipe\\${name}`;
  }
  return join(tmpdir(), `${name}.sock`);
}

/**
 * Create IPC server for receiving messages
 * Used by TUI to listen for checkpoint updates
 */
export function createIPCServer(
  onMessage: (message: IPCMessage) => void
): net.Server {
  const pipePath = getIPCPath();

  logger.debug(`[IPC SERVER] Creating IPC server on path: ${pipePath}`);

  // Cleanup existing socket on Unix (Windows auto-cleans)
  if (process.platform !== 'win32') {
    try {
      fs.unlinkSync(pipePath);
      logger.debug(`[IPC SERVER] Cleaned up existing socket at ${pipePath}`);
    } catch {
      logger.debug(
        `[IPC SERVER] No existing socket to clean up at ${pipePath}`
      );
    }
  }

  const server = net.createServer(client => {
    logger.debug(`[IPC SERVER] Client connected`);

    client.on('data', data => {
      try {
        const message: IPCMessage = JSON.parse(data.toString());
        logger.debug(
          `[IPC SERVER] Message received: ${JSON.stringify(message)}`
        );
        onMessage(message);
        logger.debug(`[IPC SERVER] onMessage callback completed`);
      } catch (err) {
        logger.error(`[IPC SERVER] Failed to parse message: ${err}`);
      }
    });

    client.on('error', err => {
      logger.error(`[IPC SERVER] Client error: ${err.message}`);
    });

    client.on('close', () => {
      logger.debug(`[IPC SERVER] Client disconnected`);
    });
  });

  server.on('error', err => {
    logger.error(`[IPC SERVER] Server error: ${err}`);
  });

  server.on('listening', () => {
    logger.debug(`[IPC SERVER] Server listening on ${pipePath}`);
  });

  return server;
}

/**
 * Send IPC message to server
 * Used by checkpoint commands to notify TUI
 * Fails silently if TUI is not running
 * Returns a promise that resolves when message is sent or rejects on error
 */
export function sendIPCMessage(message: IPCMessage): Promise<void> {
  return new Promise(resolve => {
    const pipePath = getIPCPath();

    logger.debug(`[IPC CLIENT] Attempting to send message to: ${pipePath}`);
    logger.debug(`[IPC CLIENT] Message: ${JSON.stringify(message)}`);

    const client = net.connect(pipePath);

    client.on('connect', () => {
      logger.debug(`[IPC CLIENT] Connected to server, sending message`);
      const messageStr = JSON.stringify(message) + '\n'; // Add newline as delimiter
      logger.debug(`[IPC CLIENT] Writing ${messageStr.length} bytes`);

      // Write and immediately call destroy to force flush
      client.write(messageStr);
      // Small delay to allow write to complete before destroying
      setTimeout(() => {
        logger.debug(`[IPC CLIENT] Destroying connection to force flush`);
        client.destroy();
        resolve(); // Resolve promise after destroying connection
      }, 0);
    });

    client.on('close', () => {
      logger.debug(`[IPC CLIENT] Connection closed`);
    });

    // Fail silently if TUI not running
    client.on('error', err => {
      logger.debug(`[IPC CLIENT] Failed to send message: ${err.message}`);
      logger.debug(`[IPC CLIENT] TUI is not running - socket does not exist`);
      resolve(); // Don't reject - fail silently as per original behavior
    });
  });
}

/**
 * Cleanup IPC server resources
 * Call this when TUI exits
 */
export function cleanupIPCServer(server: net.Server): void {
  server.close();

  const pipePath = getIPCPath();

  // Manual cleanup for Unix sockets (Windows auto-cleans)
  if (process.platform !== 'win32') {
    try {
      fs.unlinkSync(pipePath);
    } catch {
      // Ignore cleanup errors (file may not exist)
    }
  }
}
