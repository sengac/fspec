/**
 * Feature: spec/features/merge-worktree-ux.feature
 *
 * Unit tests for the pure formatting functions in mergeSummaryFormatting.ts.
 * These test buildMergeSummary, parseConflictFiles, and buildConflictSummary
 * in isolation — no NAPI, no React, no side effects.
 *
 * Work Unit: GIT-037
 */

import { describe, it, expect } from 'vitest';
import {
  buildMergeSummary,
  parseConflictFiles,
  buildConflictSummary,
} from '../mergeSummaryFormatting';

describe('Feature: Merge summary formatting (GIT-037)', () => {
  // ========================================================================
  // buildMergeSummary
  // ========================================================================
  describe('buildMergeSummary', () => {
    it('should list all file paths grouped by Modified, Added, Deleted', () => {
      // @step Given a merge result with 3 modified, 1 added, and 0 deleted files
      const mergeResult = {
        filesModified: [
          'src/auth/login.ts',
          'src/auth/register.ts',
          'src/utils/helpers.ts',
        ],
        filesAdded: ['src/auth/types.ts'],
        filesDeleted: [],
      };

      // @step When buildMergeSummary is called
      const summary = buildMergeSummary(mergeResult);

      // @step Then the summary should contain "Merge successful"
      expect(summary).toContain('✓ Merge successful');

      // @step And Modified files should be listed with count
      expect(summary).toContain('Modified (3):');
      expect(summary).toContain('    src/auth/login.ts');
      expect(summary).toContain('    src/auth/register.ts');
      expect(summary).toContain('    src/utils/helpers.ts');

      // @step And Added files should be listed with count
      expect(summary).toContain('Added (1):');
      expect(summary).toContain('    src/auth/types.ts');

      // @step And Deleted should show 0
      expect(summary).toContain('Deleted (0)');
    });

    it('should handle empty file lists', () => {
      const summary = buildMergeSummary({
        filesModified: [],
        filesAdded: [],
        filesDeleted: [],
      });

      expect(summary).toContain('✓ Merge successful');
      expect(summary).toContain('Modified (0):');
      expect(summary).toContain('Added (0):');
      expect(summary).toContain('Deleted (0)');
    });

    it('should list deleted files when present', () => {
      const summary = buildMergeSummary({
        filesModified: [],
        filesAdded: [],
        filesDeleted: ['old-file.ts', 'deprecated.ts'],
      });

      expect(summary).toContain('Deleted (2)');
      expect(summary).toContain('    old-file.ts');
      expect(summary).toContain('    deprecated.ts');
    });
  });

  // ========================================================================
  // parseConflictFiles
  // ========================================================================
  describe('parseConflictFiles', () => {
    it('should parse file paths from Rust Debug format', () => {
      // @step Given a Rust error with Debug-formatted Vec<String>
      const errorMessage =
        'Conflict detected: ["src/auth/login.ts", "src/utils/helpers.ts"] have been modified in both session and main worktree';

      // @step When parseConflictFiles is called
      const files = parseConflictFiles(errorMessage);

      // @step Then it should extract the file paths
      expect(files).toEqual(['src/auth/login.ts', 'src/utils/helpers.ts']);
    });

    it('should parse single file conflict', () => {
      const files = parseConflictFiles(
        'Conflict detected: ["src/main.ts"] have been modified'
      );
      expect(files).toEqual(['src/main.ts']);
    });

    it('should return null when no bracket content found', () => {
      const files = parseConflictFiles('Some unrelated error message');
      expect(files).toBeNull();
    });

    it('should return null for empty bracket content', () => {
      const files = parseConflictFiles('Error: []');
      expect(files).toBeNull();
    });
  });

  // ========================================================================
  // buildConflictSummary
  // ========================================================================
  describe('buildConflictSummary', () => {
    it('should build rich conflict summary with parsed file paths', () => {
      // @step Given a Rust conflict error with file paths
      const errorMessage =
        'Conflict detected: ["src/auth/login.ts", "src/utils/helpers.ts"] have been modified in both session and main worktree';

      // @step When buildConflictSummary is called
      const summary = buildConflictSummary(errorMessage);

      // @step Then it should contain the conflict header
      expect(summary).toContain('⚠ Merge conflicts detected');

      // @step And list the conflicting files
      expect(summary).toContain('  Conflicting files:');
      expect(summary).toContain('    src/auth/login.ts');
      expect(summary).toContain('    src/utils/helpers.ts');

      // @step And include guidance text
      expect(summary).toContain(
        'These files were modified in both this session and the main worktree.'
      );
      expect(summary).toContain(
        'Resolve the conflicts, then run /merge-worktree again.'
      );
    });

    it('should fall back to raw error when parsing fails', () => {
      // @step Given an error without bracket-formatted file paths
      const errorMessage = 'Some unexpected conflict error format';

      // @step When buildConflictSummary is called
      const summary = buildConflictSummary(errorMessage);

      // @step Then it should show the raw error with guidance
      expect(summary).toContain('⚠ Merge conflicts detected');
      expect(summary).toContain(errorMessage);
      expect(summary).toContain(
        'Resolve the conflicts, then run /merge-worktree again.'
      );
    });
  });
});
