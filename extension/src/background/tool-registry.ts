/**
 * fspec WebMCP Extension - Tool Registry
 *
 * Manages the registry of available tools (both native browser control
 * tools and WebMCP tools discovered from web pages).
 *
 * Implemented by: EXT-004
 */

import type { ToolRegistryEntry } from '../types';

export interface ToolRegistryAPI {
  register: (tool: ToolRegistryEntry) => void;
  unregister: (toolName: string) => void;
  getAll: () => ToolRegistryEntry[];
  getByTab: (tabId: number) => ToolRegistryEntry[];
  getByName: (name: string) => ToolRegistryEntry | undefined;
  clear: () => void;
  size: () => number;
}

export function createToolRegistry(): ToolRegistryAPI {
  const tools = new Map<string, ToolRegistryEntry>();

  return {
    register(tool: ToolRegistryEntry): void {
      tools.set(tool.name, tool);
    },

    unregister(toolName: string): void {
      tools.delete(toolName);
    },

    getAll(): ToolRegistryEntry[] {
      return Array.from(tools.values());
    },

    getByTab(tabId: number): ToolRegistryEntry[] {
      return Array.from(tools.values()).filter((t) => t.tabId === tabId);
    },

    getByName(name: string): ToolRegistryEntry | undefined {
      return tools.get(name);
    },

    clear(): void {
      tools.clear();
    },

    size(): number {
      return tools.size;
    },
  };
}
