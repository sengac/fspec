/**
 * Tests for InputHandlerRegistry
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { createInputHandlerRegistry } from '../InputHandlerRegistry';
import { InputPriority } from '../types';
import type { Key } from 'ink';

// Helper to create a mock Key object
function mockKey(overrides: Partial<Key> = {}): Key {
  return {
    upArrow: false,
    downArrow: false,
    leftArrow: false,
    rightArrow: false,
    pageDown: false,
    pageUp: false,
    return: false,
    escape: false,
    ctrl: false,
    shift: false,
    tab: false,
    backspace: false,
    delete: false,
    meta: false,
    ...overrides,
  };
}

describe('InputHandlerRegistry', () => {
  describe('createInputHandlerRegistry', () => {
    it('should create a registry with all required methods', () => {
      const registry = createInputHandlerRegistry();

      expect(registry.register).toBeInstanceOf(Function);
      expect(registry.unregister).toBeInstanceOf(Function);
      expect(registry.has).toBeInstanceOf(Function);
      expect(registry.getHandlerIds).toBeInstanceOf(Function);
      expect(registry.setDebugMode).toBeInstanceOf(Function);
      expect(registry.getOrderedHandlers).toBeInstanceOf(Function);
    });

    it('should start with no handlers', () => {
      const registry = createInputHandlerRegistry();

      expect(registry.getHandlerIds()).toEqual([]);
      expect(registry.getOrderedHandlers()).toEqual([]);
    });
  });

  describe('register', () => {
    it('should register a handler', () => {
      const registry = createInputHandlerRegistry();
      const handler = () => false;

      registry.register({
        id: 'test-handler',
        handler,
        priority: InputPriority.MEDIUM,
      });

      expect(registry.has('test-handler')).toBe(true);
      expect(registry.getHandlerIds()).toEqual(['test-handler']);
    });

    it('should return an unregister function', () => {
      const registry = createInputHandlerRegistry();
      const handler = () => false;

      const unregister = registry.register({
        id: 'test-handler',
        handler,
        priority: InputPriority.MEDIUM,
      });

      expect(registry.has('test-handler')).toBe(true);

      unregister();

      expect(registry.has('test-handler')).toBe(false);
    });

    it('should replace existing handler with same id', () => {
      const registry = createInputHandlerRegistry();
      const handler1 = () => false;
      const handler2 = () => true;

      registry.register({
        id: 'test-handler',
        handler: handler1,
        priority: InputPriority.LOW,
      });

      registry.register({
        id: 'test-handler',
        handler: handler2,
        priority: InputPriority.HIGH,
      });

      const handlers = registry.getOrderedHandlers();
      expect(handlers).toHaveLength(1);
      expect(handlers[0].priority).toBe(InputPriority.HIGH);
    });
  });

  describe('unregister', () => {
    it('should unregister a handler by id', () => {
      const registry = createInputHandlerRegistry();

      registry.register({
        id: 'test-handler',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      expect(registry.has('test-handler')).toBe(true);

      registry.unregister('test-handler');

      expect(registry.has('test-handler')).toBe(false);
    });

    it('should not throw when unregistering non-existent handler', () => {
      const registry = createInputHandlerRegistry();

      expect(() => registry.unregister('non-existent')).not.toThrow();
    });
  });

  describe('getOrderedHandlers', () => {
    it('should return handlers sorted by priority (highest first)', () => {
      const registry = createInputHandlerRegistry();

      registry.register({
        id: 'low',
        handler: () => false,
        priority: InputPriority.LOW,
      });

      registry.register({
        id: 'high',
        handler: () => false,
        priority: InputPriority.HIGH,
      });

      registry.register({
        id: 'medium',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      const handlers = registry.getOrderedHandlers();

      expect(handlers.map(h => h.id)).toEqual(['high', 'medium', 'low']);
    });

    it('should use stable sort for equal priorities (earlier registered first)', () => {
      const registry = createInputHandlerRegistry();

      registry.register({
        id: 'first',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      registry.register({
        id: 'second',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      registry.register({
        id: 'third',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      const handlers = registry.getOrderedHandlers();

      expect(handlers.map(h => h.id)).toEqual(['first', 'second', 'third']);
    });

    it('should include isActive function defaulting to true', () => {
      const registry = createInputHandlerRegistry();

      registry.register({
        id: 'test',
        handler: () => false,
        priority: InputPriority.MEDIUM,
      });

      const handlers = registry.getOrderedHandlers();
      expect(handlers[0].isActive()).toBe(true);
    });

    it('should use provided isActive function', () => {
      const registry = createInputHandlerRegistry();
      let isActive = false;

      registry.register({
        id: 'test',
        handler: () => false,
        priority: InputPriority.MEDIUM,
        isActive: () => isActive,
      });

      const handlers = registry.getOrderedHandlers();
      expect(handlers[0].isActive()).toBe(false);

      isActive = true;
      expect(handlers[0].isActive()).toBe(true);
    });
  });

  describe('handler execution', () => {
    it('should call handlers in priority order until one returns true', () => {
      const registry = createInputHandlerRegistry();
      const calls: string[] = [];

      registry.register({
        id: 'low',
        handler: () => {
          calls.push('low');
          return false;
        },
        priority: InputPriority.LOW,
      });

      registry.register({
        id: 'high',
        handler: () => {
          calls.push('high');
          return true; // Stop propagation
        },
        priority: InputPriority.HIGH,
      });

      registry.register({
        id: 'medium',
        handler: () => {
          calls.push('medium');
          return false;
        },
        priority: InputPriority.MEDIUM,
      });

      // Simulate dispatch
      const handlers = registry.getOrderedHandlers();
      for (const handler of handlers) {
        if (handler.isActive()) {
          const result = handler.handler('', mockKey());
          if (result === true) break;
        }
      }

      // High runs first and stops propagation
      expect(calls).toEqual(['high']);
    });

    it('should skip inactive handlers', () => {
      const registry = createInputHandlerRegistry();
      const calls: string[] = [];
      let criticalActive = false;

      registry.register({
        id: 'critical',
        handler: () => {
          calls.push('critical');
          return true;
        },
        priority: InputPriority.CRITICAL,
        isActive: () => criticalActive,
      });

      registry.register({
        id: 'low',
        handler: () => {
          calls.push('low');
          return false;
        },
        priority: InputPriority.LOW,
      });

      // Simulate dispatch
      const handlers = registry.getOrderedHandlers();
      for (const handler of handlers) {
        if (handler.isActive()) {
          const result = handler.handler('', mockKey());
          if (result === true) break;
        }
      }

      // Critical is inactive, so low runs
      expect(calls).toEqual(['low']);

      // Now make critical active
      calls.length = 0;
      criticalActive = true;

      const handlers2 = registry.getOrderedHandlers();
      for (const handler of handlers2) {
        if (handler.isActive()) {
          const result = handler.handler('', mockKey());
          if (result === true) break;
        }
      }

      // Critical is now active and stops propagation
      expect(calls).toEqual(['critical']);
    });
  });
});
