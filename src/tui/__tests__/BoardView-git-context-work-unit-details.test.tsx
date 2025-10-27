/**
 * Feature: spec/features/consolidate-git-info-and-add-work-unit-details-panel.feature
 *
 * Tests for BOARD-007: Consolidate Git info and add work unit details panel
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import * as git from 'isomorphic-git';
import { getStagedFiles, getUnstagedFiles } from '../../git/status';

// Import components
import { BoardView } from '../components/BoardView';

// Mock git utilities
vi.mock('isomorphic-git');
vi.mock('../../git/status');

describe('Feature: Consolidate Git info and add work unit details panel', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for test isolation
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Reset mocks
    vi.clearAllMocks();
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Git Context panel combines stashes and changed files', () => {
    it('should display git stashes and changed files in single panel', async () => {
      // @step Given UnifiedBoardLayout component renders the TUI board
      // @step And there are 2 git stashes
      const mockStashes = [
        {
          oid: 'abc123',
          commit: {
            message: 'fspec-checkpoint:BOARD-001:baseline:1234567890',
            author: { timestamp: Date.now() / 1000 - 7200 }, // 2 hours ago
          },
        },
        {
          oid: 'def456',
          commit: {
            message: 'fspec-checkpoint:BOARD-002:experiment:1234567890',
            author: { timestamp: Date.now() / 1000 - 3600 }, // 1 hour ago
          },
        },
      ];

      vi.mocked(git.log).mockResolvedValue(mockStashes as any);

      // @step And there are 3 staged files and 1 unstaged file
      vi.mocked(getStagedFiles).mockResolvedValue([
        'src/auth.ts',
        'src/index.ts',
        'README.md',
      ]);

      vi.mocked(getUnstagedFiles).mockResolvedValue(['src/utils.ts']);

      // @step When the Git Context panel is rendered
      const { lastFrame } = render(<BoardView cwd={testDir} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then it should display "Git Stashes (2)" as the first section
      // @step And it should list the stash names below the header
      // @step And it should display "Changed Files (3 staged, 1 unstaged)" below the stashes
      // @step And it should list the changed files with staged/unstaged indicators
      // @step And both sections should be in the same panel box
      const frame = lastFrame();
      expect(frame).toContain('Git Stashes (2)');
      expect(frame).toContain('baseline');
      expect(frame).toContain('experiment');
      expect(frame).toContain('Changed Files (3 staged, 1 unstaged)');
      expect(frame).toContain('src/auth.ts');
      expect(frame).toContain('src/utils.ts');

      // Verify both sections are in same panel (no separator between them)
      const lines = frame.split('\n');
      let foundStashes = false;
      let foundChangedFiles = false;
      let foundSeparatorBetween = false;

      for (let i = 0; i < lines.length; i++) {
        if (lines[i].includes('Git Stashes')) {
          foundStashes = true;
        }
        if (foundStashes && !foundChangedFiles && lines[i].includes('├')) {
          foundSeparatorBetween = true;
        }
        if (lines[i].includes('Changed Files')) {
          foundChangedFiles = true;
        }
      }

      expect(foundStashes).toBe(true);
      expect(foundChangedFiles).toBe(true);
      expect(foundSeparatorBetween).toBe(false); // No separator = same panel
    });
  });

  describe('Scenario: Work Unit Details panel shows selected work unit metadata', () => {
    it('should display selected work unit with icon, title, truncated description, and metadata', async () => {
      // @step Given UnifiedBoardLayout component renders the TUI board
      // @step And work unit BOARD-001 is selected
      // @step And BOARD-001 is a story with title "Test Feature"
      // @step And BOARD-001 has description "This is a longer description that spans multiple lines and needs to be truncated for display"
      // @step And BOARD-001 has dependencies ["BOARD-002", "BOARD-003"]

      // Create work units data with selected work unit
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'BOARD-001': {
            id: 'BOARD-001',
            title: 'Test Feature',
            description:
              'This is a longer description that spans multiple lines and needs to be truncated for display.\nLine 2 of the description.\nLine 3 of the description.\nLine 4 of the description.\nLine 5 of the description.',
            status: 'implementing',
            type: 'story',
            epic: 'test-epic',
            estimate: 5,
            dependencies: ['BOARD-002', 'BOARD-003'],
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['BOARD-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // @step When the Work Unit Details panel is rendered
      const { lastFrame } = render(<BoardView cwd={testDir} />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then it should display "BOARD-001: Test Feature" as the title
      // @step And it should display the first 3 lines of the description
      // @step And it should display "Press ↵ to view full details" indicator
      // @step And it should display "Dependencies: BOARD-002, BOARD-003"
      // @step And it should display epic, estimate, and status metadata
      const frame = lastFrame();
      // BOARD-008: Story icon emoji removed
      expect(frame).toContain('BOARD-001');
      expect(frame).toContain('Test Feature');
      expect(frame).toMatch(/Press.*↵.*full details/i); // Indicator
      expect(frame).toContain('BOARD-002');
      expect(frame).toContain('BOARD-003');
      expect(frame).toContain('test-epic');
      expect(frame).toContain('5'); // Estimate
      expect(frame).toContain('implementing'); // Status
    });
  });

  describe('Scenario: Work Unit Details panel shows empty state when nothing selected', () => {
    it('should display user-friendly message when no work unit is selected', async () => {
      // @step Given UnifiedBoardLayout component renders the TUI board
      // @step And no work unit is selected (focused column is empty)

      // Create work units data with empty focused column
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      await writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // @step When the Work Unit Details panel is rendered
      const { lastFrame } = render(<BoardView cwd={testDir} />);

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then it should display "No work unit selected"
      // @step And the message should be user-friendly and centered
      // @step And no metadata fields should be displayed
      const frame = lastFrame();
      expect(frame).toMatch(/No work unit selected/i);

      // Verify no metadata fields are present
      expect(frame).not.toContain('Dependencies:');
      expect(frame).not.toContain('Epic:');
      expect(frame).not.toContain('Estimate:');
    });
  });
});
