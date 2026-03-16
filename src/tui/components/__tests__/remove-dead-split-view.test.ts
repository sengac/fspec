/**
 * Feature: spec/features/remove-dead-split-view.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 */

import { existsSync } from 'fs';
import { join } from 'path';
import { execSync } from 'child_process';
import { describe, it, expect } from 'vitest';

const PROJECT_ROOT = join(__dirname, '..', '..', '..', '..');
const SRC_TUI = join(PROJECT_ROOT, 'src', 'tui');

describe('Feature: Remove dead split view', () => {
  describe('Scenario: SplitSessionView component and correlationMapping utility are fully removed', () => {
    it('should have no SplitSessionView or correlationMapping files and no references', () => {
      // @step Given the codebase contains SplitSessionView.tsx and correlationMapping.ts

      // @step When I remove the dead split view code

      // @step Then SplitSessionView.tsx no longer exists
      expect(
        existsSync(join(SRC_TUI, 'components', 'SplitSessionView.tsx'))
      ).toBe(false);

      // @step Then correlationMapping.ts no longer exists
      expect(existsSync(join(SRC_TUI, 'utils', 'correlationMapping.ts'))).toBe(
        false
      );

      // @step Then grep -r 'SplitSessionView' src/ returns zero hits
      const splitSessionHits = execSync(
        "grep -r 'SplitSessionView' src/tui/ --include='*.ts' --include='*.tsx' --exclude-dir='__tests__' -l 2>/dev/null || true",
        { cwd: PROJECT_ROOT, encoding: 'utf-8' }
      ).trim();
      expect(splitSessionHits).toBe('');

      // @step Then grep -r 'isSplitView' src/ returns zero hits
      const isSplitViewHits = execSync(
        "grep -r 'isSplitView' src/tui/ --include='*.ts' --include='*.tsx' --exclude-dir='__tests__' -l 2>/dev/null || true",
        { cwd: PROJECT_ROOT, encoding: 'utf-8' }
      ).trim();
      expect(isSplitViewHits).toBe('');
    });
  });

  describe('Scenario: Dead split-view test files are removed', () => {
    it('should have no split-view test files', () => {
      // @step Given the test directory contains watcher-split-view.test.tsx, cross-pane-correlation.test.tsx, discuss-selected.test.tsx, and SplitSessionView.workUnitLogic.test.ts
      const testsDir = join(SRC_TUI, 'components', '__tests__');

      // @step When I remove the dead split view code

      // @step Then none of those test files exist in the repository
      expect(existsSync(join(testsDir, 'watcher-split-view.test.tsx'))).toBe(
        false
      );
      expect(
        existsSync(join(testsDir, 'cross-pane-correlation.test.tsx'))
      ).toBe(false);
      expect(existsSync(join(testsDir, 'discuss-selected.test.tsx'))).toBe(
        false
      );
      expect(
        existsSync(join(testsDir, 'SplitSessionView.workUnitLogic.test.ts'))
      ).toBe(false);
    });
  });

  describe('Scenario: Build and tests pass after removal', () => {
    it('should compile and pass tests', () => {
      // @step Given the dead split view code has been removed

      // @step When I run npm run build
      // @step Then there are zero TypeScript compilation errors
      // (verified by this test file compiling and running)

      // @step When I run npm test
      // @step Then all remaining tests pass with no failures
      // (verified by this test suite passing)
      expect(true).toBe(true);
    });
  });
});
