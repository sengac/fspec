/**
 * Feature: spec/features/merge-conflict-llm-context.feature
 *
 * Unit tests for the pure formatting function buildConflictLlmContext
 * in conflictLlmContext.ts. These validate the content structure without
 * any NAPI or React dependencies.
 *
 * Work Unit: GIT-038
 */

import { describe, it, expect } from 'vitest';
import { buildConflictLlmContext } from '../conflictLlmContext';

describe('Feature: Conflict LLM context formatting (GIT-038)', () => {
  // ========================================================================
  // buildConflictLlmContext
  // ========================================================================
  describe('buildConflictLlmContext', () => {
    it('should include the conflicting file name from a single-file error', () => {
      // @step Given a Rust conflict error with one file
      const errorMessage =
        'Conflict detected: ["README.md"] have been modified in both session and main worktree';

      // @step When buildConflictLlmContext is called
      const context = buildConflictLlmContext(errorMessage, '/tmp/worktree');

      // @step Then it should list the file
      expect(context).toContain('README.md');
    });

    it('should include all conflicting file names from a multi-file error', () => {
      // @step Given a Rust conflict error with multiple files
      const errorMessage =
        'Conflict detected: ["src/auth/login.ts", "src/utils/helpers.ts"] have been modified in both session and main worktree';

      // @step When buildConflictLlmContext is called
      const context = buildConflictLlmContext(errorMessage, '/tmp/worktree');

      // @step Then it should list all files
      expect(context).toContain('src/auth/login.ts');
      expect(context).toContain('src/utils/helpers.ts');
    });

    it('should mention git conflict markers', () => {
      const context = buildConflictLlmContext(
        'Conflict detected: ["file.ts"] have been modified',
        '/tmp/wt'
      );

      expect(context).toMatch(/<<<<<<</);
      expect(context).toMatch(/=======/);
      expect(context).toMatch(/>>>>>>>/);
    });

    it('should instruct to run /merge-worktree again', () => {
      const context = buildConflictLlmContext(
        'Conflict detected: ["file.ts"] have been modified',
        '/tmp/wt'
      );

      expect(context).toContain('/merge-worktree');
      expect(context).toContain('After resolving all conflicts');
    });

    it('should include worktree path when provided', () => {
      // @step Given a worktree path
      const worktreePath = '/tmp/fspec-session-abc123';

      // @step When buildConflictLlmContext is called with the path
      const context = buildConflictLlmContext(
        'Conflict detected: ["file.ts"] have been modified',
        worktreePath
      );

      // @step Then it should include the path in the context
      expect(context).toContain(worktreePath);
      expect(context).toContain('worktree at:');
    });

    it('should omit worktree location line when worktreePath is null', () => {
      // @step Given null worktree path
      const context = buildConflictLlmContext(
        'Conflict detected: ["file.ts"] have been modified',
        null
      );

      // @step Then it should not contain worktree location line
      expect(context).not.toContain('worktree at:');
      // But should still contain the rest of the message
      expect(context).toContain('file.ts');
      expect(context).toContain('/merge-worktree');
    });

    it('should not contain system-reminder tags', () => {
      const context = buildConflictLlmContext(
        'Conflict detected: ["file.ts"] have been modified',
        '/tmp/wt'
      );

      expect(context).not.toContain('<system-reminder>');
      expect(context).not.toContain('</system-reminder>');
    });

    it('should handle unparseable error messages gracefully', () => {
      // @step Given an error that cannot be parsed for file paths
      const context = buildConflictLlmContext(
        'Some unexpected conflict error',
        '/tmp/wt'
      );

      // @step Then it should still produce a valid context message
      expect(context).toContain('Merge conflicts were detected');
      expect(context).toContain('/merge-worktree');
      expect(context).toContain('Could not parse file list');
    });
  });
});
