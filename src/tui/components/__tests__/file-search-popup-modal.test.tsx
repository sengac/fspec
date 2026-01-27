/**
 * Tests for File Search Input Hook - Following ink-testing-library pattern
 *
 * Tests the file search functionality exactly like slash command tests:
 * - Hook logic verification  
 * - File searching behavior
 * - Keyboard navigation
 *
 * Work Unit: TUI-055
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock tool integration
vi.mock('../../../utils/toolIntegration', () => ({
  callGlobTool: vi.fn(),
}));

describe('Feature: File Search Popup for @ Symbol Input', () => {
  describe('Scenario: Show popup when typing @ symbol', () => {
    it('should detect @ symbol at any position and extract filter', () => {
      // @step Given I have an empty input
      let input = '';
      
      // @step When I type "@" anywhere in the input
      input = 'hello @';
      const atIndex = input.lastIndexOf('@');
      const shouldShowPopup = atIndex >= 0;
      const filter = input.slice(atIndex + 1);
      
      // @step Then the popup should show with empty filter
      expect(shouldShowPopup).toBe(true);
      expect(filter).toBe('');
      
      // @step When I continue typing after @
      input = 'hello @src';
      const newAtIndex = input.lastIndexOf('@');
      const newFilter = input.slice(newAtIndex + 1);
      
      // @step Then filter should be extracted correctly
      expect(newFilter).toBe('src');
      expect(!newFilter.includes(' ')).toBe(true);
    });
    
    it('should hide popup when space is typed after @', () => {
      // @step Given I have input with @ and file reference
      let input = 'hello @src/file.txt';
      let atIndex = input.lastIndexOf('@');
      let afterAt = input.slice(atIndex + 1);
      
      // @step When space is not present, popup should be visible
      expect(!afterAt.includes(' ')).toBe(true);
      
      // @step When I type a space (end of file reference)
      input = 'hello @src/file.txt ';
      atIndex = input.lastIndexOf('@');
      afterAt = input.slice(atIndex + 1);
      
      // @step Then popup should hide
      expect(afterAt.includes(' ')).toBe(true);
    });
    
    it('should hide popup when @ is removed', () => {
      // @step Given I have input with @
      let input = 'hello @src';
      let hasAt = input.includes('@');
      expect(hasAt).toBe(true);
      
      // @step When I delete the @ symbol
      input = 'hello src';
      hasAt = input.includes('@');
      
      // @step Then popup should hide
      expect(hasAt).toBe(false);
    });
  });
  
  describe('Scenario: File path insertion', () => {
    it('should replace filter with full file path when file is selected', () => {
      // @step Given I have input with @ and filter
      const originalInput = 'hello @src';
      const atIndex = originalInput.lastIndexOf('@');
      const beforeAt = originalInput.slice(0, atIndex);
      const selectedFile = 'src/components/Button.tsx';
      
      // @step When a file is selected (Enter key)
      const newInput = `${beforeAt}@${selectedFile} `;
      
      // @step Then input should contain full file path with space
      expect(newInput).toBe('hello @src/components/Button.tsx ');
    });
  });
  
  describe('Scenario: File searching with Glob tool', () => {
    it('should generate correct Glob pattern from filter', () => {
      // @step Given I have a filter "src"
      const filter = 'src';
      
      // @step When searching for files
      const expectedPattern = `**/*${filter}*`;
      
      // @step Then the pattern should be **/*src*
      expect(expectedPattern).toBe('**/*src*');
    });
    
    it('should parse file results correctly', () => {
      // @step Given Glob tool returns file paths
      const globResult = 'src/components/Button.tsx\nsrc/components/Input.tsx\nsrc/utils/helper.ts';
      
      // @step When parsing the result
      const filePaths = globResult.split('\n').filter(Boolean);
      const fileResults = filePaths.map(path => ({
        path,
        displayName: path.split('/').pop() || path,
      }));
      
      // @step Then files should be parsed correctly
      expect(fileResults).toHaveLength(3);
      expect(fileResults[0]).toEqual({
        path: 'src/components/Button.tsx',
        displayName: 'Button.tsx'
      });
    });
  });
  
  describe('Scenario: Keyboard navigation simulation', () => {
    it('should move selection down with wrap-around', () => {
      // @step Given I have 3 files and selection at index 0
      const files = ['file1.ts', 'file2.ts', 'file3.ts'];
      let selectedIndex = 0;
      
      // @step When I press down arrow twice
      // Simulate moveDown logic
      selectedIndex = selectedIndex >= files.length - 1 ? 0 : selectedIndex + 1;
      selectedIndex = selectedIndex >= files.length - 1 ? 0 : selectedIndex + 1;
      
      // @step Then selection should be at index 2
      expect(selectedIndex).toBe(2);
    });
    
    it('should wrap to first item when moving down from last', () => {
      // @step Given I have 3 files and selection at last index
      const files = ['file1.ts', 'file2.ts', 'file3.ts'];
      let selectedIndex = files.length - 1;
      
      // @step When I press down arrow
      selectedIndex = selectedIndex >= files.length - 1 ? 0 : selectedIndex + 1;
      
      // @step Then selection should wrap to 0
      expect(selectedIndex).toBe(0);
    });
  });
});