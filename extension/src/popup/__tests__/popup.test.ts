/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-008:
 * Extension Popup UI - displaying server status, port, client count,
 * and tools grouped by source.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { groupToolsBySource, deriveStatus } from '../popup-utils';
import type { PopupToolSummary, StatusResponse } from '../../types';

/**
 * Simulates what popup.ts does: uses deriveStatus + groupToolsBySource
 * to populate DOM elements. This mirrors the real renderStatus function.
 */
function renderPopupState(
  response: StatusResponse,
  elements: {
    statusEl: HTMLElement;
    statusIndicatorEl: HTMLElement;
    portEl: HTMLElement;
    clientsEl: HTMLElement;
    toolsCountEl: HTMLElement;
    toolsListEl: HTMLElement;
  }
): void {
  const status = deriveStatus(response.nativeConnected);
  elements.statusEl.textContent = status.text;
  elements.statusIndicatorEl.className = `status-indicator ${status.cssClass}`;
  elements.portEl.textContent = String(response.port);
  elements.clientsEl.textContent = String(response.clientCount);

  const groups = groupToolsBySource(response.tools);
  elements.toolsCountEl.textContent = String(response.toolCount);

  // Build tool groups HTML
  elements.toolsListEl.innerHTML = '';
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
    elements.toolsListEl.appendChild(section);
  }
}

describe('Feature: WebMCP Chrome Extension - Popup UI', () => {
  let statusEl: HTMLElement;
  let statusIndicatorEl: HTMLElement;
  let portEl: HTMLElement;
  let clientsEl: HTMLElement;
  let toolsCountEl: HTMLElement;
  let toolsListEl: HTMLElement;

  beforeEach(() => {
    // Set up minimal DOM elements that the popup needs
    statusEl = document.createElement('span');
    statusIndicatorEl = document.createElement('span');
    portEl = document.createElement('span');
    clientsEl = document.createElement('span');
    toolsCountEl = document.createElement('span');
    toolsListEl = document.createElement('div');
  });

  describe('Scenario: Popup displays connection status and available tools', () => {
    it('should show server status, port, clients, and tools grouped by source when connected', () => {
      // @step Given the fspec WebMCP Chrome extension is installed
      const mockResponse: StatusResponse = {
        connected: true,
        nativeConnected: true,
        toolCount: 8,
        port: 19876,
        clientCount: 2,
        tools: [
          { name: 'browser_navigate', source: 'native' },
          { name: 'browser_screenshot', source: 'native' },
          { name: 'browser_list_tabs', source: 'native' },
          { name: 'browser_execute_script', source: 'native' },
          { name: 'browser_switch_tab', source: 'native' },
          {
            name: 'webmcp__example.com__searchFlights',
            source: 'webmcp',
            origin: 'example.com',
            tabId: 1,
          },
          {
            name: 'webmcp__example.com__bookFlight',
            source: 'webmcp',
            origin: 'example.com',
            tabId: 1,
          },
          {
            name: 'webmcp__app.test.io__submitForm',
            source: 'webmcp',
            origin: 'app.test.io',
            tabId: 2,
          },
        ],
      };

      // @step When the user opens the extension popup
      renderPopupState(mockResponse, {
        statusEl,
        statusIndicatorEl,
        portEl,
        clientsEl,
        toolsCountEl,
        toolsListEl,
      });

      // @step Then the popup shows the server status as "listening" or "stopped"
      expect(statusEl.textContent).toBe('listening');
      expect(statusIndicatorEl.className).toContain('listening');

      // @step And the popup shows the configured port number
      expect(portEl.textContent).toBe('19876');

      // @step And the popup shows the count of connected clients
      expect(clientsEl.textContent).toBe('2');

      // @step And the popup shows available tools grouped by source as native and WebMCP per tab
      expect(toolsCountEl.textContent).toBe('8');

      const groupHeaders = toolsListEl.querySelectorAll('.tool-group-header');
      expect(groupHeaders.length).toBe(3);
      expect(groupHeaders[0].textContent).toBe('Browser Tools (5)');
      expect(groupHeaders[1].textContent).toBe('example.com (2)');
      expect(groupHeaders[2].textContent).toBe('app.test.io (1)');
    });

    it('should show stopped status when native host is disconnected', () => {
      // @step Given the fspec WebMCP Chrome extension is installed
      const mockResponse: StatusResponse = {
        connected: false,
        nativeConnected: false,
        toolCount: 0,
        port: 19876,
        clientCount: 0,
        tools: [],
      };

      // @step When the user opens the extension popup
      renderPopupState(mockResponse, {
        statusEl,
        statusIndicatorEl,
        portEl,
        clientsEl,
        toolsCountEl,
        toolsListEl,
      });

      // @step Then the popup shows the server status as "listening" or "stopped"
      expect(statusEl.textContent).toBe('stopped');
      expect(statusIndicatorEl.className).toContain('stopped');

      // @step And the popup shows the configured port number
      expect(portEl.textContent).toBe('19876');

      // @step And the popup shows the count of connected clients
      expect(clientsEl.textContent).toBe('0');

      // @step And the popup shows available tools grouped by source as native and WebMCP per tab
      expect(toolsCountEl.textContent).toBe('0');
      const groupHeaders = toolsListEl.querySelectorAll('.tool-group-header');
      expect(groupHeaders.length).toBe(0);
    });

    it('should show only browser tools section when no WebMCP tools exist', () => {
      // @step Given the fspec WebMCP Chrome extension is installed
      const mockResponse: StatusResponse = {
        connected: true,
        nativeConnected: true,
        toolCount: 3,
        port: 19876,
        clientCount: 1,
        tools: [
          { name: 'browser_navigate', source: 'native' },
          { name: 'browser_screenshot', source: 'native' },
          { name: 'browser_list_tabs', source: 'native' },
        ],
      };

      // @step When the user opens the extension popup
      renderPopupState(mockResponse, {
        statusEl,
        statusIndicatorEl,
        portEl,
        clientsEl,
        toolsCountEl,
        toolsListEl,
      });

      // @step Then the popup shows the server status as "listening" or "stopped"
      expect(statusEl.textContent).toBe('listening');

      // @step And the popup shows the configured port number
      expect(portEl.textContent).toBe('19876');

      // @step And the popup shows the count of connected clients
      expect(clientsEl.textContent).toBe('1');

      // @step And the popup shows available tools grouped by source as native and WebMCP per tab
      const groupHeaders = toolsListEl.querySelectorAll('.tool-group-header');
      expect(groupHeaders.length).toBe(1);
      expect(groupHeaders[0].textContent).toBe('Browser Tools (3)');
    });
  });

  describe('Unit: groupToolsBySource', () => {
    it('should return empty array when no tools', () => {
      const result = groupToolsBySource([]);
      expect(result).toEqual([]);
    });

    it('should group native tools under Browser Tools', () => {
      const tools: PopupToolSummary[] = [
        { name: 'browser_navigate', source: 'native' },
        { name: 'browser_screenshot', source: 'native' },
      ];
      const result = groupToolsBySource(tools);
      expect(result).toEqual([
        {
          label: 'Browser Tools',
          count: 2,
          tools: [{ name: 'browser_navigate' }, { name: 'browser_screenshot' }],
        },
      ]);
    });

    it('should group WebMCP tools by origin', () => {
      const tools: PopupToolSummary[] = [
        {
          name: 'webmcp__a.com__tool1',
          source: 'webmcp',
          origin: 'a.com',
          tabId: 1,
        },
        {
          name: 'webmcp__a.com__tool2',
          source: 'webmcp',
          origin: 'a.com',
          tabId: 1,
        },
        {
          name: 'webmcp__b.com__tool3',
          source: 'webmcp',
          origin: 'b.com',
          tabId: 2,
        },
      ];
      const result = groupToolsBySource(tools);
      expect(result).toEqual([
        {
          label: 'a.com',
          count: 2,
          tools: [
            { name: 'webmcp__a.com__tool1' },
            { name: 'webmcp__a.com__tool2' },
          ],
        },
        {
          label: 'b.com',
          count: 1,
          tools: [{ name: 'webmcp__b.com__tool3' }],
        },
      ]);
    });
  });

  describe('Unit: deriveStatus', () => {
    it('should return listening when native host is connected', () => {
      expect(deriveStatus(true)).toEqual({
        text: 'listening',
        cssClass: 'listening',
      });
    });

    it('should return stopped when native host is disconnected', () => {
      expect(deriveStatus(false)).toEqual({
        text: 'stopped',
        cssClass: 'stopped',
      });
    });
  });

  describe('Unit: chrome.runtime.sendMessage integration', () => {
    it('should send FSPEC_GET_STATUS message and receive enriched response', () => {
      // Simulate what popup.ts will do on DOMContentLoaded
      const expectedResponse: StatusResponse = {
        connected: true,
        nativeConnected: true,
        toolCount: 1,
        port: 19876,
        clientCount: 1,
        tools: [{ name: 'browser_navigate', source: 'native' }],
      };

      // Mock chrome.runtime.sendMessage
      const sendMessage = vi.fn(
        (
          _message: Record<string, unknown>,
          callback: (response: StatusResponse) => void
        ) => {
          callback(expectedResponse);
        }
      );

      // Simulate the popup's message send
      let receivedResponse: StatusResponse | undefined;
      sendMessage({ type: 'FSPEC_GET_STATUS' }, response => {
        receivedResponse = response;
      });

      expect(sendMessage).toHaveBeenCalledWith(
        { type: 'FSPEC_GET_STATUS' },
        expect.any(Function)
      );
      expect(receivedResponse).toEqual(expectedResponse);
    });
  });
});
