/**
 * fspec Browser Agent - Content Script Entry Point
 *
 * Runs in every web page (isolated world). Acts as relay between:
 * - Main-world injected scripts (WebMCP tool discovery/invocation)
 * - Service worker (via chrome.runtime.sendMessage)
 *
 * Content scripts share the page's DOM but NOT its JavaScript context.
 * They communicate with main-world scripts via window.postMessage().
 *
 * Implemented by: EXT-004
 */

import { createContentRelay } from './relay';

createContentRelay({
  win: window,
  runtime: chrome.runtime,
});

export {};
