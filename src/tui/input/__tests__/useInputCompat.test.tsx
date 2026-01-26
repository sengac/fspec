/**
 * Tests for useInputCompat Hook
 *
 * Verifies the compatibility hook works correctly in both modes:
 * 1. With InputManager context (uses priority-based dispatch)
 * 2. Without InputManager context (falls back to raw useInput)
 */

import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { InputManager } from '../InputManager';
import { useInputCompat } from '../useInputCompat';
import { InputPriority } from '../types';

describe('useInputCompat', () => {
  describe('with InputManager context', () => {
    it('should register handler with InputManager', async () => {
      const handler = vi.fn(() => true);

      function TestComponent() {
        useInputCompat({
          id: 'test-handler',
          handler,
          priority: InputPriority.MEDIUM,
          isActive: true,
        });
        return <Text>Test</Text>;
      }

      const { stdin } = render(
        <InputManager>
          <TestComponent />
        </InputManager>
      );

      // Send a key
      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 10));

      // Handler should be called through InputManager
      expect(handler).toHaveBeenCalledWith('a', expect.any(Object));
    });

    it('should respect priority ordering', async () => {
      const calls: string[] = [];

      function HighPriorityComponent() {
        useInputCompat({
          id: 'high',
          handler: () => {
            calls.push('high');
            return true; // Stop propagation
          },
          priority: InputPriority.HIGH,
          isActive: true,
        });
        return null;
      }

      function LowPriorityComponent() {
        useInputCompat({
          id: 'low',
          handler: () => {
            calls.push('low');
            return false;
          },
          priority: InputPriority.LOW,
          isActive: true,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <HighPriorityComponent />
          <LowPriorityComponent />
          <Text>Test</Text>
        </InputManager>
      );

      stdin.write('x');
      await new Promise(resolve => setTimeout(resolve, 10));

      // Only high priority should run (it returns true to stop propagation)
      expect(calls).toEqual(['high']);
    });

    it('should respect isActive flag', async () => {
      const handler = vi.fn(() => true);

      function TestComponent({ active }: { active: boolean }) {
        useInputCompat({
          id: 'conditional',
          handler,
          priority: InputPriority.MEDIUM,
          isActive: active,
        });
        return <Text>Active: {String(active)}</Text>;
      }

      const { stdin, rerender } = render(
        <InputManager>
          <TestComponent active={false} />
        </InputManager>
      );

      // Send key when inactive
      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).not.toHaveBeenCalled();

      // Activate and send key
      rerender(
        <InputManager>
          <TestComponent active={true} />
        </InputManager>
      );

      stdin.write('b');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).toHaveBeenCalled();
    });
  });

  describe('without InputManager context (fallback mode)', () => {
    it('should fall back to raw useInput', async () => {
      const handler = vi.fn(() => true);

      function TestComponent() {
        useInputCompat({
          id: 'standalone',
          handler,
          priority: InputPriority.MEDIUM,
          isActive: true,
        });
        return <Text>Standalone</Text>;
      }

      // Render WITHOUT InputManager
      const { stdin } = render(<TestComponent />);

      stdin.write('z');
      await new Promise(resolve => setTimeout(resolve, 10));

      // Handler should still be called via fallback useInput
      expect(handler).toHaveBeenCalledWith('z', expect.any(Object));
    });

    it('should respect isActive in fallback mode', async () => {
      const handler = vi.fn(() => true);

      function TestComponent({ active }: { active: boolean }) {
        useInputCompat({
          id: 'standalone-conditional',
          handler,
          priority: InputPriority.MEDIUM,
          isActive: active,
        });
        return <Text>Active: {String(active)}</Text>;
      }

      // Render WITHOUT InputManager
      const { stdin, rerender } = render(<TestComponent active={false} />);

      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).not.toHaveBeenCalled();

      rerender(<TestComponent active={true} />);
      stdin.write('b');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).toHaveBeenCalled();
    });
  });

  describe('isActive as function', () => {
    it('should support isActive as a function', async () => {
      const handler = vi.fn(() => true);
      let shouldBeActive = false;

      function TestComponent() {
        useInputCompat({
          id: 'function-active',
          handler,
          priority: InputPriority.MEDIUM,
          isActive: () => shouldBeActive,
        });
        return <Text>Test</Text>;
      }

      const { stdin } = render(
        <InputManager>
          <TestComponent />
        </InputManager>
      );

      // Inactive
      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).not.toHaveBeenCalled();

      // Make active
      shouldBeActive = true;
      stdin.write('b');
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(handler).toHaveBeenCalled();
    });
  });
});
