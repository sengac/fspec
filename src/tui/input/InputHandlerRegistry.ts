/**
 * Input Handler Registry
 *
 * Manages the collection of input handlers with proper ordering by priority.
 * Handlers with higher priority values run first.
 * When priorities are equal, earlier-registered handlers run first (stable sort).
 *
 * This is a pure data structure - no React dependencies.
 */

import type {
  InputHandlerConfig,
  RegisteredHandler,
  InputManagerAPI,
} from './types.js';

/**
 * Creates a new input handler registry.
 * This is a factory function to allow multiple independent registries if needed.
 */
export function createInputHandlerRegistry(): InputManagerAPI & {
  /** Get handlers sorted by priority (highest first) */
  getOrderedHandlers: () => RegisteredHandler[];
  /** Get all handlers (for debugging) */
  getAllHandlers: () => Map<string, RegisteredHandler>;
} {
  const handlers = new Map<string, RegisteredHandler>();
  let registrationCounter = 0;

  /**
   * Get handlers sorted by priority (highest first).
   * Stable sort: equal priorities preserve registration order.
   */
  function getOrderedHandlers(): RegisteredHandler[] {
    return Array.from(handlers.values()).sort((a, b) => {
      // Higher priority first
      if (b.priority !== a.priority) {
        return b.priority - a.priority;
      }
      // Equal priority: earlier registration first
      return a.registeredAt - b.registeredAt;
    });
  }

  /**
   * Register a new input handler.
   */
  function register(config: InputHandlerConfig): () => void {
    const { id, handler, priority, isActive, description } = config;

    const registered: RegisteredHandler = {
      id,
      handler,
      priority,
      isActive: isActive ?? (() => true),
      description: description ?? id,
      registeredAt: registrationCounter++,
    };

    handlers.set(id, registered);

    // Return unregister function
    return () => unregister(id);
  }

  /**
   * Unregister a handler by ID.
   */
  function unregister(id: string): void {
    handlers.delete(id);
  }

  /**
   * Check if a handler is registered.
   */
  function has(id: string): boolean {
    return handlers.has(id);
  }

  /**
   * Get list of registered handler IDs.
   */
  function getHandlerIds(): string[] {
    return Array.from(handlers.keys());
  }

  /**
   * Get all handlers (for debugging).
   */
  function getAllHandlers(): Map<string, RegisteredHandler> {
    return new Map(handlers);
  }

  return {
    register,
    unregister,
    has,
    getHandlerIds,
    getOrderedHandlers,
    getAllHandlers,
  };
}

/**
 * Type for the full registry (includes internal methods).
 */
export type InputHandlerRegistry = ReturnType<
  typeof createInputHandlerRegistry
>;
