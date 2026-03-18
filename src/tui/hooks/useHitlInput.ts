/**
 * BUG-118 + TOOL-018: HITL input state management hook
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
 * - TOOL-018: "Other..." freeform fallback for questions WITH options
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
  /** Currently selected option index for questions with options (includes virtual "Other..." at end) */
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
  /** TOOL-018: Whether the user selected "Other..." and is in freeform text entry mode */
  isOtherActive: boolean;
  /** TOOL-018: Whether to show the "Please type a response" hint after empty submit */
  showEmptyHint: boolean;
}

/**
 * Hook that manages HITL input state and keyboard handling.
 *
 * Registers a HIGH-priority useInputCompat handler for:
 * - ↑/↓: Navigate options including virtual "Other..." entry (wrapping)
 * - Enter: Select option / activate Other / capture freeform / advance / submit all
 * - Esc: Cancel HITL request, or return from Other freeform to option list
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
  // TOOL-018: "Other..." freeform mode state
  const [isOtherActive, setIsOtherActive] = useState(false);
  const [showEmptyHint, setShowEmptyHint] = useState(false);

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
      setIsOtherActive(false);
      setShowEmptyHint(false);
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

  /** Advance to next question or submit all answers */
  const advanceOrSubmit = useCallback(
    (answer: HitlAnswer) => {
      const newAnswers = [...answers, answer];

      if (hitlRequest && questionIndex < hitlRequest.questions.length - 1) {
        // Advance to next question
        setAnswers(newAnswers);
        setQuestionIndex(prev => prev + 1);
        setSelectedOption(0);
        setIsOtherActive(false);
        setShowEmptyHint(false);
      } else {
        // Last question — submit all answers
        handleSubmitAll(newAnswers);
      }
    },
    [answers, questionIndex, hitlRequest, handleSubmitAll]
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

      // TOOL-018: When in "Other..." freeform mode, Escape goes back to option list
      if (key.escape && isOtherActive) {
        setIsOtherActive(false);
        setShowEmptyHint(false);
        clearInputValue();
        return true;
      }

      // Escape: cancel the entire HITL request
      if (key.escape) {
        handleCancel();
        return true;
      }

      // TOOL-018: When in "Other..." freeform mode, handle Enter for submission
      if (isOtherActive && key.return) {
        if (inputValue.trim() === '') {
          // Reject empty submission
          setShowEmptyHint(true);
          return true;
        }
        // Submit as freeform answer
        setShowEmptyHint(false);
        const answer: HitlAnswer = {
          id: currentQuestion.id,
          selected: [],
          other: inputValue,
        };
        clearInputValue();
        advanceOrSubmit(answer);
        return true;
      }

      // TOOL-018: When in "Other..." freeform mode, let character input through
      if (isOtherActive) {
        // Clear hint on any typing
        if (showEmptyHint) {
          setShowEmptyHint(false);
        }
        return false;
      }

      // Up/Down: navigate options including virtual "Other..." entry
      if (hasOptions && currentQuestion.options) {
        // TOOL-018: Total items = options + 1 for "Other..."
        const totalItems = currentQuestion.options.length + 1;

        if (key.upArrow) {
          setSelectedOption(prev => (prev > 0 ? prev - 1 : totalItems - 1));
          return true;
        }
        if (key.downArrow) {
          setSelectedOption(prev => (prev < totalItems - 1 ? prev + 1 : 0));
          return true;
        }
      }

      // Enter: select option / activate Other / capture freeform / advance / submit
      if (key.return) {
        if (hasOptions && currentQuestion.options) {
          // TOOL-018: Check if "Other..." is selected (last index)
          if (selectedOption === currentQuestion.options.length) {
            // Activate "Other..." freeform mode
            setIsOtherActive(true);
            setShowEmptyHint(false);
            return true;
          }

          // Option question: capture selected option label
          const answer: HitlAnswer = {
            id: currentQuestion.id,
            selected: [currentQuestion.options[selectedOption].label],
          };
          advanceOrSubmit(answer);
        } else {
          // Freeform question: capture current input value as `other`
          const answer: HitlAnswer = {
            id: currentQuestion.id,
            selected: [],
            other: inputValue,
          };
          clearInputValue();
          advanceOrSubmit(answer);
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
    isOtherActive,
    showEmptyHint,
  };
}
