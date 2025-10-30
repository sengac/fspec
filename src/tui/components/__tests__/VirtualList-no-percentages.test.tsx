/**
 * Feature: spec/features/virtuallist-flexbox-text-wrapping.feature
 *
 * Tests to ensure NO percentage-based values (flexBasis, width, height) are used
 * in components using VirtualList. ALL components must use flexGrow/flexShrink.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

const componentsUsingVirtualList = [
  'src/tui/components/CheckpointViewer.tsx',
  'src/tui/components/FileDiffViewer.tsx',
  'src/tui/components/BoardView.tsx',
];

describe('Feature: VirtualList flexbox text wrapping', () => {
  describe('Scenario: CheckpointViewer uses flexGrow with NO percentages', () => {
    it('should NOT use flexBasis with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/CheckpointViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Check for flexBasis="XX%" or flexBasis='XX%'
      const percentageFlexBasisPattern = /flexBasis=["'][0-9]+%["']/g;
      const matches = content.match(percentageFlexBasisPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use width with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/CheckpointViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Check for width="XX%" or width='XX%'
      const percentageWidthPattern = /width=["'][0-9]+%["']/g;
      const matches = content.match(percentageWidthPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use height with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/CheckpointViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Check for height="XX%" or height='XX%'
      const percentageHeightPattern = /height=["'][0-9]+%["']/g;
      const matches = content.match(percentageHeightPattern);

      expect(matches).toBeNull();
    });

    it('should use flexGrow properties', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/CheckpointViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Should have flexGrow for proportional sizing
      expect(content).toMatch(/flexGrow=\{/);
    });
  });

  describe('Scenario: FileDiffViewer uses flexGrow with NO percentages', () => {
    it('should NOT use flexBasis with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/FileDiffViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageFlexBasisPattern = /flexBasis=["'][0-9]+%["']/g;
      const matches = content.match(percentageFlexBasisPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use width with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/FileDiffViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageWidthPattern = /width=["'][0-9]+%["']/g;
      const matches = content.match(percentageWidthPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use height with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/FileDiffViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageHeightPattern = /height=["'][0-9]+%["']/g;
      const matches = content.match(percentageHeightPattern);

      expect(matches).toBeNull();
    });

    it('should use flexGrow properties', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/FileDiffViewer.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Should have flexGrow for proportional sizing
      expect(content).toMatch(/flexGrow=\{/);
    });
  });

  describe('Scenario: BoardView uses flexGrow with NO percentages', () => {
    it('should NOT use flexBasis with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/BoardView.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageFlexBasisPattern = /flexBasis=["'][0-9]+%["']/g;
      const matches = content.match(percentageFlexBasisPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use width with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/BoardView.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageWidthPattern = /width=["'][0-9]+%["']/g;
      const matches = content.match(percentageWidthPattern);

      expect(matches).toBeNull();
    });

    it('should NOT use height with percentage values', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/BoardView.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      const percentageHeightPattern = /height=["'][0-9]+%["']/g;
      const matches = content.match(percentageHeightPattern);

      expect(matches).toBeNull();
    });

    it('should use flexGrow properties', () => {
      const filePath = path.join(process.cwd(), 'src/tui/components/BoardView.tsx');
      const content = fs.readFileSync(filePath, 'utf-8');

      // Should have flexGrow for proportional sizing
      expect(content).toMatch(/flexGrow=\{/);
    });
  });

});
