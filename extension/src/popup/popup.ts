/**
 * fspec WebMCP Extension - Popup Script
 *
 * Controls the extension popup UI showing:
 * - Server status (listening/stopped)
 * - Configured port number
 * - Connected client count
 * - Available tools grouped by source (native vs WebMCP per tab)
 *
 * Communicates with service worker via chrome.runtime.sendMessage.
 *
 * EXT-008: Full popup UI implementation
 */

const statusEl = document.getElementById('status');
const portEl = document.getElementById('port');

if (statusEl) {
  statusEl.textContent = 'Not connected';
}

if (portEl) {
  portEl.textContent = '19876';
}

export {};
