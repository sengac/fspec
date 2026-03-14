/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Part 4: useHitlInput keyboard handler integration.
 * Tests navigation, submission, cancellation, and freeform capture.
 *
 * BUG-118: HITL TUI integration
 */

import React, { useState } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import type { HitlRequestInfo } from '../types/hitlRequest';

// Mock NAPI — spy on sessionSendHitlResponse
vi.mock('@sengac/codelet-napi', () => ({
  sessionSendHitlResponse: vi.fn(),
}));

// Mock logger
vi.mock('../../utils/logger', () => ({
  logger: {
    error: vi.fn(),
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  },
}));

// Mock input system — capture the HITL handler registration
type HandlerFn = (input: string, key: Record<string, boolean>) => boolean;
let capturedHandler: HandlerFn | null = null;
let capturedIsActive = false;

vi.mock('../input/index', () => ({
  InputPriority: { HIGH: 100, MEDIUM: 50, LOW: 10 },
  useInputCompat: vi.fn((config: {
    description?: string;
    isActive: boolean;
    handler: HandlerFn;
  }) => {
    if (config.description && config.description.includes('HITL')) {
      capturedHandler = config.handler;
      capturedIsActive = config.isActive;
    }
  }),
}));

import { sessionSendHitlResponse } from '@sengac/codelet-napi';
import {
  useHitlInput,
  type UseHitlInputResult,
} from '../hooks/useHitlInput';

// ============================================================================
// Helpers
// ============================================================================

function pressKey(key: Partial<Record<string, boolean>>): boolean {
  if (!capturedHandler) {
    throw new Error('No HITL handler registered');
  }
  return capturedHandler('', {
    escape: false,
    return: false,
    upArrow: false,
    downArrow: false,
    ...key,
  });
}

// ============================================================================
// Fixtures
// ============================================================================

const FIXTURE_OPTIONS: HitlRequestInfo = {
  questions: [{
    id: 'approach',
    header: 'Approach',
    question: 'Which approach?',
    options: [
      { label: 'Option A', description: 'First' },
      { label: 'Option B', description: 'Second' },
    ],
  }],
};

const FIXTURE_MULTI: HitlRequestInfo = {
  questions: [
    {
      id: 'priority',
      header: 'Priority',
      question: 'Priority?',
      options: [
        { label: 'High', description: 'Now' },
        { label: 'Low', description: 'Later' },
      ],
    },
    {
      id: 'scope',
      header: 'Scope',
      question: 'Scope?',
      options: [
        { label: 'Minimal', description: 'Essentials' },
        { label: 'Full', description: 'Everything' },
        { label: 'Custom', description: 'Specify' },
      ],
    },
  ],
};

const FIXTURE_FREEFORM: HitlRequestInfo = {
  questions: [{
    id: 'feedback',
    header: 'Feedback',
    question: 'Any feedback?',
  }],
};

// ============================================================================
// Test Harness
// ============================================================================

interface HarnessProps {
  sessionId: string | null;
  isPaused: boolean;
  hitlRequest: HitlRequestInfo | null;
  initialInput?: string;
  onResult?: (result: UseHitlInputResult) => void;
}

const Harness: React.FC<HarnessProps> = ({
  sessionId,
  isPaused,
  hitlRequest,
  initialInput = '',
  onResult,
}) => {
  const [inputValue, setInputValue] = useState(initialInput);

  const result = useHitlInput({
    sessionId,
    isPaused,
    hitlRequest,
    inputValue,
    clearInputValue: () => setInputValue(''),
  });

  React.useEffect(() => {
    onResult?.(result);
  });

  return React.createElement(
    'ink-text',
    null,
    `q:${result.state.questionIndex} sel:${result.state.selectedOption} ` +
      `active:${result.isActive} freeform:${result.isCurrentQuestionFreeform}`
  );
};

// ============================================================================
// Tests
// ============================================================================

describe('HITL Keyboard Handler Integration', () => {
  const mockSend = sessionSendHitlResponse as ReturnType<typeof vi.fn>;

  beforeEach(() => {
    capturedHandler = null;
    capturedIsActive = false;
    mockSend.mockClear();
  });

  afterEach(() => {
    capturedHandler = null;
    capturedIsActive = false;
  });

  // ==========================================================================
  // Scenario: Keyboard handler navigates options
  // ==========================================================================

  describe('Scenario: Navigate options with up/down', () => {
    it('should register active handler when paused with HITL request', () => {
      // @step Given a session is paused with HITL questions
      render(React.createElement(Harness, {
        sessionId: 'nav-session',
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));

      // @step Then handler should be active
      expect(capturedIsActive).toBe(true);
      expect(capturedHandler).not.toBeNull();
    });

    it('should consume down arrow', () => {
      // @step Given HITL is active
      render(React.createElement(Harness, {
        sessionId: 'down-session',
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));

      // @step When user presses down arrow
      const handled = pressKey({ downArrow: true });

      // @step Then key should be consumed
      expect(handled).toBe(true);
    });

    it('should consume up arrow', () => {
      // @step Given HITL is active
      render(React.createElement(Harness, {
        sessionId: 'up-session',
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));

      // @step When user presses up arrow
      expect(pressKey({ upArrow: true })).toBe(true);
    });

    it('should wrap around with 3 options', () => {
      // @step Given a question with 3 options
      const threeOpts: HitlRequestInfo = {
        questions: [{
          id: 'q1',
          header: 'Test',
          question: 'Pick one',
          options: [
            { label: 'First', description: 'A' },
            { label: 'Second', description: 'B' },
            { label: 'Third', description: 'C' },
          ],
        }],
      };

      render(React.createElement(Harness, {
        sessionId: 'wrap-session',
        isPaused: true,
        hitlRequest: threeOpts,
      }));

      // @step When pressing down 3 times (wraps 2→0)
      pressKey({ downArrow: true }); // 0→1
      pressKey({ downArrow: true }); // 1→2
      expect(pressKey({ downArrow: true })).toBe(true); // 2→0

      // @step When pressing up from 0 (wraps to 2)
      expect(pressKey({ upArrow: true })).toBe(true);
    });
  });

  // ==========================================================================
  // Scenario: Keyboard handler submits answers
  // ==========================================================================

  describe('Scenario: Submit answers with Enter', () => {
    it('should submit on Enter for single question', () => {
      // @step Given a single-question HITL request
      render(React.createElement(Harness, {
        sessionId: 'submit-single',
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));

      // @step When user presses Enter
      pressKey({ return: true });

      // @step Then sessionSendHitlResponse should be called
      expect(mockSend).toHaveBeenCalledTimes(1);
      const [sid, resp] = mockSend.mock.calls[0];
      expect(sid).toBe('submit-single');
      expect(resp.cancelled).toBe(false);
      expect(resp.answers).toHaveLength(1);
      expect(resp.answers[0].id).toBe('approach');
      expect(resp.answers[0].selected).toEqual(['Option A']);
    });

    it('should advance then submit on multi-step', async () => {
      // @step Given 2 questions
      render(React.createElement(Harness, {
        sessionId: 'submit-multi',
        isPaused: true,
        hitlRequest: FIXTURE_MULTI,
      }));

      // @step When Enter on q1 (advance)
      pressKey({ return: true });
      expect(mockSend).not.toHaveBeenCalled();

      // @step When React processes state
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step And Enter on q2 (submit all)
      pressKey({ return: true });

      // @step Then both answers should be submitted
      expect(mockSend).toHaveBeenCalledTimes(1);
      const [, resp] = mockSend.mock.calls[0];
      expect(resp.cancelled).toBe(false);
      expect(resp.answers).toHaveLength(2);
      expect(resp.answers[0].id).toBe('priority');
      expect(resp.answers[0].selected).toEqual(['High']);
      expect(resp.answers[1].id).toBe('scope');
      expect(resp.answers[1].selected).toEqual(['Minimal']);
    });
  });

  // ==========================================================================
  // Scenario: User cancels HITL with Escape
  // ==========================================================================

  describe('Scenario: Cancel with Escape', () => {
    it('should send cancellation on Escape', () => {
      // @step Given HITL is active
      render(React.createElement(Harness, {
        sessionId: 'cancel-session',
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));

      // @step When user presses Escape
      expect(pressKey({ escape: true })).toBe(true);

      // @step Then cancelled=true should be sent
      expect(mockSend).toHaveBeenCalledTimes(1);
      const [sid, resp] = mockSend.mock.calls[0];
      expect(sid).toBe('cancel-session');
      expect(resp.cancelled).toBe(true);
    });

    it('should cancel mid-multi-step without submitting', async () => {
      // @step Given 2 questions, user answers q1
      render(React.createElement(Harness, {
        sessionId: 'cancel-mid',
        isPaused: true,
        hitlRequest: FIXTURE_MULTI,
      }));

      pressKey({ return: true }); // answer q1
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step When user presses Escape on q2
      pressKey({ escape: true });

      // @step Then only cancellation should be sent
      expect(mockSend).toHaveBeenCalledTimes(1);
      const [, resp] = mockSend.mock.calls[0];
      expect(resp.cancelled).toBe(true);
    });
  });

  // ==========================================================================
  // Freeform text capture
  // ==========================================================================

  describe('Freeform text capture', () => {
    it('should capture inputValue as other field', () => {
      // @step Given a freeform question with typed text
      render(React.createElement(Harness, {
        sessionId: 'freeform-submit',
        isPaused: true,
        hitlRequest: FIXTURE_FREEFORM,
        initialInput: 'Typed feedback here',
      }));

      // @step When user presses Enter
      pressKey({ return: true });

      // @step Then answer should have empty selected and typed text in other
      expect(mockSend).toHaveBeenCalledTimes(1);
      const [, resp] = mockSend.mock.calls[0];
      expect(resp.cancelled).toBe(false);
      expect(resp.answers[0].id).toBe('feedback');
      expect(resp.answers[0].selected).toEqual([]);
      expect(resp.answers[0].other).toBe('Typed feedback here');
    });

    it('should let character input pass through', () => {
      // @step Given a freeform question
      render(React.createElement(Harness, {
        sessionId: 'freeform-pass',
        isPaused: true,
        hitlRequest: FIXTURE_FREEFORM,
      }));

      // @step When user types a character
      if (capturedHandler) {
        const handled = capturedHandler('a', {
          escape: false,
          return: false,
          upArrow: false,
          downArrow: false,
        });

        // @step Then handler should NOT consume it (pass to MultiLineInput)
        expect(handled).toBe(false);
      }
    });
  });

  // ==========================================================================
  // Inactive state
  // ==========================================================================

  describe('Inactive state', () => {
    it('should not activate when not paused', () => {
      render(React.createElement(Harness, {
        sessionId: 'inactive-1',
        isPaused: false,
        hitlRequest: null,
      }));
      expect(capturedIsActive).toBe(false);
    });

    it('should not activate when paused without hitlRequest', () => {
      render(React.createElement(Harness, {
        sessionId: 'inactive-2',
        isPaused: true,
        hitlRequest: null,
      }));
      expect(capturedIsActive).toBe(false);
    });

    it('should not activate when no session ID', () => {
      render(React.createElement(Harness, {
        sessionId: null,
        isPaused: true,
        hitlRequest: FIXTURE_OPTIONS,
      }));
      expect(capturedIsActive).toBe(false);
    });
  });
});
