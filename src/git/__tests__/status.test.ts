/**
 * Feature: spec/features/replace-git-cli-usage-with-isomorphic-git-library.feature
 *
 * DEPRECATED: This test file validated the isomorphic-git based implementation.
 *
 * As of GIT-013, the git status operations have been migrated to use gitoxide (gix)
 * via NAPI-RS bindings. These tests used memfs (in-memory filesystem) which is not
 * supported by gitoxide.
 *
 * The new implementation is tested by:
 * - src/git/__tests__/gitoxide-operations.test.ts (uses real temp directories)
 *
 * These tests are skipped because:
 * 1. memfs is not compatible with gitoxide (native Rust library)
 * 2. The new gitoxide tests provide equivalent coverage
 * 3. The fs option in GitStatusOptions is deprecated
 *
 * TODO: Either remove this file or convert tests to use real temp directories
 * if additional coverage of getStagedFilesWithChangeType/getUnstagedFilesWithChangeType
 * is needed beyond what gitoxide-operations.test.ts provides.
 */

import { describe, it, expect } from 'vitest';

describe.skip('Feature: Replace git CLI usage with isomorphic-git library (DEPRECATED)', () => {
  it('tests skipped - see file header comment', () => {
    expect(true).toBe(true);
  });
});

describe.skip('Feature: Changed files view missing unstaged files (DEPRECATED)', () => {
  it('tests skipped - see file header comment', () => {
    expect(true).toBe(true);
  });
});
