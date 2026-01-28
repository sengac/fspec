/**
 * Input Handling System
 *
 * A centralized, priority-based input handling architecture for terminal UI.
 *
 * Problem solved:
 * - Multiple useInput hooks fighting for input events
 * - No way to stop propagation in Ink
 * - Race conditions with async state updates
 *
 * Solution:
 * - Single useInput owner (InputManager)
 * - Priority-based dispatch to registered handlers
 * - Handlers return true to stop propagation
 *
 * Usage:
 * ```tsx
 * // Wrap your app with InputProvider (or InputManager)
 * <InputProvider>
 *   <App />
 * </InputProvider>
 *
 * // In components, use useInputCompat (recommended)
 * useInputCompat({
 *   id: 'my-dialog',
 *   priority: InputPriority.CRITICAL,
 *   handler: (input, key) => {
 *     if (key.escape) { onClose(); return true; }
 *     return false;
 *   },
 *   isActive: isOpen,
 * });
 * ```
 *
 * Note: useInputCompat works both inside and outside InputProvider context,
 * falling back to Ink's useInput when no context is available.
 */

// Types
export {
  InputPriority,
  type InputPriorityLevel,
  type InputHandlerFn,
  type InputHandlerConfig,
  type RegisteredHandler,
  type InputManagerAPI,
} from './types';

// Registry (for advanced use cases)
export {
  createInputHandlerRegistry,
  type InputHandlerRegistry,
} from './InputHandlerRegistry';

// Context
export {
  InputContext,
  useInputManager,
  useOptionalInputManager,
} from './InputContext';

// Components
export { InputManager } from './InputManager';
export { InputProvider } from '../components/InputProvider';

// Hooks
export {
  useInputHandler,
  type UseInputHandlerOptions,
} from './useInputHandler';

export { useInputCompat, type UseInputCompatOptions } from './useInputCompat';
