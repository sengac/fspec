/**
 * Feature: spec/features/add-fspec-logo-to-tui-header.feature
 *
 * Tests for BOARD-019: Add fspec logo to TUI header
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { BoardView } from '../components/BoardView';
import { useFspecStore } from '../store/fspecStore';

describe('Feature: Add fspec logo to TUI header', () => {
  beforeEach(() => {
    // Reset store before each test
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
    });
  });

  describe('Scenario: Logo displays in header with correct ASCII art', () => {
    it('should display the three-line fspec ASCII logo', async () => {
      // Given I have a fspec project with work units
      useFspecStore.setState({
        workUnits: [
          {
            id: 'TEST-001',
            title: 'Test Work Unit',
            type: 'story',
            status: 'backlog',
            description: 'Test description',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T00:00:00.000Z',
          },
        ],
        isLoaded: true,
      });

      // When I launch the fspec TUI
      const { lastFrame } = render(<BoardView />);
      await new Promise(resolve => setTimeout(resolve, 100));

      const frame = lastFrame();

      // Then the logo line 1 should display "┏┓┏┓┏┓┏┓┏┓"
      expect(frame).toContain('┏┓┏┓┏┓┏┓┏┓');

      // And the logo line 2 should display "┣ ┗┓┃┃┣ ┃"
      expect(frame).toContain('┣ ┗┓┃┃┣ ┃');

      // And the logo line 3 should display "┻ ┗┛┣┛┗┛┗┛"
      expect(frame).toContain('┻ ┗┛┣┛┗┛┗┛');
    });
  });

  describe('Scenario: TUI uses hybrid rendering with table borders and Box content', () => {
    it('should render with Box borderStyle for header and table borders for columns', async () => {
      // Given I have a fspec project with work units
      useFspecStore.setState({
        workUnits: [
          {
            id: 'TEST-001',
            title: 'Test Work Unit',
            type: 'story',
            status: 'backlog',
            description: 'Test description',
            createdAt: '2025-01-01T00:00:00.000Z',
            updatedAt: '2025-01-01T00:00:00.000Z',
          },
        ],
        isLoaded: true,
      });

      // When I launch the fspec TUI
      const { lastFrame } = render(<BoardView />);
      await new Promise(resolve => setTimeout(resolve, 100));

      const frame = lastFrame();

      // Then header should use Box borderStyle (single border around header)
      expect(frame).toContain('┌'); // Top-left corner of header box
      expect(frame).toContain('│'); // Vertical borders of header box
      expect(frame).toContain('└'); // Bottom-left corner of header box

      // And logo should appear in header
      expect(frame).toContain('┏┓┏┓┏┓┏┓┏┓');

      // And Git panels should appear in header
      expect(frame).toContain('Git Stashes');
      expect(frame).toContain('Changed Files');

      // And table borders should be rendered for columns section
      expect(frame).toContain('├'); // Left T-junction
      expect(frame).toContain('┼'); // Cross junction
      expect(frame).toContain('┤'); // Right T-junction
      expect(frame).toContain('┬'); // Top T-junction
      expect(frame).toContain('┴'); // Bottom T-junction

      // And the Logo component should be a Box with fixed width
      const lines = frame.split('\n');
      const logoLine = lines.find(line => line.includes('┏┓┏┓┏┓┏┓┏┓'));
      expect(logoLine).toBeDefined();
      if (logoLine) {
        expect(logoLine.indexOf('┏┓┏┓┏┓┏┓┏┓')).toBeGreaterThanOrEqual(0);
      }
    });
  });
});
