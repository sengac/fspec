/**
 * Feature: spec/features/reverse-acdd-strategy-d-not-detecting-implementation-files.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { reverse } from '../reverse';
import { getSessionPath } from '../../utils/reverse-session';

describe('Feature: Reverse ACDD Strategy D Not Detecting Implementation Files', () => {
  let testDir: string;
  let sessionFile: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await fs.mkdir(testDir, { recursive: true });

    // Session file will be in OS tmpdir with project-specific hash
    sessionFile = await getSessionPath(testDir);
  });

  afterEach(async () => {
    // Cleanup test directory
    await fs.rm(testDir, { recursive: true, force: true });

    // Cleanup session file (may be in OS tmpdir)
    try {
      await fs.unlink(sessionFile);
    } catch {
      // Session file may not exist - that's fine
    }
  });

  describe('Scenario: Persona-driven discovery from UI component implementation', () => {
    it('should guide AI through outside-in BDD workflow using foundation.json personas', async () => {
      // Given: AI provides implementation context "MusicPlayer.tsx with play/pause/skip buttons"
      const srcDir = join(testDir, 'src', 'components');
      await fs.mkdir(srcDir, { recursive: true });
      await fs.writeFile(
        join(srcDir, 'MusicPlayer.tsx'),
        `
        export function MusicPlayer() {
          return (
            <div>
              <button>Play</button>
              <button>Pause</button>
              <button>Skip</button>
            </div>
          );
        }
        `
      );

      // And: foundation.json exists with "Music Listener" persona
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          project: {
            name: 'Music App',
            vision: 'Music streaming application',
          },
          personas: [
            {
              name: 'Music Listener',
              description: 'Person who listens to music',
              goals: ['Control playback', 'Discover new music'],
            },
          ],
        })
      );

      // When: Strategy D prompts "Who uses this? Check foundation.json personas"
      const result = await reverse({
        cwd: testDir,
        strategy: 'D',
        implementationContext: 'MusicPlayer.tsx with play/pause/skip buttons',
      });

      // Then: AI identifies "Music Listener" persona from foundation.json
      expect(result.systemReminder).toContain('Music Listener');
      expect(result.systemReminder).toContain('WHO uses this?');
      expect(result.systemReminder).toContain('Check foundation.json personas');

      // And: Strategy D prompts "What does Music Listener want to accomplish?"
      expect(result.systemReminder).toContain(
        'What does Music Listener want to accomplish?'
      );

      // And: AI creates example map with examples "Music Listener plays song" and "Music Listener pauses playback"
      // (This would be done via fspec add-example commands in actual workflow)
      expect(result.guidance).toContain('example map');
      expect(result.guidance).toContain('fspec add-example');

      // And: AI runs "fspec generate-scenarios" to create feature file
      expect(result.guidance).toContain('fspec generate-scenarios');

      // And: Strategy D generates test skeletons
      expect(result.guidance).toContain('test skeleton');

      // And: Strategy D auto-links coverage to MusicPlayer.tsx implementation
      expect(result.guidance).toContain('fspec link-coverage');
      expect(result.guidance).toContain('--skip-validation');
    });
  });

  describe('Scenario: Behavior identification from state management implementation', () => {
    it('should guide AI to identify user behavior from implementation details', async () => {
      // Given: AI provides implementation context "usePlaylistStore.ts with addSong(), removeSong(), clearPlaylist()"
      const srcDir = join(testDir, 'src', 'stores');
      await fs.mkdir(srcDir, { recursive: true });
      await fs.writeFile(
        join(srcDir, 'usePlaylistStore.ts'),
        `
        export const usePlaylistStore = () => {
          const addSong = (song) => {};
          const removeSong = (id) => {};
          const clearPlaylist = () => {};
        };
        `
      );

      // And: foundation.json exists with personas
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          project: { name: 'Music App' },
          personas: [
            {
              name: 'Playlist Curator',
              description: 'Person managing playlists',
              goals: ['Organize music'],
            },
          ],
        })
      );

      // When: Strategy D prompts "What user behavior does this support?"
      const result = await reverse({
        cwd: testDir,
        strategy: 'D',
        implementationContext:
          'usePlaylistStore.ts with addSong(), removeSong(), clearPlaylist()',
      });

      // Then: AI identifies user behavior "Managing playlists"
      expect(result.systemReminder).toContain('What user behavior');
      expect(result.systemReminder).toContain('not which system calls it');
      expect(result.systemReminder).toContain('who BENEFITS');

      // And: AI creates example map with rules "User can add songs" and "User can remove songs"
      expect(result.guidance).toContain('fspec add-rule');
      expect(result.guidance).toContain('fspec add-example');

      // And: AI runs "fspec generate-scenarios" to create playlist-management.feature
      expect(result.guidance).toContain('fspec generate-scenarios');

      // And: Strategy D generates test skeleton
      expect(result.guidance).toContain('test skeleton');

      // And: Strategy D links coverage to usePlaylistStore.ts implementation
      expect(result.guidance).toContain('fspec link-coverage');
    });
  });

  describe('Scenario: Foundation-driven scenario generation from audio implementation', () => {
    it('should use persona goals to guide scenario creation', async () => {
      // Given: foundation.json exists with "Mobile App User" persona with goal "Stream music on the go"
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        join(testDir, 'spec', 'foundation.json'),
        JSON.stringify({
          project: { name: 'Music App' },
          personas: [
            {
              name: 'Mobile App User',
              description: 'Person using mobile app',
              goals: ['Stream music on the go', 'Control playback offline'],
            },
          ],
        })
      );

      // And: AI provides implementation context "AudioPlayer.tsx"
      const srcDir = join(testDir, 'src', 'components');
      await fs.mkdir(srcDir, { recursive: true });
      await fs.writeFile(
        join(srcDir, 'AudioPlayer.tsx'),
        `
        export function AudioPlayer() {
          return <audio controls />;
        }
        `
      );

      // When: Strategy D prompts "How does Mobile App User stream music?"
      const result = await reverse({
        cwd: testDir,
        strategy: 'D',
        implementationContext: 'AudioPlayer.tsx',
      });

      // Then: AI creates example map with examples "User taps play", "User adjusts volume", "User sees album art"
      expect(result.systemReminder).toContain('Mobile App User');
      expect(result.systemReminder).toContain('Stream music on the go');
      expect(result.guidance).toContain('example map');
      expect(result.guidance).toContain('fspec add-example');

      // And: AI runs "fspec generate-scenarios" to create feature file
      expect(result.guidance).toContain('fspec generate-scenarios');

      // And: generated feature file contains user-centric scenarios based on Mobile App User persona
      expect(result.systemReminder).toContain('user-centric');
      expect(result.systemReminder).toContain('persona');
    });
  });
});
