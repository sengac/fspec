/**
 * Cross-platform IPC utility for fspec
 * Uses Unix domain sockets (Linux/Mac) and Named Pipes (Windows)
 */

import net from 'net';
import fs from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

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

  // Cleanup existing socket on Unix (Windows auto-cleans)
  if (process.platform !== 'win32') {
    try {
      fs.unlinkSync(pipePath);
    } catch (error) {
      // Ignore cleanup errors (file may not exist)
    }
  }

  const server = net.createServer(client => {
    client.on('data', data => {
      try {
        const message: IPCMessage = JSON.parse(data.toString());
        onMessage(message);
      } catch (error) {
        // Ignore malformed messages
      }
    });

    client.on('error', () => {
      // Ignore client errors
    });
  });

  server.on('error', error => {
    // Ignore server errors (e.g., address already in use)
  });

  return server;
}

/**
 * Send IPC message to server
 * Used by checkpoint commands to notify TUI
 * Fails silently if TUI is not running
 */
export function sendIPCMessage(message: IPCMessage): void {
  const pipePath = getIPCPath();

  const client = net.connect(pipePath, () => {
    client.write(JSON.stringify(message));
    client.end();
  });

  // Fail silently if TUI not running
  client.on('error', () => {
    // TUI not running, ignore
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
    } catch (error) {
      // Ignore cleanup errors (file may not exist)
    }
  }
}
