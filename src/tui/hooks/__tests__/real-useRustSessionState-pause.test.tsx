/**
 * PAUSE-001: Real useRustSessionState Pause Integration Tests
 * 
 * These tests verify the ACTUAL useRustSessionState hook has pause state fields.
 * Tests will FAIL until implementation is complete.
 * 
 * Required additions to RustSessionSnapshot:
 * 1. isPaused: boolean (derived from status === 'paused')
 * 2. pauseInfo: PauseInfo | null (from sessionGetPauseState)
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { Text } from 'ink';

// Import the REAL hook and types
import { useRustSessionState, type RustSessionSnapshot } from '../../hooks/useRustSessionState';

// Mock the NAPI module
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetStatus: vi.fn().mockReturnValue('idle'),
  sessionGetModel: vi.fn().mockReturnValue(null),
  sessionGetTokens: vi.fn().mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionGetDebugEnabled: vi.fn().mockReturnValue(false),
  sessionGetMergedOutput: vi.fn().mockReturnValue([]),
  sessionAttach: vi.fn(),
  sessionDetach: vi.fn(),
  // The NEW functions that must exist
  sessionGetPauseState: vi.fn().mockReturnValue(null),
}));

import * as codeletNapi from '@sengac/codelet-napi';

// Test component that uses the hook
const TestComponent: React.FC<{ sessionId: string }> = ({ sessionId }) => {
  const { snapshot } = useRustSessionState(sessionId);
  const pauseInfoDisplay = snapshot.pauseInfo ? snapshot.pauseInfo.toolName : 'null';
  return (
    <Text>
      {`status:${snapshot.status}|isPaused:${String(snapshot.isPaused)}|pauseInfo:${pauseInfoDisplay}`}
    </Text>
  );
};

describe('PAUSE-001: Real useRustSessionState Pause Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // =============================================================================
  // RustSessionSnapshot Type Tests
  // =============================================================================

  describe('RustSessionSnapshot type has pause fields', () => {
    it('snapshot should have isPaused boolean field', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('idle');
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      // This test FAILS if isPaused is not in the snapshot type
      // The output should contain isPaused: (true or false, not undefined)
      expect(output).toMatch(/isPaused:(true|false)/);
    });

    it('snapshot should have pauseInfo field (null or PauseInfo)', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('idle');
      vi.mocked(codeletNapi.sessionGetPauseState).mockReturnValue(null);
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      // This test FAILS if pauseInfo is not in the snapshot type
      // pauseInfo should be accessible and render as 'null' or a tool name
      expect(output).toContain('pauseInfo:');
    });
  });

  // =============================================================================
  // isPaused Derivation Tests
  // =============================================================================

  describe('isPaused derivation from status', () => {
    it('isPaused should be false when status is "idle"', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('idle');
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      expect(output).toContain('isPaused:false');
    });

    it('isPaused should be false when status is "running"', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('running');
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      expect(output).toContain('isPaused:false');
    });

    it('isPaused should be true when status is "paused"', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('paused');
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      expect(output).toContain('isPaused:true');
    });
  });

  // =============================================================================
  // pauseInfo Integration Tests
  // =============================================================================

  describe('pauseInfo from sessionGetPauseState', () => {
    it('pauseInfo should be null when session is not paused', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('running');
      vi.mocked(codeletNapi.sessionGetPauseState).mockReturnValue(null);
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      expect(output).toContain('pauseInfo:null');
    });

    it('pauseInfo should contain pause details when session is paused', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('paused');
      vi.mocked(codeletNapi.sessionGetPauseState).mockReturnValue({
        kind: 'continue',
        toolName: 'WebSearch',
        message: 'Page loaded at https://example.com',
        details: null,
      });
      
      const { lastFrame } = render(<TestComponent sessionId="test-session" />);
      const output = lastFrame() || '';
      
      expect(output).toContain('pauseInfo:WebSearch');
    });
  });

  // =============================================================================
  // Hook calls sessionGetPauseState
  // =============================================================================

  describe('hook fetches pause state from NAPI', () => {
    it('should call sessionGetPauseState when fetching snapshot', () => {
      vi.mocked(codeletNapi.sessionGetStatus).mockReturnValue('paused');
      
      render(<TestComponent sessionId="test-session" />);
      
      // The hook should call sessionGetPauseState
      expect(codeletNapi.sessionGetPauseState).toHaveBeenCalledWith('test-session');
    });
  });
});
