/**
 * Feature: spec/features/multilineinput-should-show-compaction-status-instead-of-conversation-message.feature
 * UX-002: MultiLineInput should show compaction status instead of conversation message
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { MultiLineInput } from '../MultiLineInput';
import { InputManager } from '../../input/InputManager';

describe('UX-002: MultiLineInput Compaction Status Display', () => {

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Input placeholder shows compaction progress instead of conversation message', () => {
    it('should show compaction status in placeholder when isCompacting=true', () => {
      const onChange = vi.fn();
      const onSubmit = vi.fn();
      
      // @step Given I have a conversation with multiple turns
      // @step When I type "/compact" and press Enter
      // (Compaction triggers isCompacting=true with progress)
      const { lastFrame } = render(
        <MultiLineInput 
          value=""
          onChange={onChange}
          onSubmit={onSubmit}
          isCompacting={true}
          compactionProgress={{
            phase: 'analyzing anchors',
            current: 15,
            total: 32
          }}
        />
      );

      // @step Then the input placeholder should show "Compacting: analyzing anchors... 15/32 turns"
      expect(lastFrame()).toContain('Compacting: analyzing anchors... 15/32 turns');

      // @step And the input area should remain visible and responsive
      expect(lastFrame()).toBeDefined();
    });
  });

  describe('Scenario: Input placeholder shows detailed compaction phases', () => {
    it('should show different phases in placeholder during compaction', () => {
      const onChange = vi.fn();
      
      // @step Given I have started a compaction process
      const { rerender, lastFrame } = render(
        <MultiLineInput 
          value=""
          onChange={onChange}
          onSubmit={vi.fn()}
          isCompacting={true}
          compactionProgress={{
            phase: 'analyzing anchors',
            current: 15,
            total: 32
          }}
        />
      );

      // @step When the compaction progresses through phases
      // @step Then the input placeholder should show "Compacting: analyzing anchors... 15/32 turns"
      expect(lastFrame()).toContain('Compacting: analyzing anchors... 15/32 turns');

      rerender(
        <MultiLineInput 
          value=""
          onChange={onChange}
          onSubmit={vi.fn()}
          isCompacting={true}
          compactionProgress={{
            phase: 'generating summary',
            current: 1,
            total: 1
          }}
        />
      );

      // @step And then it should show "Compacting: generating summary..."
      expect(lastFrame()).toContain('Compacting: generating summary... 1/1 turns');
    });
  });

  describe('Scenario: Input area blocks typing but shows progress during compaction', () => {
    it('should not accept text input during compaction but still show progress', async () => {
      const onChange = vi.fn();
      const onSubmit = vi.fn();
      
      // @step Given compaction is in progress
      const { stdin, lastFrame } = render(
        <InputManager>
          <MultiLineInput 
            value=""
            onChange={onChange}
            onSubmit={onSubmit}
            isCompacting={true}
            isActive={true}
            suppressEnter={true}
            compactionProgress={{
              phase: 'analyzing anchors',
              current: 10,
              total: 25
            }}
          />
        </InputManager>
      );

      // @step When I try to type characters in the input area
      stdin.write('hello');
      await new Promise(resolve => setTimeout(resolve, 20));

      // @step Then the characters should not be captured or displayed
      expect(onChange).not.toHaveBeenCalled();

      // @step And the input placeholder should continue showing compaction progress
      expect(lastFrame()).toContain('Compacting: analyzing anchors... 10/25 turns');

      // @step And I should not be able to submit messages
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 20));
      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('should block backspace and delete during compaction', async () => {
      const onChange = vi.fn();
      
      // @step Given compaction is in progress
      const { stdin } = render(
        <InputManager>
          <MultiLineInput 
            value="existing text"
            onChange={onChange}
            onSubmit={vi.fn()}
            isCompacting={true}
            isActive={true}
            compactionProgress={{
              phase: 'analyzing anchors',
              current: 10,
              total: 25
            }}
          />
        </InputManager>
      );

      // @step When I try to delete characters
      stdin.write('\x7f'); // Backspace
      await new Promise(resolve => setTimeout(resolve, 20));

      // @step Then the text should not be modified
      expect(onChange).not.toHaveBeenCalled();

      stdin.write('\x1b[3~'); // Delete key
      await new Promise(resolve => setTimeout(resolve, 20));
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Input area returns to normal state after compaction completes', () => {
    it('should return to normal placeholder and accept input when compaction finishes', async () => {
      const onChange = vi.fn();
      const onSubmit = vi.fn();
      
      // @step Given compaction is showing progress in the input placeholder
      const { stdin, rerender, lastFrame } = render(
        <InputManager>
          <MultiLineInput 
            value=""
            onChange={onChange}
            onSubmit={onSubmit}
            placeholder="Type a message..."
            isCompacting={true}
            isActive={true}
            suppressEnter={true}
            compactionProgress={{
              phase: 'generating summary',
              current: 1,
              total: 1
            }}
          />
        </InputManager>
      );

      expect(lastFrame()).toContain('Compacting: generating summary... 1/1 turns');

      // @step When the compaction process completes successfully
      rerender(
        <InputManager>
          <MultiLineInput 
            value=""
            onChange={onChange}
            onSubmit={onSubmit}
            placeholder="Type a message..."
            isCompacting={false}
            isActive={true}
            suppressEnter={false}
            compactionProgress={null}
          />
        </InputManager>
      );

      // @step Then the input placeholder should immediately return to "Type a message..."
      expect(lastFrame()).toContain('Type a message...');

      // @step And I should be able to type and submit messages normally
      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 20));
      expect(onChange).toHaveBeenCalled();

      onChange.mockClear();
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 20));
      expect(onSubmit).toHaveBeenCalled();
    });
  });

});
