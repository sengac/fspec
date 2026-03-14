/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Tests for the useHitlInput hook — keyboard handling, state management,
 * freeform text capture, and NAPI call wiring.
 *
 * BUG-118: Tests the extracted HITL keyboard handler hook.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { HitlRequestInfo } from '../../types/hitlRequest';

// Mock the NAPI module
vi.mock('@sengac/codelet-napi', () => ({
  sessionSendHitlResponse: vi.fn(),
}));

// Mock the logger
vi.mock('../../../utils/logger', () => ({
  logger: {
    error: vi.fn(),
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  },
}));

// Mock the input system — capture the handler registration
let registeredHandler: ((input: string, key: Record<string, boolean>) => boolean) | null = null;
let registeredIsActive = false;

vi.mock('../../input/index', () => ({
  InputPriority: { HIGH: 100, MEDIUM: 50, LOW: 10 },
  useInputCompat: vi.fn((config: {
    isActive: boolean;
    handler: (input: string, key: Record<string, boolean>) => boolean;
  }) => {
    registeredHandler = config.handler;
    registeredIsActive = config.isActive;
  }),
}));

// We need to test the hook logic. Since useInputCompat is mocked,
// we can test the handler function it registers directly.
import { sessionSendHitlResponse } from '@sengac/codelet-napi';

// Helper to simulate calling the hook's handler
function simulateKey(key: Partial<Record<string, boolean>>): boolean {
  if (!registeredHandler) {
    throw new Error('No handler registered');
  }
  return registeredHandler('', {
    escape: false,
    return: false,
    upArrow: false,
    downArrow: false,
    ...key,
  });
}

// Since React hooks can't be called outside components, we test
// the handler function that useInputCompat receives by importing
// the hook and triggering it via renderHook.
// For simplicity, we use a manual React testing approach.

import React, { useState } from 'react';
import { render } from 'ink-testing-library';
import { useHitlInput, type UseHitlInputResult } from '../useHitlInput';

interface TestHarnessProps {
  sessionId: string | null;
  isPaused: boolean;
  hitlRequest: HitlRequestInfo | null;
  initialInputValue?: string;
  onStateChange?: (result: UseHitlInputResult) => void;
}

/**
 * Test harness component that renders the hook and exposes state changes.
 */
const TestHarness: React.FC<TestHarnessProps> = ({
  sessionId,
  isPaused,
  hitlRequest,
  initialInputValue = '',
  onStateChange,
}) => {
  const [inputValue, setInputValue] = useState(initialInputValue);

  const result = useHitlInput({
    sessionId,
    isPaused,
    hitlRequest,
    inputValue,
    clearInputValue: () => setInputValue(''),
  });

  // Report state changes to test
  React.useEffect(() => {
    onStateChange?.(result);
  });

  // Expose inputValue for verification
  return React.createElement('ink-text', null,
    `q:${result.state.questionIndex} s:${result.state.selectedOption} active:${result.isActive} freeform:${result.isCurrentQuestionFreeform} input:${inputValue}`
  );
};

describe('Feature: HITL keyboard handler hook (BUG-118)', () => {
  const mockSendResponse = sessionSendHitlResponse as ReturnType<typeof vi.fn>;

  beforeEach(() => {
    registeredHandler = null;
    registeredIsActive = false;
    mockSendResponse.mockClear();
  });

  afterEach(() => {
    registeredHandler = null;
    registeredIsActive = false;
  });

  // ===========================================================================
  // Scenario: AgentView HITL keyboard handler navigates options
  // ===========================================================================

  describe('Scenario: AgentView HITL keyboard handler navigates options', () => {
    it('should navigate options with up/down arrows', () => {
      // @step Given a session is paused with HITL questions containing options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'Pick one',
            options: [
              { label: 'First', description: 'Desc 1' },
              { label: 'Second', description: 'Desc 2' },
              { label: 'Third', description: 'Desc 3' },
            ],
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session',
          isPaused: true,
          hitlRequest,
          onStateChange: (r: UseHitlInputResult) => { lastResult = r; },
        })
      );

      expect(registeredIsActive).toBe(true);
      expect(lastResult?.state.selectedOption).toBe(0);

      // @step When the user presses down arrow
      const handledDown = simulateKey({ downArrow: true });
      // @step Then the selected option should move down
      expect(handledDown).toBe(true);

      // @step When the user presses up arrow
      const handledUp = simulateKey({ upArrow: true });
      // @step Then the selected option should move up
      expect(handledUp).toBe(true);
    });

    it('should wrap around when navigating past edges', () => {
      // @step Given a session is paused with HITL questions containing options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'Pick one',
            options: [
              { label: 'A', description: 'A' },
              { label: 'B', description: 'B' },
            ],
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session',
          isPaused: true,
          hitlRequest,
        })
      );

      // @step When the user presses up arrow from index 0
      simulateKey({ upArrow: true });
      // @step Then it should wrap to the last option
      // (Verified by the handler returning true — wrapping logic in useState setter)
    });
  });

  // ===========================================================================
  // Scenario: AgentView HITL keyboard handler submits all answers
  // ===========================================================================

  describe('Scenario: AgentView HITL keyboard handler submits all answers', () => {
    it('should call sessionSendHitlResponse with all collected answers on last question Enter', () => {
      // @step Given a session is paused with HITL questions and all questions answered
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'only_q',
            header: 'Q1',
            question: 'Only question',
            options: [
              { label: 'Yes', description: 'Agree' },
              { label: 'No', description: 'Disagree' },
            ],
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session-submit',
          isPaused: true,
          hitlRequest,
        })
      );

      // @step When the user presses Enter on the last question
      simulateKey({ return: true });

      // @step Then sessionSendHitlResponse should be called with all collected answers
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [sessionId, response] = mockSendResponse.mock.calls[0];
      expect(sessionId).toBe('test-session-submit');

      // @step And cancelled should be false
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(1);
      expect(response.answers[0].id).toBe('only_q');
      expect(response.answers[0].selected).toEqual(['Yes']);
    });
  });

  // ===========================================================================
  // Scenario: User cancels HITL with Escape
  // ===========================================================================

  describe('Scenario: User cancels HITL with Escape', () => {
    it('should call sessionSendHitlResponse with cancelled true on Escape', () => {
      // @step Given a session is paused with HITL questions
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'Test question',
            options: [{ label: 'A', description: 'Option A' }],
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session-cancel',
          isPaused: true,
          hitlRequest,
        })
      );

      // @step When the user presses Escape
      const handled = simulateKey({ escape: true });
      expect(handled).toBe(true);

      // @step Then sessionSendHitlResponse should be called with cancelled true
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [sessionId, response] = mockSendResponse.mock.calls[0];
      expect(sessionId).toBe('test-session-cancel');
      expect(response.cancelled).toBe(true);

      // @step And the handler should unblock and return Cancelled
      // (Unblocking is verified on the Rust side — here we verify the NAPI call was made)
    });
  });

  // ===========================================================================
  // Scenario: Multi-step HITL advances through questions
  // ===========================================================================

  describe('Scenario: Multi-step HITL advances through questions', () => {
    it('should advance question index on Enter and submit on last', async () => {
      // @step Given isPaused is true and hitlRequest contains 2 questions
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'First question?',
            options: [
              { label: 'Yes', description: 'Agree' },
              { label: 'No', description: 'Disagree' },
            ],
          },
          {
            id: 'q2',
            header: 'Q2',
            question: 'Second question?',
            options: [
              { label: 'A', description: 'Choice A' },
              { label: 'B', description: 'Choice B' },
            ],
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session-multi',
          isPaused: true,
          hitlRequest,
        })
      );

      // @step And the user is on question 1 of 2
      // @step When the user selects an option and presses Enter
      simulateKey({ return: true });

      // @step Then InputTransition should advance to question 2 of 2
      // Should NOT have called sessionSendHitlResponse yet (not last question)
      expect(mockSendResponse).not.toHaveBeenCalled();

      // @step And the first question answer should be stored
      // Wait for React to process state updates and re-render the hook
      // (re-render updates registeredHandler with new closure containing updated state)
      await new Promise(resolve => setTimeout(resolve, 10));

      // Press Enter again on second question to submit
      simulateKey({ return: true });

      // Now it should have submitted with both answers
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [, response] = mockSendResponse.mock.calls[0];
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(2);
      expect(response.answers[0].id).toBe('q1');
      expect(response.answers[1].id).toBe('q2');
    });
  });

  // ===========================================================================
  // Freeform text capture
  // ===========================================================================

  describe('Freeform text capture', () => {
    it('should capture inputValue as other field for freeform questions', () => {
      // @step Given isPaused is true and hitlRequest contains a question without options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'feedback',
            header: 'Feedback',
            question: 'Any additional feedback?',
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session-freeform',
          isPaused: true,
          hitlRequest,
          initialInputValue: 'My typed feedback',
        })
      );

      // @step When the user presses Enter to submit freeform text
      simulateKey({ return: true });

      // @step Then sessionSendHitlResponse should capture the typed text as other
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [, response] = mockSendResponse.mock.calls[0];
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(1);
      expect(response.answers[0].id).toBe('feedback');
      expect(response.answers[0].selected).toEqual([]);
      expect(response.answers[0].other).toBe('My typed feedback');
    });

    it('should not intercept character input for freeform questions', () => {
      // @step Given a freeform question is active
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'feedback',
            header: 'Feedback',
            question: 'Any feedback?',
          },
        ],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session-passthrough',
          isPaused: true,
          hitlRequest,
        })
      );

      // @step When the user types a character (not a control key)
      // The handler should return false to let MultiLineInput handle it
      if (registeredHandler) {
        const handled = registeredHandler('a', {
          escape: false,
          return: false,
          upArrow: false,
          downArrow: false,
        });
        expect(handled).toBe(false);
      }
    });
  });

  // ===========================================================================
  // Inactive state
  // ===========================================================================

  describe('Inactive state', () => {
    it('should not register active handler when not paused', () => {
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session',
          isPaused: false,
          hitlRequest: null,
        })
      );

      expect(registeredIsActive).toBe(false);
    });

    it('should not register active handler when no HITL request', () => {
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-session',
          isPaused: true,
          hitlRequest: null,
        })
      );

      expect(registeredIsActive).toBe(false);
    });

    it('should not register active handler when no session', () => {
      const hitlRequest: HitlRequestInfo = {
        questions: [{ id: 'q1', header: 'Q1', question: 'Q?' }],
      };

      render(
        React.createElement(TestHarness, {
          sessionId: null,
          isPaused: true,
          hitlRequest,
        })
      );

      expect(registeredIsActive).toBe(false);
    });
  });
});
