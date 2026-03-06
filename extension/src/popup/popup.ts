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

import type { StatusResponse } from '../types';
import { MESSAGE_TYPES } from '../types';
import { groupToolsBySource, deriveStatus } from './popup-utils';

function renderStatus(response: StatusResponse): void {
  const statusEl = document.getElementById('status');
  const statusIndicatorEl = document.querySelector('.status-indicator');
  const portEl = document.getElementById('port');
  const clientsEl = document.getElementById('clients');
  const toolsCountEl = document.getElementById('tools');
  const toolsListEl = document.getElementById('tools-list');

  if (
    !statusEl ||
    !statusIndicatorEl ||
    !portEl ||
    !clientsEl ||
    !toolsCountEl
  ) {
    return;
  }

  // Status
  const status = deriveStatus(response.nativeConnected);
  statusEl.textContent = status.text;
  statusIndicatorEl.className = `status-indicator ${status.cssClass}`;

  // Port
  portEl.textContent = String(response.port);

  // Clients
  clientsEl.textContent = String(response.clientCount);

  // Tools count
  toolsCountEl.textContent = String(response.toolCount);

  // Tools grouped display
  if (toolsListEl) {
    const groups = groupToolsBySource(response.tools);
    toolsListEl.innerHTML = '';

    for (const group of groups) {
      const section = document.createElement('div');
      section.className = 'tool-group';

      const header = document.createElement('div');
      header.className = 'tool-group-header';
      header.textContent = `${group.label} (${group.count})`;
      section.appendChild(header);

      for (const tool of group.tools) {
        const item = document.createElement('div');
        item.className = 'tool-item';
        item.textContent = tool.name;
        section.appendChild(item);
      }

      toolsListEl.appendChild(section);
    }
  }
}

// Query service worker for current status on popup open
chrome.runtime.sendMessage(
  { type: MESSAGE_TYPES.GET_STATUS },
  (response: StatusResponse) => {
    if (chrome.runtime.lastError) {
      // Service worker not responding — show disconnected state
      renderStatus({
        connected: false,
        nativeConnected: false,
        toolCount: 0,
        port: 19876,
        clientCount: 0,
        tools: [],
      });
      return;
    }
    renderStatus(response);
  }
);

export {};
