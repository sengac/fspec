/**
 * useInputCompat Hook
 *
 * A compatibility hook that provides a smooth migration path from
 * Ink's useInput to the centralized InputManager system.
 *
 * Behavior:
 * - When inside an InputManager context: uses useInputHandler (priority-based)
 * - When NOT inside InputManager: falls back to Ink's useInput (legacy)
 *
 * This allows components to be migrated one at a time while maintaining
 * backward compatibility with existing code and tests.
 *
 * Usage:
 * ```tsx
 * useInputCompat({
 *   id: 'my-component',
 *   priority: InputPriority.CRITICAL,
 *   handler: (input, key) => {
 *     if (key.escape) { onClose(); return true; }
 *     return false;
 *   },
 *   isActive: isVisible,
 * });
 * ```
 */

import { useEffect, useRef, useCallback } from 'react';
import { useInput, type Key } from 'ink';
import { useOptionalInputManager } from './InputContext';
import type { InputHandlerConfig, InputHandlerFn } from './types';

/**
 * Options for useInputCompat hook.
 */
export interface UseInputCompatOptions {
  /** Unique identifier for this handler (used with InputManager) */
  id: string;

  /** Handler function */
  handler: InputHandlerFn;

  /** Priority level (higher runs first). Used only with InputManager. */
  priority: number;

  /**
   * Whether this handler is currently active.
   * Can be a boolean or a function that returns boolean.
   * @default true
   */
  isActive?: boolean | (() => boolean);

  /**
   * Optional description for debugging.
   */
  description?: string;
}

/**
 * Compatibility hook for input handling.
 *
 * Uses InputManager when available, falls back to Ink's useInput otherwise.
 * This enables gradual migration to the centralized input system.
 */
export function useInputCompat(options: UseInputCompatOptions): void {
  const { id, handler, priority, isActive = true, description } = options;

  // Try to get InputManager context (returns null if not available)
  const manager = useOptionalInputManager();

  // Keep handler ref up to date to avoid stale closures
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  // Keep isActive ref up to date
  const isActiveRef = useRef(isActive);
  isActiveRef.current = isActive;

  // Stable wrapper function that calls the current handler
  const stableHandler = useCallback<InputHandlerFn>((input, key) => {
    return handlerRef.current(input, key);
  }, []);

  // Stable isActive function that checks the current value
  const stableIsActive = useCallback((): boolean => {
    const current = isActiveRef.current;
    return typeof current === 'function' ? current() : current;
  }, []);

  // If InputManager is available, register with it
  useEffect(() => {
    if (!manager) {
      return;
    }

    const config: InputHandlerConfig = {
      id,
      handler: stableHandler,
      priority,
      isActive: stableIsActive,
      description,
    };

    const unregister = manager.register(config);

    return () => {
      unregister();
    };
  }, [manager, id, priority, stableHandler, stableIsActive, description]);

  // Compute isActive for Ink's useInput
  const isActiveValue = typeof isActive === 'function' ? isActive() : isActive;

  // If InputManager is NOT available, use Ink's useInput as fallback
  // The handler is wrapped to ignore the return value (useInput doesn't use it)
  useInput(
    (input: string, key: Key) => {
      stableHandler(input, key);
    },
    {
      // Only use Ink's useInput when InputManager is NOT available
      // This prevents double-handling when InputManager IS available
      isActive: !manager && isActiveValue,
    }
  );
}
