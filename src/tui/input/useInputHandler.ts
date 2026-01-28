/**
 * useInputHandler Hook
 *
 * A React hook for registering input handlers with the centralized InputManager.
 * Automatically handles registration on mount and cleanup on unmount.
 *
 * Usage:
 * ```tsx
 * useInputHandler({
 *   id: 'my-dialog',
 *   priority: InputPriority.CRITICAL,
 *   handler: (input, key) => {
 *     if (key.escape) {
 *       onClose();
 *       return true; // Consumed
 *     }
 *     return false; // Not consumed
 *   },
 *   isActive: () => isDialogOpen,
 * });
 * ```
 */

import { useEffect, useRef, useCallback } from 'react';
import { useInputManager, useOptionalInputManager } from './InputContext';
import type { InputHandlerConfig, InputHandlerFn } from './types';

/**
 * Options for useInputHandler hook.
 */
export interface UseInputHandlerOptions {
  /** Unique identifier for this handler */
  id: string;

  /** Handler function */
  handler: InputHandlerFn;

  /** Priority level (higher runs first). Use InputPriority constants. */
  priority: number;

  /**
   * Whether this handler is currently active.
   * Can be a boolean or a function that returns boolean.
   * When false, the handler is skipped during dispatch.
   * @default true
   */
  isActive?: boolean | (() => boolean);

  /**
   * Optional description for debugging.
   */
  description?: string;

  /**
   * Whether to throw if InputManager is not available.
   * Set to false for components that can work without centralized input.
   * @default true
   */
  required?: boolean;
}

/**
 * Hook to register an input handler with the InputManager.
 *
 * The handler is automatically registered on mount and unregistered on unmount.
 * The handler function is kept up-to-date via a ref, so you don't need to
 * memoize it (though you can for performance).
 */
export function useInputHandler(options: UseInputHandlerOptions): void {
  const {
    id,
    handler,
    priority,
    isActive = true,
    description,
    required = true,
  } = options;

  // Get the input manager (throws if required and not available)
  const manager = required ? useInputManager() : useOptionalInputManager();

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

  // Register on mount, unregister on unmount
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
}
