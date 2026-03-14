/**
 * Feature: spec/features/hitl-handler-wiring.feature
 *
 * Part 3: InputTransition HITL rendering integration.
 * Real React renders with fixture data — verifies inline HITL UI output.
 *
 * BUG-118: HITL TUI integration
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { InputTransition } from '../components/InputTransition';
import type { HitlRequestInfo } from '../types/hitlRequest';

// ============================================================================
// Fixtures
// ============================================================================

const FIXTURE_OPTIONS: HitlRequestInfo = {
  questions: [{
    id: 'approach',
    header: 'Approach',
    question: 'Which approach do you prefer?',
    options: [
      { label: 'Option A', description: 'First approach — simple' },
      { label: 'Option B', description: 'Second approach — thorough' },
    ],
  }],
};

const FIXTURE_MULTI: HitlRequestInfo = {
  questions: [
    {
      id: 'priority',
      header: 'Priority',
      question: 'What is the priority level?',
      options: [
        { label: 'High', description: 'Do it now' },
        { label: 'Low', description: 'Do it later' },
      ],
    },
    {
      id: 'scope',
      header: 'Scope',
      question: 'What scope should this cover?',
      options: [
        { label: 'Minimal', description: 'Essentials only' },
        { label: 'Full', description: 'Everything possible' },
        { label: 'Custom', description: 'Let me specify' },
      ],
    },
  ],
};

const FIXTURE_FREEFORM: HitlRequestInfo = {
  questions: [{
    id: 'feedback',
    header: 'Feedback',
    question: 'Any additional feedback or context?',
  }],
};

// ============================================================================
// Tests
// ============================================================================

describe('HITL InputTransition Rendering Integration', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const baseProps = {
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    placeholder: 'Type a message...',
  };

  // ==========================================================================
  // Scenario: Renders HITL question with options inline
  // ==========================================================================

  describe('Scenario: Renders options question inline', () => {
    it('should show pause icon, header, text, and options', () => {
      // @step Given isPaused with hitlRequest containing options
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );

      const output = lastFrame() ?? '';

      // @step Then it should show ⏸ icon
      expect(output).toContain('⏸');

      // @step And question header and text
      expect(output).toContain('Approach');
      expect(output).toContain('Which approach do you prefer?');

      // @step And both options with descriptions
      expect(output).toContain('Option A');
      expect(output).toContain('First approach');
      expect(output).toContain('Option B');
      expect(output).toContain('Second approach');

      // @step And navigation hints
      expect(output).toMatch(/↑.*↓|Navigate/);
      expect(output).toMatch(/Enter/);
      expect(output).toMatch(/Esc/);
    });

    it('should show filled indicator on selected option', () => {
      // @step Given first option is selected
      const { lastFrame: f0 } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );
      expect(f0()).toContain('●');

      // @step Given second option is selected
      const { lastFrame: f1 } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 1,
        })
      );
      expect(f1()).toContain('●');
    });
  });

  // ==========================================================================
  // Scenario: Renders freeform-only HITL question
  // ==========================================================================

  describe('Scenario: Renders freeform question', () => {
    it('should show text input when no options', () => {
      // @step Given a freeform question (no options)
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_FREEFORM,
          hitlQuestionIndex: 0,
          hitlSelectedOption: -1,
          hitlFreeformActive: true,
        })
      );

      const output = lastFrame() ?? '';

      // @step Then it should show ⏸ icon and question text
      expect(output).toContain('⏸');
      expect(output).toContain('Any additional feedback');

      // @step And a text input placeholder
      expect(output).toMatch(/type your answer|Enter.*Submit|Esc.*Cancel/i);
    });
  });

  // ==========================================================================
  // Scenario: Multi-step HITL advances through questions
  // ==========================================================================

  describe('Scenario: Multi-step question display', () => {
    it('should show [1/2] on first and [2/2] on second', () => {
      // @step Given multi-step request, on question 1
      const { lastFrame: f1 } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_MULTI,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );
      const o1 = f1() ?? '';
      expect(o1).toContain('1/2');
      expect(o1).toContain('What is the priority level?');
      expect(o1).toContain('High');

      // @step Given on question 2
      const { lastFrame: f2 } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_MULTI,
          hitlQuestionIndex: 1,
          hitlSelectedOption: 0,
        })
      );
      const o2 = f2() ?? '';
      expect(o2).toContain('2/2');
      expect(o2).toContain('What scope should this cover?');
      expect(o2).toContain('Minimal');
      expect(o2).toContain('Custom');
    });

    it('should NOT show [1/1] for single question', () => {
      // @step Given a single-question request
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );
      expect(lastFrame()).not.toContain('1/1');
    });
  });

  // ==========================================================================
  // Scenario: HITL UI priority over other states
  // ==========================================================================

  describe('Scenario: HITL UI priority', () => {
    it('should show HITL instead of Thinking when paused with hitlRequest', () => {
      // @step Given isLoading=true AND isPaused=true with hitlRequest
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: true,
          isPaused: true,
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );

      const output = lastFrame() ?? '';
      expect(output).toContain('⏸');
      expect(output).toContain('Approach');
      expect(output).not.toContain('Thinking...');
    });

    it('should show Thinking when paused without pauseInfo or hitlRequest', () => {
      // @step Given isPaused but no pauseInfo and no hitlRequest
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: true,
          isPaused: true,
        })
      );
      expect(lastFrame()).toContain('Thinking...');
    });

    it('should show hitlRequest UI when both pauseInfo and hitlRequest present', () => {
      // @step Given both pauseInfo and hitlRequest (hitlRequest takes priority)
      const { lastFrame } = render(
        React.createElement(InputTransition, {
          ...baseProps,
          isLoading: false,
          isPaused: true,
          pauseInfo: {
            kind: 'continue' as const,
            toolName: 'Bash',
            message: 'Command completed',
          },
          hitlRequest: FIXTURE_OPTIONS,
          hitlQuestionIndex: 0,
          hitlSelectedOption: 0,
        })
      );

      const output = lastFrame() ?? '';
      // HITL UI should render, not the pauseInfo UI
      expect(output).toContain('Approach');
      expect(output).not.toContain('Command completed');
    });
  });
});
