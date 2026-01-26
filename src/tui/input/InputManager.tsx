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

  /**
   * Enable debug logging of input dispatch.
   * @default false
   */
  debug?: boolean;
}

/**
 * InputManager component.
 * Provides centralized input handling for all descendants.
 */
export function InputManager({
  children,
  isActive = true,
  debug = false,
}: InputManagerProps): React.JSX.Element {
  // Use ref to maintain stable registry across renders
  const registryRef = useRef<InputHandlerRegistry | null>(null);

  // Initialize registry lazily
  if (!registryRef.current) {
    registryRef.current = createInputHandlerRegistry();
    if (debug) {
      registryRef.current.setDebugMode(true);
    }
  }

  const registry = registryRef.current;

  // Update debug mode if it changes
  React.useEffect(() => {
    registry.setDebugMode(debug);
  }, [debug, registry]);

  // Create stable API object for context
  const api: InputManagerAPI = useMemo(
    () => ({
      register: registry.register,
      unregister: registry.unregister,
      has: registry.has,
      getHandlerIds: registry.getHandlerIds,
      setDebugMode: registry.setDebugMode,
    }),
    [registry]
  );

  // The SINGLE useInput hook for the entire subtree
  useInput(
    (input, key) => {
      const handlers = registry.getOrderedHandlers();

      if (debug) {
        // eslint-disable-next-line no-console
        console.log(
          `[InputManager] Dispatching input="${input}" key=${JSON.stringify({
            return: key.return,
            escape: key.escape,
            tab: key.tab,
            upArrow: key.upArrow,
            downArrow: key.downArrow,
            ctrl: key.ctrl,
            meta: key.meta,
          })} to ${handlers.length} handlers`
        );
      }

      // Dispatch to handlers in priority order
      for (const handler of handlers) {
        // Skip inactive handlers
        if (!handler.isActive()) {
          if (debug) {
            // eslint-disable-next-line no-console
            console.log(`[InputManager] Skipping inactive handler '${handler.id}'`);
          }
          continue;
        }

        try {
          const handled = handler.handler(input, key);

          if (debug) {
            // eslint-disable-next-line no-console
            console.log(
              `[InputManager] Handler '${handler.id}' returned ${handled ? 'true (stopping)' : 'false/void (continuing)'}`
            );
          }

          if (handled === true) {
            // Handler consumed the input, stop propagation
            return;
          }
        } catch (error) {
          // eslint-disable-next-line no-console
          console.error(`[InputManager] Error in handler '${handler.id}':`, error);
        }
      }

      if (debug && handlers.length === 0) {
        // eslint-disable-next-line no-console
        console.log('[InputManager] No handlers registered');
      }
    },
    { isActive }
  );

  return <InputContext.Provider value={api}>{children}</InputContext.Provider>;
}
