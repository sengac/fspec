/**
 * Feature: spec/features/hitl-freeform-other-option-for-predefined-choices.feature
 *
 * This test file validates the "Other..." freeform option appended to HITL
 * questions with predefined choices.
 *
 * Uses the same TestHarness + ink-testing-library pattern as useHitlInput.test.tsx
 * to render the hook inside a real React component and exercise the keyboard
 * handler captured by the mocked useInputCompat.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React, { useState } from 'react';
import { render } from 'ink-testing-library';
import type { HitlRequestInfo } from '../../types/hitlRequest';

// Mock NAPI
vi.mock('@sengac/codelet-napi', () => ({
  sessionSendHitlResponse: vi.fn(),
}));

// Mock logger
vi.mock('../../../utils/logger', () => ({
  logger: {
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
}));

// Capture the keyboard handler registered by useInputCompat
let registeredHandler:
  | ((input: string, key: Record<string, boolean>) => boolean)
  | null = null;
let registeredIsActive = false;

vi.mock('../../input/index', () => ({
  InputPriority: { HIGH: 100, MEDIUM: 50, LOW: 10 },
  useInputCompat: vi.fn(
    (config: {
      isActive: boolean;
      handler: (input: string, key: Record<string, boolean>) => boolean;
    }) => {
      registeredHandler = config.handler;
      registeredIsActive = config.isActive;
    }
  ),
}));

import { sessionSendHitlResponse } from '@sengac/codelet-napi';
import { useHitlInput, type UseHitlInputResult } from '../useHitlInput';

function makeKey(
  overrides: Partial<Record<string, boolean>> = {}
): Record<string, boolean> {
  return {
    upArrow: false,
    downArrow: false,
    return: false,
    escape: false,
    ctrl: false,
    meta: false,
    ...overrides,
  };
}

function simulateKey(key: Partial<Record<string, boolean>>): boolean {
  if (!registeredHandler) {
    throw new Error('No handler registered');
  }
  return registeredHandler('', makeKey(key));
}

interface TestHarnessProps {
  sessionId: string | null;
  isPaused: boolean;
  hitlRequest: HitlRequestInfo | null;
  initialInputValue?: string;
  onStateChange?: (result: UseHitlInputResult) => void;
}

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

  React.useEffect(() => {
    onStateChange?.(result);
  });

  return React.createElement(
    'ink-text',
    null,
    `q:${result.state.questionIndex} s:${result.state.selectedOption} active:${result.isActive} freeform:${result.isCurrentQuestionFreeform} other:${result.isOtherActive} hint:${result.showEmptyHint}`
  );
};

describe('Feature: HITL Freeform Other Option for Predefined Choices', () => {
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
  // Scenario: Select Other and submit freeform response
  // ===========================================================================

  describe('Scenario: Select Other and submit freeform response', () => {
    it('should return answer with selected [] and other text when Other is chosen', async () => {
      // @step Given the AI presents a HITL question with options "A", "B", and "C"
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach?',
            options: [
              { label: 'A', description: 'Option A' },
              { label: 'B', description: 'Option B' },
              { label: 'C', description: 'Option C' },
            ],
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-other-submit',
          isPaused: true,
          hitlRequest,
          initialInputValue: 'I want approach D which combines A and C',
          onStateChange: (r: UseHitlInputResult) => {
            lastResult = r;
          },
        })
      );

      // @step Then the TUI renders the options with an appended "Other..." entry in dim text
      // With 3 options, "Other..." is at index 3. Navigation wraps at 4 (3+1).
      expect(registeredIsActive).toBe(true);
      expect(lastResult?.state.selectedOption).toBe(0);

      // @step When I navigate to "Other..." and press Enter
      // Navigate down 3 times: 0→1→2→3 (Other...)
      simulateKey({ downArrow: true });
      simulateKey({ downArrow: true });
      simulateKey({ downArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      expect(lastResult?.state.selectedOption).toBe(3);

      // Press Enter to activate Other
      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the option list is replaced by a freeform text input
      expect(lastResult?.isOtherActive).toBe(true);

      // @step When I type "I want approach D which combines A and C" and press Enter
      // inputValue was set via initialInputValue, simulate Enter
      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the answer is returned as selected [] and other "I want approach D which combines A and C"
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [sessionId, response] = mockSendResponse.mock.calls[0];
      expect(sessionId).toBe('test-other-submit');
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(1);
      expect(response.answers[0].id).toBe('approach');
      expect(response.answers[0].selected).toEqual([]);
      expect(response.answers[0].other).toBe(
        'I want approach D which combines A and C'
      );
    });
  });

  // ===========================================================================
  // Scenario: Escape from Other freeform returns to option list
  // ===========================================================================

  describe('Scenario: Escape from Other freeform returns to option list', () => {
    it('should return to option selection when Escape is pressed in Other freeform mode', async () => {
      // @step Given the AI presents a HITL question with options "A" and "B"
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach?',
            options: [
              { label: 'A', description: 'Option A' },
              { label: 'B', description: 'Option B' },
            ],
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-other-escape',
          isPaused: true,
          hitlRequest,
          onStateChange: (r: UseHitlInputResult) => {
            lastResult = r;
          },
        })
      );

      // @step When I navigate to "Other..." and press Enter
      // Navigate down 2 times: 0→1→2 (Other... at index 2 for 2-option question)
      simulateKey({ downArrow: true });
      simulateKey({ downArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(lastResult?.state.selectedOption).toBe(2);

      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the option list is replaced by a freeform text input
      expect(lastResult?.isOtherActive).toBe(true);

      // @step When I press Escape
      simulateKey({ escape: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the option list is displayed again with "A", "B", and "Other..."
      expect(lastResult?.isOtherActive).toBe(false);
      // sessionSendHitlResponse must NOT have been called (Escape didn't cancel flow)
      expect(mockSendResponse).not.toHaveBeenCalled();

      // @step When I navigate to "A" and press Enter
      // After escape, selectedOption is still at 2 (Other...). Navigate up twice to reach 0.
      simulateKey({ upArrow: true });
      simulateKey({ upArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(lastResult?.state.selectedOption).toBe(0);

      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the answer is returned as selected ["A"]
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [, response] = mockSendResponse.mock.calls[0];
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(1);
      expect(response.answers[0].selected).toEqual(['A']);
      expect(response.answers[0].other).toBeUndefined();
    });
  });

  // ===========================================================================
  // Scenario: Mixed Other and predefined across multi-question flow
  // ===========================================================================

  describe('Scenario: Mixed Other and predefined across multi-question flow', () => {
    it('should submit mixed answers across multi-step questions', async () => {
      // @step Given the AI presents 2 HITL questions each with predefined options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'First question',
            options: [
              { label: 'X', description: 'Option X' },
              { label: 'Y', description: 'Option Y' },
            ],
          },
          {
            id: 'q2',
            header: 'Q2',
            question: 'Second question',
            options: [
              { label: 'Option A', description: 'Choice A' },
              { label: 'Option B', description: 'Choice B' },
            ],
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-other-multi',
          isPaused: true,
          hitlRequest,
          initialInputValue: 'custom text',
          onStateChange: (r: UseHitlInputResult) => {
            lastResult = r;
          },
        })
      );

      // @step When I select "Other..." on question 1 and type "custom text" and press Enter
      // Navigate to Other... (index 2 for 2-option question)
      simulateKey({ downArrow: true });
      simulateKey({ downArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(lastResult?.state.selectedOption).toBe(2);

      // Activate Other
      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(lastResult?.isOtherActive).toBe(true);

      // Submit freeform (inputValue is 'custom text')
      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the flow advances to question 2
      expect(lastResult?.state.questionIndex).toBe(1);
      expect(lastResult?.isOtherActive).toBe(false);
      expect(mockSendResponse).not.toHaveBeenCalled();

      // @step When I select "Option B" on question 2 and press Enter
      // Navigate to index 1 (Option B)
      simulateKey({ downArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(lastResult?.state.selectedOption).toBe(1);

      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then both answers are submitted with question 1 as selected [] other "custom text" and question 2 as selected ["Option B"]
      expect(mockSendResponse).toHaveBeenCalledTimes(1);
      const [, response] = mockSendResponse.mock.calls[0];
      expect(response.cancelled).toBe(false);
      expect(response.answers).toHaveLength(2);

      expect(response.answers[0].id).toBe('q1');
      expect(response.answers[0].selected).toEqual([]);
      expect(response.answers[0].other).toBe('custom text');

      expect(response.answers[1].id).toBe('q2');
      expect(response.answers[1].selected).toEqual(['Option B']);
      expect(response.answers[1].other).toBeUndefined();
    });
  });

  // ===========================================================================
  // Scenario: Empty freeform submission is rejected
  // ===========================================================================

  describe('Scenario: Empty freeform submission is rejected', () => {
    it('should not advance when submitting empty text in Other freeform mode', async () => {
      // @step Given the AI presents a HITL question with options "A" and "B"
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach?',
            options: [
              { label: 'A', description: 'Option A' },
              { label: 'B', description: 'Option B' },
            ],
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-other-empty',
          isPaused: true,
          hitlRequest,
          initialInputValue: '',
          onStateChange: (r: UseHitlInputResult) => {
            lastResult = r;
          },
        })
      );

      // @step When I navigate to "Other..." and press Enter
      simulateKey({ downArrow: true });
      simulateKey({ downArrow: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the option list is replaced by a freeform text input
      expect(lastResult?.isOtherActive).toBe(true);

      // @step When I press Enter with empty text
      simulateKey({ return: true });
      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then I see an inline hint "Please type a response or press Esc to go back"
      expect(lastResult?.showEmptyHint).toBe(true);

      // @step And the cursor remains in the freeform text input
      expect(lastResult?.isOtherActive).toBe(true);
      expect(mockSendResponse).not.toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // Scenario: Freeform-only question does not show Other
  // ===========================================================================

  describe('Scenario: Freeform-only question does not show Other', () => {
    it('should not expose Other-related state for questions without options', async () => {
      // @step Given the AI presents a HITL question without predefined options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'feedback',
            header: 'Feedback',
            question: 'Type something',
          },
        ],
      };

      let lastResult: UseHitlInputResult | null = null;
      render(
        React.createElement(TestHarness, {
          sessionId: 'test-no-other',
          isPaused: true,
          hitlRequest,
          onStateChange: (r: UseHitlInputResult) => {
            lastResult = r;
          },
        })
      );

      await new Promise(resolve => setTimeout(resolve, 10));

      // @step Then the TUI renders only a freeform text input
      expect(lastResult?.isCurrentQuestionFreeform).toBe(true);

      // @step And no "Other..." entry is displayed
      expect(lastResult?.isOtherActive).toBe(false);
      expect(lastResult?.showEmptyHint).toBe(false);
    });
  });
});
