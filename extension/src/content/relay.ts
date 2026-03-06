/**
 * fspec WebMCP Extension - Content Script Relay
 *
 * Extracts the bidirectional relay logic into a testable factory.
 * The relay bridges:
 *   Main world (window.postMessage) ↔ Service worker (chrome.runtime)
 *
 * Using FSPEC_WEBMCP_ prefix for main→SW tool registration,
 * and FSPEC_INVOKE_ prefix for SW→main tool invocations.
 *
 * Implemented by: EXT-004
 */

import { MESSAGE_TYPES } from '../types';

/** Prefix shared by all WebMCP registration messages */
const WEBMCP_PREFIX = 'FSPEC_WEBMCP_';

/** Minimal interface for the window's postMessage + addEventListener */
export interface WindowLike {
  addEventListener: (type: string, handler: (event: MessageEvent) => void) => void;
  postMessage: (message: unknown, targetOrigin: string) => void;
}

/** Minimal interface for chrome.runtime messaging from content scripts */
export interface ContentRuntimeLike {
  sendMessage: (message: unknown) => void;
  onMessage: {
    addListener: (
      callback: (
        message: { type?: string },
        sender: unknown,
        sendResponse: (response?: unknown) => void
      ) => boolean | void
    ) => void;
  };
}

export interface ContentRelayAPI {
  /**
   * Process a MessageEvent from window.addEventListener('message').
   * Returns true if the message was forwarded to the service worker.
   */
  handleWindowMessage: (event: { source: unknown; data: unknown }) => boolean;

  /**
   * Process a message from chrome.runtime.onMessage.
   * Returns true if the message was forwarded to the main world.
   */
  handleRuntimeMessage: (message: { type?: string }) => boolean;
}

export interface ContentRelayOptions {
  win: WindowLike;
  runtime: ContentRuntimeLike;
}

export function createContentRelay(options: ContentRelayOptions): ContentRelayAPI {
  const { win, runtime } = options;

  const relay: ContentRelayAPI = {
    handleWindowMessage(event: { source: unknown; data: unknown }): boolean {
      // Only accept messages from our own window
      if (event.source !== win) {
        return false;
      }

      const data = event.data as { type?: string } | undefined;
      if (!data?.type) {
        return false;
      }

      // Forward WebMCP tool registration/unregistration to service worker
      if (data.type.startsWith(WEBMCP_PREFIX)) {
        runtime.sendMessage(data);
        return true;
      }

      // Forward tool invocation results to service worker
      if (data.type === MESSAGE_TYPES.INVOKE_RESULT) {
        runtime.sendMessage(data);
        return true;
      }

      return false;
    },

    handleRuntimeMessage(message: { type?: string }): boolean {
      // Forward tool invocation requests to main world
      if (message?.type === MESSAGE_TYPES.INVOKE_TOOL) {
        win.postMessage(message, '*');
        return true;
      }

      return false;
    },
  };

  // Wire up event listeners
  win.addEventListener('message', (event: MessageEvent) => {
    relay.handleWindowMessage(event);
  });

  runtime.onMessage.addListener(
    (
      message: { type?: string },
      _sender: unknown,
      _sendResponse: (response?: unknown) => void
    ) => {
      relay.handleRuntimeMessage(message);
      return false;
    }
  );

  return relay;
}
