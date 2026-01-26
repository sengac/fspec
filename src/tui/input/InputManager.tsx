/**
 * Input Manager
 *
 * The central input handling component that owns the SINGLE useInput hook
 * for the entire application (or a section of it).
 *
 * Design:
 * - Wraps children with InputContext provider
 * - Owns the only useInput hook
 * - Dispatches input events to registered handlers in priority order
 * - Stops propagation when a handler returns true
 *
 * Usage:
 * ```tsx
 * <InputManager>
 *   <YourApp />
 * </InputManager>
 * ```
 */

import React, { useMemo, useRef, type ReactNode } from 'react';
import { useInput } from 'ink';
import { InputContext } from './InputContext.js';
import { createInputHandlerRegistry, type InputHandlerRegistry } from './InputHandlerRegistry.js';
import type { InputManagerAPI } from './types.js';

interface InputManagerProps {
  /** Child components */
  children: ReactNode;

  /**
   * Whether input handling is active.
   * Set to false to disable all input handling (e.g., during exit).
   * @default true
   */
  isActive?: boolean;
}

/**
 * InputManager component.
 * Provides centralized input handling for all descendants.
 */
export function InputManager({
  children,
  isActive = true,
}: InputManagerProps): React.JSX.Element {
  // Use ref to maintain stable registry across renders
  const registryRef = useRef<InputHandlerRegistry | null>(null);

  // Initialize registry lazily
  if (!registryRef.current) {
    registryRef.current = createInputHandlerRegistry();
  }

  const registry = registryRef.current;

  // Create stable API object for context
  const api: InputManagerAPI = useMemo(
    () => ({
      register: registry.register,
      unregister: registry.unregister,
      has: registry.has,
      getHandlerIds: registry.getHandlerIds,
    }),
    [registry]
  );

  // The SINGLE useInput hook for the entire subtree
  useInput(
    (input, key) => {
      const handlers = registry.getOrderedHandlers();

      // Dispatch to handlers in priority order
      for (const handler of handlers) {
        // Skip inactive handlers
        if (!handler.isActive()) {
          continue;
        }

        const handled = handler.handler(input, key);

        if (handled === true) {
          // Handler consumed the input, stop propagation
          return;
        }
      }
    },
    { isActive }
  );

  return <InputContext.Provider value={api}>{children}</InputContext.Provider>;
}
