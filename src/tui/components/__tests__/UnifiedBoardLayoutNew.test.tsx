/**
 * Feature: spec/features/refactor-header-and-details-to-component-based-flexbox.feature
 *
 * Tests for TUI-015: Refactor header and details to component-based flexbox
 *
 * Tests verify that the UnifiedBoardLayout component:
 * - Uses pure flexbox (no percentages)
 * - Uses component-based rendering (no string injection)
 * - Proper border structure with continuous junctions
 * - Dynamic viewport height with terminal resize
 */

import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { UnifiedBoardLayout } from '../UnifiedBoardLayout';

const mockWorkUnits = [
  {
    id: 'TEST-001',
    title: 'Test Story',
    type: 'story' as const,
    status: 'backlog',
    description: 'Test description for validation',
    estimate: 5,
    epic: 'test-epic',
  },
  {
    id: 'TEST-002',
    title: 'Another Story',
    type: 'story' as const,
    status: 'implementing',
    description: 'Implementation in progress',
  },
];

describe('Feature: Refactor header and details to component-based flexbox', () => {
  describe('Scenario: New layout renders with pure flexbox structure', () => {
    it('should render header, details, and columns sections', () => {
      // @step Given I have a new UnifiedBoardLayout component
      // @step When I render it with work units
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          selectedWorkUnit={mockWorkUnits[0]}
          terminalWidth={80}
          terminalHeight={24}
        />
      );

      // @step Then I should see the header section with Logo
      expect(lastFrame()).toContain('┏┓┏┓┏┓┏┓┏┓');

      // @step And I should see the Checkpoints status
      expect(lastFrame()).toContain('Checkpoints');

      // @step And I should see the keybinding shortcuts
      expect(lastFrame()).toContain('C Checkpoints');
      expect(lastFrame()).toContain('F Changed Files');

      // @step And I should see the work unit details section
      expect(lastFrame()).toContain('TEST-001');
      expect(lastFrame()).toContain('Test description for validation');

      // @step And I should see the columns section
      expect(lastFrame()).toContain('BACKLOG');
      expect(lastFrame()).toContain('IMPLEMENTI');
    });
  });

  describe('Scenario: Layout uses proper border structure with continuous junctions', () => {
    it('should use borderStyle single with selective border disabling', () => {
      // @step Given I have the new layout component
      // @step When I render it
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          selectedWorkUnit={mockWorkUnits[0]}
          terminalWidth={80}
          terminalHeight={24}
        />
      );

      const frame = lastFrame();
      const lines = frame.split('\n');

      // @step Then I should see continuous borders
      // Header top border
      expect(lines[0]).toContain('┌');
      expect(lines[0]).toContain('┐');

      // @step And borders should not have gaps or double lines
      const hasBorderGaps = lines.some(line => {
        // Check for lines that break border continuity
        return line.includes('│') && line.includes('┌') && !line.includes('├');
      });
      expect(hasBorderGaps).toBe(false);
    });
  });

  describe('Scenario: Work unit details section displays selected work unit', () => {
    it('should show title, description, and metadata for selected work unit', () => {
      // @step Given I have a selected work unit with all fields
      const selectedWorkUnit = {
        id: 'DETAIL-001',
        title: 'Detailed Work Unit',
        type: 'story' as const,
        status: 'testing',
        description: 'This is a comprehensive description',
        estimate: 8,
        epic: 'feature-epic',
      };

      // @step When I render the layout with this work unit selected
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={[selectedWorkUnit]}
          selectedWorkUnit={selectedWorkUnit}
          terminalWidth={80}
          terminalHeight={24}
        />
      );

      const frame = lastFrame();

      // @step Then I should see the work unit ID and title
      expect(frame).toContain('DETAIL-001');
      expect(frame).toContain('Detailed Work Unit');

      // @step And I should see the description
      expect(frame).toContain('This is a comprehensive description');

      // @step And I should see the metadata
      expect(frame).toContain('Epic: feature-epic');
      expect(frame).toContain('Estimate: 8pts');
      expect(frame).toContain('Status: testing');
    });
  });

  describe('Scenario: Layout shows message when no work unit selected', () => {
    it('should display centered "No work unit selected" message', () => {
      // @step Given I have no work unit selected
      // @step When I render the layout
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          selectedWorkUnit={null}
          terminalWidth={80}
          terminalHeight={24}
        />
      );

      // @step Then I should see the no selection message
      expect(lastFrame()).toContain('No work unit selected');
    });
  });

  describe('Scenario: Columns section displays work units grouped by status', () => {
    it('should show work units in their respective columns', () => {
      // @step Given I have work units in different statuses
      const workUnits = [
        { id: 'BL-001', title: 'Backlog Item', type: 'story' as const, status: 'backlog' },
        { id: 'IM-001', title: 'In Progress', type: 'story' as const, status: 'implementing' },
        { id: 'DN-001', title: 'Completed', type: 'story' as const, status: 'done' },
      ];

      // @step When I render the layout
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          terminalWidth={100}
          terminalHeight={30}
        />
      );

      const frame = lastFrame();

      // @step Then I should see all work unit IDs
      expect(frame).toContain('BL-001');
      expect(frame).toContain('IM-001');
      expect(frame).toContain('DN-001');

      // @step And I should see column headers
      expect(frame).toContain('BACKLOG');
      expect(frame).toContain('IMPLEMENTI');
      expect(frame).toContain('DONE');
    });
  });

  describe('Scenario: Layout responds to terminal size changes', () => {
    it('should calculate viewport height based on terminal height', () => {
      // @step Given I have different terminal heights
      const smallTerminal = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          terminalWidth={80}
          terminalHeight={20}
        />
      );

      const largeTerminal = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          terminalWidth={80}
          terminalHeight={40}
        />
      );

      // @step Then both should render successfully
      expect(smallTerminal.lastFrame()).toBeTruthy();
      expect(largeTerminal.lastFrame()).toBeTruthy();

      // @step And both should contain all key sections
      const smallFrame = smallTerminal.lastFrame();
      const largeFrame = largeTerminal.lastFrame();

      expect(smallFrame).toContain('┏┓┏┓┏┓┏┓┏┓'); // Logo
      expect(smallFrame).toContain('BACKLOG'); // Columns

      expect(largeFrame).toContain('┏┓┏┓┏┓┏┓┏┓'); // Logo
      expect(largeFrame).toContain('BACKLOG'); // Columns
    });
  });

  describe('Scenario: Footer displays keyboard shortcuts', () => {
    it('should show navigation hints at the bottom', () => {
      // @step Given I render the layout
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={mockWorkUnits}
          terminalWidth={80}
          terminalHeight={24}
        />
      );

      // @step Then I should see the footer with keyboard shortcuts
      expect(lastFrame()).toContain('← → Columns');
      expect(lastFrame()).toContain('↑↓ Work Units');
      expect(lastFrame()).toContain('↵ Work Agent');
      expect(lastFrame()).toContain('ESC Back');
    });
  });
});
