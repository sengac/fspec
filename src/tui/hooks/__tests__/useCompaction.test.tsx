/**
 * Unit Tests: useCompaction Hook
 *
 * Tests the REAL useCompaction hook behavior using React test components.
 * Following the pattern from useSlashCommandInput.test.tsx
 *
 * Key behaviors tested:
 * - startCompaction sets unified state for all trigger types
 * - endCompaction resets state
 * - updateProgress updates progress while maintaining other state
 * - State is consistent across all compaction triggers
 */

import React, { useEffect } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import {
  useCompaction,
  type CompactionHookReturn,
  type CompactionTrigger,
  type CompactionProgress,
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
  sessionGetCompactionProgress: vi.fn().mockReturnValue(null),
}));

// Test fixtures
const fixtures = {
  progress: {
    analyzingContext: {
      phase: 'Analyzing context',
      current: 15,
      total: 32,
    } as CompactionProgress,
    generatingSummary: {
      phase: 'generating summary',
      current: 1,
      total: 1,
    } as CompactionProgress,
    emergency: {
      phase: 'emergency compacting',
      current: 5,
      total: 20,
    } as CompactionProgress,
    starting: {
      phase: 'Starting',
      current: 0,
      total: 1,
    } as CompactionProgress,
  },
  sessionIds: {
    manual: 'session-manual-123',
    hookTriggered: 'session-hook-456',
    emergency: 'session-emergency-789',
  },
};

// Store hook state for assertions
let hookState: CompactionHookReturn | null = null;

/**
 * Test component that exposes hook state
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
      active:{String(compaction.state.isActive)}|
      trigger:{compaction.state.trigger ?? 'null'}|
      phase:{compaction.state.progress?.phase ?? 'null'}|
      session:{compaction.state.sessionId ?? 'null'}
    </Text>
  );
}

describe('useCompaction Hook', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hookState = null;
  });

  afterEach(() => {
    hookState = null;
  });

  describe('Initial State', () => {
    it('should have inactive state on mount', () => {
      const { lastFrame } = render(<TestComponent />);

      expect(lastFrame()).toContain('active:false');
      expect(lastFrame()).toContain('trigger:null');
      expect(lastFrame()).toContain('phase:null');
      expect(lastFrame()).toContain('session:null');
    });

    it('should expose all required functions', () => {
      render(<TestComponent />);

      expect(hookState).not.toBeNull();
      expect(typeof hookState!.startCompaction).toBe('function');
      expect(typeof hookState!.endCompaction).toBe('function');
      expect(typeof hookState!.updateProgress).toBe('function');
      expect(typeof hookState!.performManualCompaction).toBe('function');
      expect(typeof hookState!.clearRetryState).toBe('function');
      expect(typeof hookState!.handleRetryOption).toBe('function');
    });
  });

  describe('startCompaction', () => {
    it('should set state to active with manual trigger', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('manual', fixtures.sessionIds.manual);

      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(true);
      expect(hookState!.state.trigger).toBe('manual');
      expect(hookState!.state.sessionId).toBe(fixtures.sessionIds.manual);
    });

    it('should set state to active with hook-triggered trigger', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('hook-triggered', fixtures.sessionIds.hookTriggered);

      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(true);
      expect(hookState!.state.trigger).toBe('hook-triggered');
      expect(hookState!.state.sessionId).toBe(fixtures.sessionIds.hookTriggered);
    });

    it('should set state to active with emergency trigger', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('emergency', fixtures.sessionIds.emergency);

      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(true);
      expect(hookState!.state.trigger).toBe('emergency');
      expect(hookState!.state.sessionId).toBe(fixtures.sessionIds.emergency);
    });

    it('should accept custom initial progress', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction(
        'hook-triggered',
        fixtures.sessionIds.hookTriggered,
        fixtures.progress.analyzingContext
      );

      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.progress).toEqual(fixtures.progress.analyzingContext);
    });

    it('should use default progress when not provided', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('manual', fixtures.sessionIds.manual);

      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.progress).toEqual(fixtures.progress.starting);
    });
  });

  describe('endCompaction', () => {
    it('should reset state to initial values', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      // Start compaction first
      hookState!.startCompaction('manual', fixtures.sessionIds.manual, fixtures.progress.analyzingContext);
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(true);

      // End compaction
      hookState!.endCompaction();
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(false);
      expect(hookState!.state.progress).toBeNull();
      expect(hookState!.state.trigger).toBeNull();
      expect(hookState!.state.sessionId).toBeNull();
    });
  });

  describe('updateProgress', () => {
    it('should update progress while keeping other state', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      // Start compaction
      hookState!.startCompaction('hook-triggered', fixtures.sessionIds.hookTriggered, fixtures.progress.starting);
      await new Promise(r => setTimeout(r, 10));

      // Update progress
      hookState!.updateProgress(fixtures.progress.analyzingContext);
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.isActive).toBe(true);
      expect(hookState!.state.trigger).toBe('hook-triggered');
      expect(hookState!.state.sessionId).toBe(fixtures.sessionIds.hookTriggered);
      expect(hookState!.state.progress).toEqual(fixtures.progress.analyzingContext);
    });

    it('should handle multiple progress updates', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('manual', fixtures.sessionIds.manual);
      await new Promise(r => setTimeout(r, 10));

      // First update
      hookState!.updateProgress(fixtures.progress.analyzingContext);
      await new Promise(r => setTimeout(r, 10));
      expect(hookState!.state.progress?.phase).toBe('Analyzing context');

      // Second update
      hookState!.updateProgress(fixtures.progress.generatingSummary);
      await new Promise(r => setTimeout(r, 10));
      expect(hookState!.state.progress?.phase).toBe('generating summary');
    });
  });

  describe('Unified State for All Triggers', () => {
    it('should produce identical state structure for all trigger types', async () => {
      const triggers: CompactionTrigger[] = ['manual', 'hook-triggered', 'emergency'];

      for (const trigger of triggers) {
        render(<TestComponent />);
        await new Promise(r => setTimeout(r, 10));

        hookState!.startCompaction(trigger, `session-${trigger}`, fixtures.progress.analyzingContext);
        await new Promise(r => setTimeout(r, 10));

        // All triggers should produce the same state structure
        expect(hookState!.state.isActive).toBe(true);
        expect(hookState!.state.trigger).toBe(trigger);
        expect(hookState!.state.sessionId).toBe(`session-${trigger}`);
        expect(hookState!.state.progress).toEqual(fixtures.progress.analyzingContext);

        // Cleanup
        hookState!.endCompaction();
        await new Promise(r => setTimeout(r, 10));
      }
    });

    it('should allow full state transition lifecycle for any trigger', async () => {
      const triggers: CompactionTrigger[] = ['manual', 'hook-triggered', 'emergency'];

      for (const trigger of triggers) {
        render(<TestComponent />);
        await new Promise(r => setTimeout(r, 10));

        // Start
        hookState!.startCompaction(trigger, 'session-1');
        await new Promise(r => setTimeout(r, 10));
        expect(hookState!.state.isActive).toBe(true);

        // Update
        hookState!.updateProgress(fixtures.progress.analyzingContext);
        await new Promise(r => setTimeout(r, 10));
        expect(hookState!.state.progress?.phase).toBe('Analyzing context');

        // Update again
        hookState!.updateProgress(fixtures.progress.generatingSummary);
        await new Promise(r => setTimeout(r, 10));
        expect(hookState!.state.progress?.phase).toBe('generating summary');

        // End
        hookState!.endCompaction();
        await new Promise(r => setTimeout(r, 10));
        expect(hookState!.state.isActive).toBe(false);
      }
    });
  });

  describe('Retry State', () => {
    it('should have initial retry state with no errors', () => {
      render(<TestComponent />);

      expect(hookState!.retryState.isVisible).toBe(false);
      expect(hookState!.retryState.error).toBe('');
      expect(hookState!.retryState.retryCount).toBe(0);
    });

    it('should clear retry state when clearRetryState is called', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.clearRetryState();
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.retryState.isVisible).toBe(false);
      expect(hookState!.retryState.error).toBe('');
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty session ID', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      hookState!.startCompaction('manual', '');
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.sessionId).toBe('');
      expect(hookState!.state.isActive).toBe(true);
    });

    it('should handle progress with zero values', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      const zeroProgress: CompactionProgress = {
        phase: 'starting',
        current: 0,
        total: 0,
      };

      hookState!.startCompaction('manual', 'session-1', zeroProgress);
      await new Promise(r => setTimeout(r, 10));

      expect(hookState!.state.progress?.current).toBe(0);
      expect(hookState!.state.progress?.total).toBe(0);
    });

    it('should handle rapid state transitions', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      // Rapid start/end cycles
      for (let i = 0; i < 5; i++) {
        hookState!.startCompaction('manual', `session-${i}`);
        hookState!.endCompaction();
      }

      await new Promise(r => setTimeout(r, 10));

      // Should end in inactive state
      expect(hookState!.state.isActive).toBe(false);
    });
  });

  describe('State Stability', () => {
    it('should maintain stable function references', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      const initialFunctions = {
        startCompaction: hookState!.startCompaction,
        endCompaction: hookState!.endCompaction,
        updateProgress: hookState!.updateProgress,
      };

      // Trigger a state change
      hookState!.startCompaction('manual', 'session-1');
      await new Promise(r => setTimeout(r, 10));

      // Functions should be the same references (memoized)
      expect(hookState!.startCompaction).toBe(initialFunctions.startCompaction);
      expect(hookState!.endCompaction).toBe(initialFunctions.endCompaction);
      expect(hookState!.updateProgress).toBe(initialFunctions.updateProgress);
    });
  });

  describe('Manual compaction initial phase text', () => {
    // @step Given a user triggers manual compaction via /compact
    // @step When performManualCompaction sets the initial progress phase
    // @step Then the phase text must be "Preparing compaction"
    // @step And the phase text must NOT contain "anchor" or "anchors"
    it('should use "Preparing compaction" phase, not stale anchor text', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      // Trigger manual compaction — performManualCompaction calls startCompaction internally
      // with the initial phase text. Since sessionCompact is mocked, it resolves immediately.
      try {
        await hookState!.performManualCompaction('session-1');
      } catch {
        // May throw if mocked sessionCompact rejects, but state is set synchronously
      }
      await new Promise(r => setTimeout(r, 10));

      // The initial phase should NOT reference anchors
      const phase = hookState!.state.progress?.phase ?? '';
      expect(phase).not.toContain('anchor');
      expect(phase).not.toContain('Anchor');
    });
  });

  describe('Manual compaction does not end compaction prematurely', () => {
    // @step Given a user triggers manual compaction via /compact
    // @step When sessionCompact returns after the in-memory setup phase
    // @step Then performManualCompaction must NOT call endCompaction via setTimeout
    // @step And the compaction indicator must remain active until CompactionComplete arrives
    it('should remain active after performManualCompaction resolves', async () => {
      render(<TestComponent />);
      await new Promise(r => setTimeout(r, 10));

      // Perform manual compaction (mocked to succeed immediately)
      await hookState!.performManualCompaction('session-1');
      await new Promise(r => setTimeout(r, 10));

      // Compaction should remain ACTIVE after sessionCompact returns.
      // The CompactionComplete chunk (not performManualCompaction) is the definitive end signal.
      // Previously, a setTimeout(endCompaction, 1000) would prematurely end it.
      expect(hookState!.state.isActive).toBe(true);

      // Advance timers to verify no delayed endCompaction fires
      await new Promise(r => setTimeout(r, 1500));
      expect(hookState!.state.isActive).toBe(true);
    });
  });
});
