/**
 * Input System Types
 *
 * Core type definitions for the centralized input handling architecture.
 * This replaces the scattered useInput hooks with a single-owner pattern.
 *
 * Design principles:
 * - Single useInput owner (InputManager) dispatches to registered handlers
 * - Handlers are called in priority order (highest first)
 * - Handlers return true to stop propagation to lower-priority handlers
 * - Handlers can be dynamically enabled/disabled via isActive function
 */

import type { Key } from 'ink';

/**
 * Priority levels for input handlers.
 * Higher numbers run first.
 *
 * Guidelines:
 * - CRITICAL: Modal dialogs that must capture all input
 * - HIGH: Overlays like slash command palette
 * - MEDIUM: Main text input
 * - LOW: Global shortcuts, mode-specific handlers
 * - BACKGROUND: Passive handlers (scroll, navigation)
 */
export const InputPriority = {
  /** Modal dialogs that block all other input */
  CRITICAL: 1000,
  /** Overlays and palettes that capture most input */
  HIGH: 800,
  /** Primary input area (text editor) */
  MEDIUM: 500,
  /** Global shortcuts and mode handlers */
  LOW: 200,
  /** Background handlers (scroll, passive navigation) */
  BACKGROUND: 100,
} as const;

export type InputPriorityLevel =
  (typeof InputPriority)[keyof typeof InputPriority];

/**
 * Input handler function signature.
 * Matches Ink's useInput callback signature.
 *
 * @param input - Raw input string (may be empty for special keys)
 * @param key - Parsed key object with flags for special keys
 * @returns true if the handler consumed the input (stops propagation)
 */
export type InputHandlerFn = (input: string, key: Key) => boolean | void;

/**
 * Configuration for registering an input handler.
 */
export interface InputHandlerConfig {
  /** Unique identifier for this handler */
  id: string;

  /** Handler function */
  handler: InputHandlerFn;

  /** Priority level (higher runs first) */
  priority: number;

  /**
   * Dynamic activation check. Called before each input event.
   * Return false to skip this handler for the current event.
   * Defaults to always active if not provided.
   */
  isActive?: () => boolean;

  /**
   * Optional description for debugging.
   */
  description?: string;
}

/**
 * Registered handler with all metadata.
 */
export interface RegisteredHandler {
  id: string;
  handler: InputHandlerFn;
  priority: number;
  isActive: () => boolean;
  description: string;
  registeredAt: number; // For stable sort when priorities are equal
}

/**
 * Input manager interface exposed via context.
 */
export interface InputManagerAPI {
  /**
   * Register a new input handler.
   * @returns Unregister function
   */
  register: (config: InputHandlerConfig) => () => void;

  /**
   * Unregister a handler by ID.
   */
  unregister: (id: string) => void;

  /**
   * Check if a handler is registered.
   */
  has: (id: string) => boolean;

  /**
   * Get list of registered handler IDs (for debugging).
   */
  getHandlerIds: () => string[];

  /**
   * Enable debug logging of input dispatch.
   */
  setDebugMode: (enabled: boolean) => void;
}
