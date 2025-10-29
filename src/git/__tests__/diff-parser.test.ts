/**
 * Feature: spec/features/enhanced-line-by-line-diff-with-background-colors.feature
 *
 * Tests for enhanced line-by-line diff parser with line pairing logic
 * Coverage: GIT-008 Enhanced line-by-line diff with background colors
 */

import { describe, it, expect } from 'vitest';
import { parseDiff, DiffLine } from '../diff-parser';

describe('Feature: Enhanced line-by-line diff with background colors', () => {
  describe('Scenario: Simple line replacement shows old line in red, new line in green', () => {
    it('should parse simple replacement with correct line types', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,3 +1,3 @@
 const x = 1;
-const y = 2;
+const y = 3;
 const z = 4;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then I should see correct line types
      expect(lines).toHaveLength(5);

      expect(lines[0].type).toBe('hunk');
      expect(lines[0].content).toBe('@@ -1,3 +1,3 @@');

      expect(lines[1].type).toBe('context');
      expect(lines[1].content).toBe(' const x = 1;');

      expect(lines[2].type).toBe('removed');
      expect(lines[2].content).toBe('-const y = 2;');
      expect(lines[2].changeGroup).toBe('replacement');

      expect(lines[3].type).toBe('added');
      expect(lines[3].content).toBe('+const y = 3;');
      expect(lines[3].changeGroup).toBe('replacement');

      expect(lines[4].type).toBe('context');
      expect(lines[4].content).toBe(' const z = 4;');
    });
  });

  describe('Scenario: Multi-line replacement shows paired lines with color backgrounds', () => {
    it('should pair multiple consecutive removed and added lines as replacement', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,5 +1,5 @@
 function calculate() {
-  const total = a + b;
-  return total;
+  const sum = a + b;
+  return sum * 2;
 }`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then removed and added lines should be paired as replacement
      expect(lines[2].type).toBe('removed');
      expect(lines[2].changeGroup).toBe('replacement');

      expect(lines[3].type).toBe('removed');
      expect(lines[3].changeGroup).toBe('replacement');

      expect(lines[4].type).toBe('added');
      expect(lines[4].changeGroup).toBe('replacement');

      expect(lines[5].type).toBe('added');
      expect(lines[5].changeGroup).toBe('replacement');
    });
  });

  describe('Scenario: Pure addition shows only green background', () => {
    it('should mark unpaired added line as pure addition', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,2 +1,3 @@
 const x = 1;
+const y = 2;
 const z = 3;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then the added line should be marked as pure addition
      const addedLine = lines.find(l => l.type === 'added');
      expect(addedLine).toBeDefined();
      expect(addedLine!.changeGroup).toBe('addition');
    });
  });

  describe('Scenario: Pure deletion shows only red background', () => {
    it('should mark unpaired removed line as pure deletion', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,3 +1,2 @@
 const x = 1;
-const y = 2;
 const z = 3;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then the removed line should be marked as pure deletion
      const removedLine = lines.find(l => l.type === 'removed');
      expect(removedLine).toBeDefined();
      expect(removedLine!.changeGroup).toBe('deletion');
    });
  });

  describe('Scenario: Mixed changes show appropriate colors for each type', () => {
    it('should correctly identify replacement, addition, and deletion in same diff', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,6 +1,6 @@
 const a = 1;
-const b = 2;
+const b = 3;
 const c = 4;
+const d = 5;
-const e = 6;
 const f = 7;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then I should see correct change groups
      // First removed/added pair is replacement
      expect(lines[2].type).toBe('removed');
      expect(lines[2].changeGroup).toBe('replacement');
      expect(lines[3].type).toBe('added');
      expect(lines[3].changeGroup).toBe('replacement');

      // Standalone added is pure addition
      expect(lines[5].type).toBe('added');
      expect(lines[5].changeGroup).toBe('addition');

      // Standalone removed is pure deletion
      expect(lines[6].type).toBe('removed');
      expect(lines[6].changeGroup).toBe('deletion');
    });
  });

  describe('Scenario: Unbalanced replacements handle many-to-one changes', () => {
    it('should group many removed lines with one added line as replacement', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,4 +1,2 @@
-const a = 1;
-const b = 2;
-const c = 3;
+const total = 6;
 const d = 4;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then all removed and added lines should be grouped as replacement
      expect(lines[1].type).toBe('removed');
      expect(lines[1].changeGroup).toBe('replacement');

      expect(lines[2].type).toBe('removed');
      expect(lines[2].changeGroup).toBe('replacement');

      expect(lines[3].type).toBe('removed');
      expect(lines[3].changeGroup).toBe('replacement');

      expect(lines[4].type).toBe('added');
      expect(lines[4].changeGroup).toBe('replacement');
    });
  });

  describe('Scenario: Unbalanced replacements handle one-to-many changes', () => {
    it('should group one removed line with many added lines as replacement', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,2 +1,4 @@
-const total = 6;
+const a = 1;
+const b = 2;
+const c = 3;
 const d = 4;`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then all removed and added lines should be grouped as replacement
      expect(lines[1].type).toBe('removed');
      expect(lines[1].changeGroup).toBe('replacement');

      expect(lines[2].type).toBe('added');
      expect(lines[2].changeGroup).toBe('replacement');

      expect(lines[3].type).toBe('added');
      expect(lines[3].changeGroup).toBe('replacement');

      expect(lines[4].type).toBe('added');
      expect(lines[4].changeGroup).toBe('replacement');
    });
  });

  describe('Scenario: Empty lines in diff maintain appropriate background', () => {
    it('should handle empty lines with correct type and change group', () => {
      // Given a file with the following unified diff
      const diffContent = `@@ -1,3 +1,3 @@
-
+  // Added comment
 function test() {}`;

      // When I parse the diff
      const lines: DiffLine[] = parseDiff(diffContent);

      // Then empty removed and added lines should be paired as replacement
      expect(lines[1].type).toBe('removed');
      expect(lines[1].content).toBe('-');
      expect(lines[1].changeGroup).toBe('replacement');

      expect(lines[2].type).toBe('added');
      expect(lines[2].content).toBe('+  // Added comment');
      expect(lines[2].changeGroup).toBe('replacement');
    });
  });
});
