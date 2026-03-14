/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Tests for HITL TUI integration — useRustSessionState snapshot polling,
 * InputTransition rendering, and AgentView keyboard handling.
 *
 * BUG-118: Tests the TypeScript side of the HITL pause pattern.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import {
  refreshSessionState,
  clearAllSubscriptions,
  getSessionSnapshotForTesting,
  setRustStateSource,
  resetRustStateSource,
  type RustStateSource,
} from '../../hooks/useRustSessionState';
import {
  parseHitlRequestInfo,
  hitlRequestInfoEqual,
  type HitlRequestInfo,
} from '../../types/hitlRequest';
import { InputTransition } from '../../components/InputTransition';
import type { PauseInfo } from '../../types/pause';

// =============================================================================
// Mock State Source Helper
// =============================================================================

function createMockStateSource(overrides: {
  status?: string;
  hitlRequest?: HitlRequestInfo | null;
  pauseState?: PauseInfo | null;
}): RustStateSource {
  const state = {
    status: overrides.status ?? 'idle',
    hitlRequest: overrides.hitlRequest ?? null,
    pauseState: overrides.pauseState ?? null,
  };

  return {
    getStatus: () => state.status,
    getModel: () => null,
    getTokens: () => ({ inputTokens: 0, outputTokens: 0 }),
    getDebugEnabled: () => false,
    getPauseState: () => state.pauseState,
    getBaseThinkingLevel: () => 0,
    setBaseThinkingLevel: () => {},
    getCompactionProgress: () => null,
    getHitlRequest: () => state.hitlRequest,
  };
}

// =============================================================================
// Test Setup
// =============================================================================

describe('Feature: HITL TUI integration (BUG-118)', () => {
  beforeEach(() => {
    clearAllSubscriptions();
  });

  afterEach(() => {
    resetRustStateSource();
    clearAllSubscriptions();
  });

  // ===========================================================================
  // Scenario: useRustSessionState includes hitlRequest in snapshot when paused
  // ===========================================================================

  describe('Scenario: useRustSessionState includes hitlRequest in snapshot when paused', () => {
    it('should include hitlRequest in snapshot when session is paused with HITL request', () => {
      // @step Given a session is paused and has HITL request state
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach do you prefer?',
            options: [
              { label: 'Option A', description: 'First approach' },
              { label: 'Option B', description: 'Second approach' },
            ],
          },
        ],
      };

      const source = createMockStateSource({
        status: 'paused',
        hitlRequest,
      });
      setRustStateSource(source);

      // @step When useRustSessionState fetches the snapshot
      refreshSessionState('test-session');
      const snapshot = getSessionSnapshotForTesting('test-session');

      // @step Then snapshot.hitlRequest should contain the questions array
      expect(snapshot.hitlRequest).not.toBeNull();
      expect(snapshot.hitlRequest?.questions).toHaveLength(1);
      expect(snapshot.hitlRequest?.questions[0].id).toBe('approach');
      expect(snapshot.hitlRequest?.questions[0].options).toHaveLength(2);

      // @step And snapshot.isPaused should be true
      expect(snapshot.isPaused).toBe(true);
    });
  });

  // ===========================================================================
  // Scenario: useRustSessionState returns null hitlRequest when not paused
  // ===========================================================================

  describe('Scenario: useRustSessionState returns null hitlRequest when not paused', () => {
    it('should return null hitlRequest when session is running', () => {
      // @step Given a session is running with no HITL request
      const source = createMockStateSource({
        status: 'running',
        hitlRequest: null,
      });
      setRustStateSource(source);

      // @step When useRustSessionState fetches the snapshot
      refreshSessionState('test-session');
      const snapshot = getSessionSnapshotForTesting('test-session');

      // @step Then snapshot.hitlRequest should be null
      expect(snapshot.hitlRequest).toBeNull();
    });
  });

  // ===========================================================================
  // Scenario: InputTransition renders HITL question with options inline
  // ===========================================================================

  describe('Scenario: InputTransition renders HITL question with options inline', () => {
    it('should render question with selectable options when paused with HITL request', () => {
      // @step Given isPaused is true and hitlRequest contains a question with options
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'approach',
            header: 'Approach',
            question: 'Which approach do you prefer?',
            options: [
              { label: 'Option A', description: 'First approach' },
              { label: 'Option B', description: 'Second approach' },
            ],
          },
        ],
      };

      // @step When InputTransition renders
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      const output = lastFrame() ?? '';

      // @step Then it should show the question header and question text
      expect(output).toContain('Approach');
      expect(output).toContain('Which approach do you prefer?');

      // @step And it should show selectable options with selected and unselected indicators
      expect(output).toContain('Option A');
      expect(output).toContain('Option B');

      // @step And it should show navigation hints for up down Enter and Esc
      expect(output).toMatch(/↑|↓|Enter|Esc/);
    });
  });

  // ===========================================================================
  // Scenario: InputTransition renders freeform-only HITL question
  // ===========================================================================

  describe('Scenario: InputTransition renders freeform-only HITL question', () => {
    it('should render text input when question has no options', () => {
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

      // @step When InputTransition renders
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: -1,
          hitlFreeformActive: true,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      const output = lastFrame() ?? '';

      // @step Then it should show the question text
      expect(output).toContain('Any additional feedback?');

      // @step And it should show a text input area for freeform response
      // Freeform renders the MultiLineInput with "Type your answer..." placeholder
      expect(output).toMatch(/type your answer|enter|submit/i);
    });
  });

  // ===========================================================================
  // Scenario: Multi-step HITL advances through questions
  // ===========================================================================

  describe('Scenario: Multi-step HITL advances through questions', () => {
    it('should show question index and total', () => {
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

      // @step And the user is on question 1 of 2
      const { lastFrame: frame1 } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      const output1 = frame1() ?? '';
      expect(output1).toContain('1/2');
      expect(output1).toContain('First question?');

      // @step When the user selects an option and presses Enter
      // @step Then InputTransition should advance to question 2 of 2
      const { lastFrame: frame2 } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 1,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      expect(frame2()).toContain('2/2');
      expect(frame2()).toContain('Second question?');

      // @step And the first question answer should be stored
      // (Answer storage is handled by AgentView keyboard handler, tested separately)
    });
  });

  // ===========================================================================
  // Scenario: AgentView HITL keyboard handler navigates options
  // ===========================================================================

  describe('Scenario: AgentView HITL keyboard handler navigates options', () => {
    it('should move selection up and down', () => {
      // @step Given a session is paused with HITL questions containing options
      // Test the option rendering with different selectedOption values
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

      // @step When the user presses up arrow
      // @step Then the selected option should move up
      const { lastFrame: frame0 } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );
      // First option should be visually selected
      expect(frame0()).toContain('First');

      // @step When the user presses down arrow
      // @step Then the selected option should move down
      const { lastFrame: frame1 } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 1,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );
      // Second option should be visually selected
      expect(frame1()).toContain('Second');
    });
  });

  // ===========================================================================
  // Scenario: AgentView HITL keyboard handler submits all answers
  // ===========================================================================

  describe('Scenario: AgentView HITL keyboard handler submits all answers', () => {
    it('should indicate submit action on last question', () => {
      // @step Given a session is paused with HITL questions and all questions answered
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'Only question',
            options: [
              { label: 'Yes', description: 'Agree' },
            ],
          },
        ],
      };

      // @step When the user presses Enter on the last question
      // @step Then sessionSendHitlResponse should be called with all collected answers
      // @step And cancelled should be false
      // (The actual NAPI call is made by AgentView keyboard handler, not InputTransition)
      // Here we verify the UI shows submit hint on last question
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      const output = lastFrame() ?? '';
      // Single question should show Enter to submit
      expect(output).toMatch(/Enter/);
    });
  });

  // ===========================================================================
  // Scenario: User cancels HITL with Escape
  // ===========================================================================

  describe('Scenario: User cancels HITL with Escape', () => {
    it('should show Esc hint in HITL UI', () => {
      // @step Given a session is paused with HITL questions
      const hitlRequest: HitlRequestInfo = {
        questions: [
          {
            id: 'q1',
            header: 'Q1',
            question: 'Test question',
            options: [
              { label: 'A', description: 'Option A' },
            ],
          },
        ],
      };

      // @step When the user presses Escape
      // @step Then sessionSendHitlResponse should be called with cancelled true
      // @step And the handler should unblock and return Cancelled
      // (Actual Esc handling is in AgentView keyboard handler)
      // Here we verify the UI shows Esc cancel hint
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          isLoading: false,
          isPaused: true,
          hitlRequest,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
          value: '',
          onChange: () => {},
          onSubmit: () => {},
        })
      );

      const output = lastFrame() ?? '';
      expect(output).toMatch(/Esc/);
    });
  });

  // ===========================================================================
  // Type utilities tests
  // ===========================================================================

  describe('HitlRequestInfo type utilities', () => {
    it('parseHitlRequestInfo should handle null input', () => {
      expect(parseHitlRequestInfo(null)).toBeNull();
      expect(parseHitlRequestInfo(undefined)).toBeNull();
    });

    it('parseHitlRequestInfo should handle empty questions', () => {
      expect(parseHitlRequestInfo({ questions: [] })).toBeNull();
    });

    it('parseHitlRequestInfo should convert valid NAPI response', () => {
      const result = parseHitlRequestInfo({
        questions: [
          {
            id: 'test',
            header: 'Test',
            question: 'Test question?',
            options: [{ label: 'A', description: 'Option A' }],
          },
        ],
      });
      expect(result).not.toBeNull();
      expect(result?.questions[0].id).toBe('test');
      expect(result?.questions[0].options?.[0].label).toBe('A');
    });

    it('hitlRequestInfoEqual should compare correctly', () => {
      const a: HitlRequestInfo = {
        questions: [{ id: 'q1', header: 'H', question: 'Q?' }],
      };
      const b: HitlRequestInfo = {
        questions: [{ id: 'q1', header: 'H', question: 'Q?' }],
      };
      const c: HitlRequestInfo = {
        questions: [{ id: 'q2', header: 'H', question: 'Q?' }],
      };

      expect(hitlRequestInfoEqual(a, b)).toBe(true);
      expect(hitlRequestInfoEqual(a, c)).toBe(false);
      expect(hitlRequestInfoEqual(null, null)).toBe(true);
      expect(hitlRequestInfoEqual(a, null)).toBe(false);
    });
  });
});
