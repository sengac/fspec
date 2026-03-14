/**
 * BUG-118: HITL input state management hook
 *
 * Extracts all HITL (Human-In-The-Loop) state and keyboard handling
 * from AgentView into a composable, testable hook.
 *
 * Manages:
 * - Current question index in multi-step HITL flows
 * - Selected option index for questions with options
 * - Accumulated answers across multi-step questions
 * - Keyboard navigation (↑/↓/Enter/Esc)
 * - Freeform text capture for questions without options
 */

import { useState, useEffect, useCallback } from 'react';
import { sessionSendHitlResponse } from '@sengac/codelet-napi';
import type { HitlRequestInfo } from '../types/hitlRequest';
import { useInputCompat, InputPriority } from '../input/index';
import { logger } from '../../utils/logger';

export interface HitlAnswer {
  id: string;
  selected: string[];
  other?: string;
}

export interface HitlInputState {
  /** Current question index (0-based) */
  questionIndex: number;
  /** Currently selected option index for questions with options */
  selectedOption: number;
  /** Accumulated answers from completed questions */
  answers: HitlAnswer[];
}

export interface UseHitlInputOptions {
  /** Current session ID (null if no session) */
  sessionId: string | null;
  /** Whether session is paused */
  isPaused: boolean;
  /** HITL request info from Rust snapshot (null when not in HITL) */
  hitlRequest: HitlRequestInfo | null;
  /** Current input value from MultiLineInput (used for freeform text capture) */
  inputValue: string;
  /** Callback to clear input value after freeform capture */
  clearInputValue: () => void;
}

export interface UseHitlInputResult {
  /** Current HITL input state */
  state: HitlInputState;
  /** Whether HITL is currently active (paused + request present) */
  isActive: boolean;
  /** Whether the current question is freeform (no options) */
  isCurrentQuestionFreeform: boolean;
}

/**
 * Hook that manages HITL input state and keyboard handling.
 *
 * Registers a HIGH-priority useInputCompat handler for:
 * - ↑/↓: Navigate options (wrapping)
 * - Enter: Select option / capture freeform / advance / submit all
 * - Esc: Cancel entire HITL request
 */
export function useHitlInput({
  sessionId,
  isPaused,
  hitlRequest,
  inputValue,
  clearInputValue,
}: UseHitlInputOptions): UseHitlInputResult {
  const [questionIndex, setQuestionIndex] = useState(0);
  const [selectedOption, setSelectedOption] = useState(0);
  const [answers, setAnswers] = useState<HitlAnswer[]>([]);

  const isActive = isPaused && hitlRequest !== null && sessionId !== null;
  const currentQuestion = hitlRequest?.questions[questionIndex] ?? null;
  const isCurrentQuestionFreeform =
    isActive &&
    currentQuestion !== null &&
    (!currentQuestion.options || currentQuestion.options.length === 0);

  // Reset state when HITL request ends or changes
  useEffect(() => {
    if (!isPaused || !hitlRequest) {
      setQuestionIndex(0);
      setSelectedOption(0);
      setAnswers([]);
    }
  }, [isPaused, hitlRequest]);

  const handleCancel = useCallback(() => {
    if (!sessionId) {
      return;
    }
    try {
      sessionSendHitlResponse(sessionId, {
        cancelled: true,
      });
    } catch (e) {
      logger.error('[BUG-118] Error sending HITL cancellation:', e);
    }
  }, [sessionId]);

  const handleSubmitAll = useCallback(
    (finalAnswers: HitlAnswer[]) => {
      if (!sessionId) {
        return;
      }
      try {
        sessionSendHitlResponse(sessionId, {
          cancelled: false,
          answers: finalAnswers,
        });
      } catch (e) {
        logger.error('[BUG-118] Error sending HITL response:', e);
      }
    },
    [sessionId]
  );

  // HIGH-priority keyboard handler for HITL navigation
  useInputCompat({
    id: 'hitl-input-handler',
    priority: InputPriority.HIGH,
    description:
      'HITL keyboard handler (↑/↓ navigate, Enter select, Esc cancel)',
    isActive,
    handler: (_input, key) => {
      if (!sessionId || !hitlRequest || !currentQuestion) {
        return false;
      }

      const hasOptions =
        currentQuestion.options && currentQuestion.options.length > 0;

      // Escape: cancel the entire HITL request
      if (key.escape) {
        handleCancel();
        return true;
      }

      // Up/Down: navigate options (only when question has options)
      if (hasOptions && currentQuestion.options) {
        if (key.upArrow) {
          setSelectedOption(prev =>
            prev > 0 ? prev - 1 : currentQuestion.options!.length - 1
          );
          return true;
        }
        if (key.downArrow) {
          setSelectedOption(prev =>
            prev < currentQuestion.options!.length - 1 ? prev + 1 : 0
          );
          return true;
        }
      }

      // Enter: select option / capture freeform / advance / submit
      if (key.return) {
        let answer: HitlAnswer;

        if (hasOptions && currentQuestion.options) {
          // Option question: capture selected option label
          answer = {
            id: currentQuestion.id,
            selected: [currentQuestion.options[selectedOption].label],
          };
        } else {
          // Freeform question: capture current input value as `other`
          answer = {
            id: currentQuestion.id,
            selected: [],
            other: inputValue,
          };
          clearInputValue();
        }

        const newAnswers = [...answers, answer];

        if (questionIndex < hitlRequest.questions.length - 1) {
          // Advance to next question
          setAnswers(newAnswers);
          setQuestionIndex(prev => prev + 1);
          setSelectedOption(0);
        } else {
          // Last question — submit all answers
          handleSubmitAll(newAnswers);
        }
        return true;
      }

      // For freeform questions, let character input through to MultiLineInput
      if (!hasOptions) {
        return false;
      }

      return false;
    },
  });

  return {
    state: {
      questionIndex,
      selectedOption,
      answers,
    },
    isActive,
    isCurrentQuestionFreeform,
  };
}
