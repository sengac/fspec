/**
 * InputProvider
 *
 * A wrapper component that provides the InputManager context.
 * Use this at the top level of your TUI application to enable
 * centralized input handling.
 *
 * Usage:
 * ```tsx
 * <InputProvider>
 *   <YourApp />
 * </InputProvider>
 * ```
 */

import React, { type ReactNode } from 'react';
import { InputManager } from '../input/InputManager.js';

interface InputProviderProps {
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
 * InputProvider component.
 * Wraps children with InputManager for centralized input handling.
 */
export function InputProvider({
  children,
  isActive = true,
  debug = false,
}: InputProviderProps): React.JSX.Element {
  return (
    <InputManager isActive={isActive} debug={debug}>
      {children}
    </InputManager>
  );
}
