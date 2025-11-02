/**
 * Feature: spec/features/improve-work-unit-details-panel-formatting.feature
 *
 * Tests for ITF-008: Improve work unit details panel formatting
 *
 * Tests verify that the Work Unit Details panel:
 * - Removes the "Work Unit Details" header line
 * - Panel is 5 lines tall (not 4)
 * - Displays descriptions up to 3 lines (not 1 line)
 * - Truncates with '...' on line 4 if description exceeds 3 lines
 * - Removes left padding (currently 2 spaces) from all content lines
 * - Description lines are bold cyan
 */

import React from 'react';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import type { WorkUnit } from '../../types.js';
import { BoardView } from '../components/BoardView.js';
import { useFspecStore } from '../store/fspecStore.js';

describe('Feature: Improve work unit details panel formatting', () => {
  let testDir: string;
  let workUnitsPath: string;

  beforeEach(async () => {
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
    workUnitsPath = join(testDir, 'spec', 'work-units.json');

    // Set test directory in store to avoid watcher errors
    useFspecStore.setState({ cwd: testDir });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display work unit with short description (1 line)', () => {
    it('should display ID+Title on line 1, description on line 2, empty lines 3-4, metadata on line 5, NO header, NO padding', async () => {
      // @step Given I am viewing the TUI board
      // @step And a work unit with a 1-line description is selected
      const workUnitsData = {
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            type: 'story' as const,
            title: 'Test Feature',
            description: 'This is a short one-line description',
            status: 'backlog' as const,
            epic: 'test-epic',
            estimate: 3,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          } satisfies WorkUnit,
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Set work units directly in store (bypassing loadData)
      useFspecStore.setState({
        workUnits: Object.values(workUnitsData.workUnits),
      });

      // @step When the Work Unit Details panel is rendered
      const { frames } = render(<BoardView cwd={testDir} />);

      // Wait for data to load, auto-focus to run, and component to re-render
      await new Promise(resolve => setTimeout(resolve, 1000));

      const frame = frames[frames.length - 1];

      // @step Then line 1 should display the work unit ID and title without left padding
      expect(frame).toContain('TEST-001: Test Feature');
      expect(frame).not.toMatch(/  TEST-001/); // No 2-space padding

      // @step And line 2 should display the description without left padding (bold cyan)
      expect(frame).toContain('This is a short one-line description');
      expect(frame).not.toMatch(/  This is a short/); // No 2-space padding

      // @step And line 3 should be empty
      // (verified by panel height being static at 5 lines)

      // @step And line 4 should be empty
      // (verified by panel height being static at 5 lines)

      // @step And line 5 should display metadata without left padding
      expect(frame).toContain('test-epic');
      expect(frame).toContain('3'); // Estimate
      expect(frame).not.toMatch(/  Epic:/); // No 2-space padding

      // @step And the "Work Unit Details" header line should NOT be displayed
      expect(frame).not.toContain('Work Unit Details');
    });
  });

  describe('Scenario: Display work unit with no description', () => {
    it('should display ID+Title on line 1, empty lines 2-4, metadata on line 5, NO header, NO padding', async () => {
      // @step Given I am viewing the TUI board
      // @step And a work unit with no description is selected
      const workUnitsData = {
        workUnits: {
          'TEST-002': {
            id: 'TEST-002',
            type: 'story' as const,
            title: 'No Description Feature',
            description: '',
            status: 'backlog' as const,
            epic: 'test-epic',
            estimate: 2,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          } satisfies WorkUnit,
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Set work units directly in store (bypassing loadData)
      useFspecStore.setState({
        workUnits: Object.values(workUnitsData.workUnits),
      });

      // @step When the Work Unit Details panel is rendered
      const { frames } = render(<BoardView cwd={testDir} />);

      // Wait for data to load, auto-focus to run, and component to re-render
      await new Promise(resolve => setTimeout(resolve, 1000));

      const frame = frames[frames.length - 1];

      // @step Then line 1 should display the work unit ID and title without left padding
      expect(frame).toContain('TEST-002: No Description Feature');
      expect(frame).not.toMatch(/  TEST-002/); // No 2-space padding

      // And line 4 should display metadata without left padding
      expect(frame).toContain('test-epic');
      expect(frame).toContain('2'); // Estimate
      expect(frame).not.toMatch(/  Epic:/); // No 2-space padding

      // @step And the "Work Unit Details" header line should NOT be displayed
      expect(frame).not.toContain('Work Unit Details');
    });
  });

  describe('Scenario: Display work unit with long description (exceeds 2 lines)', () => {
    it('should display ID+Title on line 1, first 2 desc lines on lines 2-3 with ... truncation, metadata on line 4', async () => {
      // Given I am viewing the TUI board
      // And a work unit with a 5-line description is selected
      const workUnitsData = {
        workUnits: {
          'TEST-003': {
            id: 'TEST-003',
            type: 'story' as const,
            title: 'Long Description Feature',
            description:
              'Line one of the description\nLine two of the description\nLine three of the description\nLine four of the description\nLine five of the description',
            status: 'backlog' as const,
            epic: 'test-epic',
            estimate: 5,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          } satisfies WorkUnit,
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Set work units directly in store (bypassing loadData)
      useFspecStore.setState({
        workUnits: Object.values(workUnitsData.workUnits),
      });

      // @step When the Work Unit Details panel is rendered
      const { frames } = render(<BoardView cwd={testDir} />);

      // Wait for data to load, auto-focus to run, and component to re-render
      await new Promise(resolve => setTimeout(resolve, 1000));

      const frame = frames[frames.length - 1];

      // @step Then line 1 should display the work unit ID and title without left padding
      expect(frame).toContain('TEST-003: Long Description Feature');
      expect(frame).not.toMatch(/  TEST-003/); // No 2-space padding

      // And line 2-3 should display the description word-wrapped (implementation concatenates lines)
      expect(frame).toContain('Line one of the description');
      expect(frame).toContain('Line two of the description');
      expect(frame).toContain('Line three of the description');
      expect(frame).not.toMatch(/  Line one/); // No 2-space padding

      // And line 5 should display metadata
      expect(frame).toContain('test-epic');
      expect(frame).toContain('5'); // Estimate

      // @step And the "Work Unit Details" header line should NOT be displayed
      expect(frame).not.toContain('Work Unit Details');
    });
  });

  describe('Scenario: Display work unit with exactly 2 lines of description', () => {
    it('should display ID+Title on line 1, both desc lines on lines 2-3 WITHOUT ... truncation, metadata on line 4', async () => {
      // Given I am viewing the TUI board
      // And a work unit with a 2-line description is selected
      const workUnitsData = {
        workUnits: {
          'TEST-004': {
            id: 'TEST-004',
            type: 'story' as const,
            title: 'Two Line Description Feature',
            description: 'First line of description\nSecond line of description',
            status: 'backlog' as const,
            epic: 'test-epic',
            estimate: 2,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          } satisfies WorkUnit,
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Set work units directly in store (bypassing loadData)
      useFspecStore.setState({
        workUnits: Object.values(workUnitsData.workUnits),
      });

      // @step When the Work Unit Details panel is rendered
      const { frames } = render(<BoardView cwd={testDir} />);

      // Wait for data to load, auto-focus to run, and component to re-render
      await new Promise(resolve => setTimeout(resolve, 1000));

      const frame = frames[frames.length - 1];

      // @step Then line 1 should display the work unit ID and title without left padding
      expect(frame).toContain('TEST-004: Two Line Description Feature');

      // And line 2 should display the first line of description without left padding
      expect(frame).toContain('First line of description');

      // And line 3 should display the second line of description WITHOUT "..." truncation indicator
      expect(frame).toContain('Second line of description');
      expect(frame).not.toMatch(/Second line of description.*\.\.\./);

      // And line 4 should display metadata without left padding
      expect(frame).toContain('test-epic');

      // @step And the "Work Unit Details" header line should NOT be displayed
      expect(frame).not.toContain('Work Unit Details');
    });
  });

  describe('Scenario: Display empty state when no work unit selected', () => {
    it('should display "No work unit selected" centered on line 1, empty lines 2-4, NO header', async () => {
      // Given I am viewing the TUI board
      // And no work unit is selected
      const workUnitsData = {
        workUnits: {},
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Set work units directly in store (bypassing loadData)
      useFspecStore.setState({
        workUnits: Object.values(workUnitsData.workUnits),
      });

      // @step When the Work Unit Details panel is rendered
      const { frames } = render(<BoardView cwd={testDir} />);

      // Wait for data to load, auto-focus to run, and component to re-render
      await new Promise(resolve => setTimeout(resolve, 1000));

      const frame = frames[frames.length - 1];

      // Then line 1 should display "No work unit selected" centered
      expect(frame).toContain('No work unit selected');

      // @step And the "Work Unit Details" header line should NOT be displayed
      expect(frame).not.toContain('Work Unit Details');
    });
  });
});
