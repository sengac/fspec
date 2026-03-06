/**
 * fspec Browser Agent - Popup Rendering Utilities
 *
 * Pure functions for popup state rendering, extracted for testability.
 * Used by popup.ts and tested directly in popup.test.ts.
 *
 * EXT-008: Popup UI rendering logic
 */

import type { PopupToolSummary } from '../types';

/** A group of tools to display in the popup (either native or per-origin) */
export interface ToolGroup {
  label: string;
  count: number;
  tools: Array<{ name: string }>;
}

/**
 * Groups tools by source: native browser tools in one group,
 * WebMCP tools grouped by their origin hostname.
 */
export function groupToolsBySource(tools: PopupToolSummary[]): ToolGroup[] {
  const groups: ToolGroup[] = [];

  // Native browser tools
  const nativeTools = tools.filter(t => t.source === 'native');
  if (nativeTools.length > 0) {
    groups.push({
      label: 'Browser Tools',
      count: nativeTools.length,
      tools: nativeTools.map(t => ({ name: t.name })),
    });
  }

  // WebMCP tools grouped by origin
  const webmcpTools = tools.filter(t => t.source === 'webmcp');
  const byOrigin = new Map<string, Array<{ name: string }>>();
  for (const tool of webmcpTools) {
    const origin = tool.origin ?? 'unknown';
    const existing = byOrigin.get(origin);
    if (existing) {
      existing.push({ name: tool.name });
    } else {
      byOrigin.set(origin, [{ name: tool.name }]);
    }
  }
  for (const [origin, originTools] of byOrigin) {
    groups.push({
      label: origin,
      count: originTools.length,
      tools: originTools,
    });
  }

  return groups;
}

/** Derives the display status text and CSS class from native connection state */
export function deriveStatus(nativeConnected: boolean): {
  text: string;
  cssClass: string;
} {
  return nativeConnected
    ? { text: 'listening', cssClass: 'listening' }
    : { text: 'stopped', cssClass: 'stopped' };
}
