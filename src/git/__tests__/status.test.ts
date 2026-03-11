/**
 * Feature: spec/features/replace-git-cli-usage-with-isomorphic-git-library.feature
 *
 * DEPRECATED: This test file validated the old isomorphic-git based implementation.
 *
 * As of GIT-013, git status operations use gitoxide (gix) via NAPI-RS bindings.
 * As of GIT-039, isomorphic-git was fully removed from the project.
 *
 * The current implementation is tested by:
 * - src/git/__tests__/gitoxide-operations.test.ts (uses real temp directories)
 *
 * TODO: Remove this file entirely.
 */

import { describe, it, expect } from 'vitest';

describe.skip('Feature: Replace git CLI usage (DEPRECATED - see gitoxide-operations.test.ts)', () => {
  it('tests skipped - see file header comment', () => {
    expect(true).toBe(true);
  });
});
