/**
 * Feature: spec/features/virtuallist-flexbox-text-wrapping.feature
 *
 * Tests to ensure VirtualList does NOT use percentage-based values 
 * (flexBasis, width, height) and uses flexGrow/flexShrink instead.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

describe('Feature: VirtualList flexbox text wrapping', () => {
  describe('Scenario: VirtualList uses flexGrow with NO percentages', () => {
    const filePath = path.join(process.cwd(), 'src/tui/components/VirtualList.tsx');
    const content = fs.readFileSync(filePath, 'utf-8');

    it('should NOT use flexBasis with percentage values', () => {
      const percentageFlexBasisPattern = /flexBasis=["'][0-9]+%["']/g;
      const matches = content.match(percentageFlexBasisPattern);
      expect(matches).toBeNull();
    });

    it('should NOT use width with percentage values', () => {
      const percentageWidthPattern = /width=["'][0-9]+%["']/g;
      const matches = content.match(percentageWidthPattern);
      expect(matches).toBeNull();
    });

    it('should NOT use height with percentage values', () => {
      const percentageHeightPattern = /height=["'][0-9]+%["']/g;
      const matches = content.match(percentageHeightPattern);
      expect(matches).toBeNull();
    });

    it('should use flexGrow properties', () => {
      expect(content).toMatch(/flexGrow=\{/);
    });
  });
});
