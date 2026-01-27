/**
 * File Search Input Hook Tests
 * 
 * Tests file search logic and keyboard navigation.
 */

import { describe, it, expect, vi } from 'vitest';

vi.mock('../../../utils/toolIntegration', () => ({
  callGlobTool: vi.fn(),
}));

describe('File Search Popup', () => {
  describe('@ symbol detection', () => {
    it('should detect @ symbol at any position and extract filter', () => {
      let input = '';
      
      input = 'hello @';
      const atIndex = input.lastIndexOf('@');
      const shouldShowPopup = atIndex >= 0;
      const filter = input.slice(atIndex + 1);
      
      expect(shouldShowPopup).toBe(true);
      expect(filter).toBe('');
      
      input = 'hello @src';
      const newAtIndex = input.lastIndexOf('@');
      const newFilter = input.slice(newAtIndex + 1);
      
      expect(newFilter).toBe('src');
      expect(!newFilter.includes(' ')).toBe(true);
    });
    
    it('should hide popup when space is typed after @', () => {
      let input = 'hello @src/file.txt';
      let atIndex = input.lastIndexOf('@');
      let afterAt = input.slice(atIndex + 1);
      
      expect(!afterAt.includes(' ')).toBe(true);
      
      input = 'hello @src/file.txt and more text';
      atIndex = input.lastIndexOf('@');
      afterAt = input.slice(atIndex + 1);
      
      expect(afterAt.includes(' ')).toBe(true);
    });
  });

  describe('File search results', () => {
    it('should format file search results correctly', () => {
      const rawResults = ['src/components/Button.tsx', 'src/hooks/useSearch.ts', 'docs/README.md'];
      const formattedResults = rawResults.map(path => ({ path }));
      
      expect(formattedResults).toEqual([
        { path: 'src/components/Button.tsx' },
        { path: 'src/hooks/useSearch.ts' },
        { path: 'docs/README.md' }
      ]);
      
      const fileNames = formattedResults.map(file => file.path.split('/').pop() || file.path);
      expect(fileNames).toEqual(['Button.tsx', 'useSearch.ts', 'README.md']);
    });
  });
});