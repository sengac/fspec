/**
 * Unit Tests: useCompaction Hook
 *
 * Tests the REAL useCompaction hook behavior using React test components.
 *
 * CMPCT-034: The hook no longer tracks display state (isActive, progress, trigger,
 * sessionId). Rust is the source of truth for compaction status — useRustSessionState
 * reads isCompacting/compactionProgress from Rust. This hook now only manages:
 * - Manual compaction operations (performManualCompaction)
 * - Retry dialog state (retryState, handleRetryOption, clearRetryState)
 */

import React, { useEffect } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import {
  useCompaction,
  type CompactionHookReturn,
} from '../useCompaction';

// Mock NAPI functions - only mock what's needed
vi.mock('@sengac/codelet-napi', () => ({
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 10000,
    compactedTokens: 3000,
    compressionRatio: 70,
    turnsSummarized: 5,
    turnsKept: 2,
  }),
}));

// Store hook state for assertions
let hookState: CompactionHookReturn | null = null;

/**
 * Test component that exposes hook state
 * CMPCT-034: No longer renders display state (isActive, trigger, etc.)
 */
function TestComponent({ onMount }: { onMount?: (hook: CompactionHookReturn) => void }) {
  const compaction = useCompaction();
  hookState = compaction;

  useEffect(() => {
    if (onMount) {
      onMount(compaction);
    }
  }, []);

  return (
    <Text>
      retryVisible:{String(compaction.retryState.isVisible)}|
      retryError:{compaction.retryState.error || 'none'}
    </Text>
  );
}

describe('useCompaction Hook (CMPCT-034)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hookState = null;
  });

  afterEach(() => {
    hookState = null;
  });

  it('returns the expected shape without display state', () => {
    const { unmount } = render(<TestComponent />);

    expect(hookState).not.toBeNull();
    // CMPCT-034: Hook should only have operations + retry state
    expect(hookState).toHaveProperty('performManualCompaction');
    expect(hookState).toHaveProperty('retryState');
    expect(hookState).toHaveProperty('clearRetryState');
    expect(hookState).toHaveProperty('handleRetryOption');

    // CMPCT-034: Hook should NOT have display state
    expect(hookState).not.toHaveProperty('state');
    expect(hookState).not.toHaveProperty('startCompaction');
    expect(hookState).not.toHaveProperty('endCompaction');
    expect(hookState).not.toHaveProperty('updateProgress');

    unmount();
  });

  it('initial retry state is not visible', () => {
    const { unmount } = render(<TestComponent />);

    expect(hookState!.retryState.isVisible).toBe(false);
    expect(hookState!.retryState.error).toBe('');
    expect(hookState!.retryState.retryCount).toBe(0);

    unmount();
  });

  it('performManualCompaction calls sessionCompact and returns result', async () => {
    const { sessionCompact } = await import('@sengac/codelet-napi');

    const { unmount } = render(<TestComponent />);

    const result = await hookState!.performManualCompaction('test-session');

    expect(sessionCompact).toHaveBeenCalledWith('test-session');
    expect(result).toEqual({
      originalTokens: 10000,
      compactedTokens: 3000,
      compressionRatio: 70,
      turnsSummarized: 5,
      turnsKept: 2,
    });

    unmount();
  });

  it('handleRetryOption clears retry dialog on cancel', () => {
    const { unmount } = render(
      <TestComponent
        onMount={(hook) => {
          // Simulate a failure to set retry state
          hook.handleRetryOption('cancel');
        }}
      />
    );

    expect(hookState!.retryState.isVisible).toBe(false);

    unmount();
  });

  it('handleRetryOption clears retry dialog on continue', () => {
    const { unmount } = render(
      <TestComponent
        onMount={(hook) => {
          hook.handleRetryOption('continue');
        }}
      />
    );

    expect(hookState!.retryState.isVisible).toBe(false);

    unmount();
  });

  it('clearRetryState resets retry state', () => {
    const { unmount } = render(
      <TestComponent
        onMount={(hook) => {
          hook.clearRetryState();
        }}
      />
    );

    expect(hookState!.retryState.isVisible).toBe(false);
    expect(hookState!.retryState.error).toBe('');
    expect(hookState!.retryState.retryCount).toBe(0);

    unmount();
  });
});
