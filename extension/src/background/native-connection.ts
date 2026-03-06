/**
 * fspec WebMCP Extension - Native Messaging Connection Manager
 *
 * Manages the chrome.runtime.connectNative connection to the native
 * messaging host process. Handles connection, disconnection, and
 * automatic reconnection.
 *
 * Implemented by: EXT-004
 */

import { NATIVE_MESSAGING_HOST_NAME } from '../server/mcp-constants';

/** Minimal Chrome runtime interface for dependency injection */
export interface ChromeRuntimeLike {
  connectNative: (hostName: string) => PortLike;
  lastError?: { message?: string } | null;
}

/** Minimal Port interface matching chrome.runtime.Port */
export interface PortLike {
  name: string;
  postMessage: (message: Record<string, unknown>) => void;
  onMessage: {
    addListener: (callback: (message: Record<string, unknown>) => void) => void;
    removeListener: (
      callback: (message: Record<string, unknown>) => void
    ) => void;
  };
  onDisconnect: {
    addListener: (callback: () => void) => void;
    removeListener: (callback: () => void) => void;
  };
  disconnect: () => void;
}

export interface NativeConnectionOptions {
  runtime: ChromeRuntimeLike;
  reconnectDelay?: number;
  maxReconnectAttempts?: number;
  onMessage?: (message: Record<string, unknown>) => void;
  onDisconnect?: () => void;
  onReconnect?: () => void;
}

export interface NativeConnectionAPI {
  connect: () => void;
  getPort: () => PortLike | null;
  isConnected: () => boolean;
  disconnect: () => void;
}

export function createNativeConnection(
  options: NativeConnectionOptions
): NativeConnectionAPI {
  const {
    runtime,
    reconnectDelay = 2000,
    maxReconnectAttempts = 5,
    onMessage,
    onDisconnect: onDisconnectCallback,
    onReconnect,
  } = options;

  let port: PortLike | null = null;
  let connected = false;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let intentionalDisconnect = false;
  let reconnectAttempts = 0;

  function messageHandler(message: Record<string, unknown>): void {
    if (onMessage) {
      onMessage(message);
    }
  }

  function disconnectHandler(): void {
    connected = false;
    port = null;

    if (onDisconnectCallback) {
      onDisconnectCallback();
    }

    // Attempt reconnection unless intentionally disconnected
    if (!intentionalDisconnect && reconnectAttempts < maxReconnectAttempts) {
      const delay = reconnectDelay * Math.pow(2, reconnectAttempts);
      reconnectAttempts++;
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        try {
          doConnect();
          reconnectAttempts = 0;
          if (onReconnect) {
            onReconnect();
          }
        } catch {
          // Trigger another reconnect attempt via the disconnect handler
          disconnectHandler();
        }
      }, delay);
    }
  }

  function doConnect(): void {
    port = runtime.connectNative(NATIVE_MESSAGING_HOST_NAME);
    connected = true;
    port.onMessage.addListener(messageHandler);
    port.onDisconnect.addListener(disconnectHandler);
  }

  return {
    connect(): void {
      if (connected) {
        return;
      }
      intentionalDisconnect = false;
      reconnectAttempts = 0;
      doConnect();
    },

    getPort(): PortLike | null {
      return port;
    },

    isConnected(): boolean {
      return connected;
    },

    disconnect(): void {
      intentionalDisconnect = true;
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
      if (port) {
        port.disconnect();
        port = null;
      }
      connected = false;
    },
  };
}
