/**
 * Input Context
 *
 * React context for the input handling system.
 * Provides access to the InputManagerAPI for registering handlers.
 */

import { createContext, useContext } from 'react';
import type { InputManagerAPI } from './types.js';

/**
 * Context for the input manager.
 * null when not inside an InputManager provider.
 */
export const InputContext = createContext<InputManagerAPI | null>(null);

/**
 * Hook to access the input manager.
 * Throws if used outside an InputManager provider.
 */
export function useInputManager(): InputManagerAPI {
  const context = useContext(InputContext);
  if (!context) {
    throw new Error(
      'useInputManager must be used within an InputManager. ' +
        'Make sure your component is wrapped with <InputManager>.'
    );
  }
  return context;
}

/**
 * Hook to optionally access the input manager.
 * Returns null if not inside an InputManager provider.
 * Useful for components that can work with or without centralized input.
 */
export function useOptionalInputManager(): InputManagerAPI | null {
  return useContext(InputContext);
}
