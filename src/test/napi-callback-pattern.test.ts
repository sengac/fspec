// Vitest tests for NAPI callback pattern
// These tests prove the callback pattern works with proper test assertions

import { describe, it, expect } from 'vitest';
import { testCallback } from '@sengac/codelet-napi';

describe('NAPI Callback Pattern', () => {
  it('should call TypeScript callback from Rust and return result', () => {
    // Arrange
    const input = 'Hello from vitest';
    const expectedOutput = 'Processed: Hello from vitest';

    // Act - call NAPI function with TypeScript callback
    const result = testCallback(input, (receivedInput: string) => {
      // Assert the callback receives the correct input
      expect(receivedInput).toBe(input);

      // Transform and return result
      return `Processed: ${receivedInput}`;
    });

    // Assert - verify the result flows back correctly
    expect(result).toBe(expectedOutput);
  });

  it('should handle different input types in callback', () => {
    // Test with empty string
    const result1 = testCallback('', (input: string) => {
      expect(input).toBe('');
      return `Empty: ${input}`;
    });
    expect(result1).toBe('Empty: ');

    // Test with special characters
    const result2 = testCallback('Hello 世界! 🌍', (input: string) => {
      expect(input).toBe('Hello 世界! 🌍');
      return `Special: ${input}`;
    });
    expect(result2).toBe('Special: Hello 世界! 🌍');
  });

  it('should maintain callback execution order', () => {
    const executionOrder: string[] = [];

    const result = testCallback('test', (input: string) => {
      executionOrder.push('callback-start');
      executionOrder.push(`received-${input}`);
      executionOrder.push('callback-end');
      return 'callback-result';
    });

    expect(result).toBe('callback-result');
    expect(executionOrder).toEqual([
      'callback-start',
      'received-test',
      'callback-end',
    ]);
  });

  it('should handle complex callback logic', () => {
    const result = testCallback('complex test', (input: string) => {
      // Complex transformation logic
      const words = input.split(' ');
      const transformed = words
        .map(word => word.charAt(0).toUpperCase() + word.slice(1))
        .join('-');

      return `Transformed: ${transformed}`;
    });

    expect(result).toBe('Transformed: Complex-Test');
  });

  it('should demonstrate real fspec command simulation', () => {
    // This simulates what a real fspec command callback would look like
    const command = 'list-work-units';

    const result = testCallback(command, (cmd: string) => {
      // Simulate what the real TypeScript callback will do:
      // 1. Import command module
      // 2. Execute command function
      // 3. Return JSON result

      if (cmd === 'list-work-units') {
        const mockWorkUnits = [
          { id: 'CODE-001', title: 'Test Story', status: 'done' },
          { id: 'CODE-002', title: 'Another Story', status: 'implementing' },
        ];

        return JSON.stringify({
          success: true,
          data: mockWorkUnits,
          command: cmd,
        });
      }

      return JSON.stringify({ success: false, error: 'Unknown command' });
    });

    const parsed = JSON.parse(result);
    expect(parsed.success).toBe(true);
    expect(parsed.command).toBe('list-work-units');
    expect(parsed.data).toHaveLength(2);
    expect(parsed.data[0].id).toBe('CODE-001');
  });
});
