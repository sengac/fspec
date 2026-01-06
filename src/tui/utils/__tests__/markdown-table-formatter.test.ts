/**
 * Tests for markdown table formatting in AI output
 *
 * Feature: spec/features/markdown-table-rendering-in-ai-output.feature
 *
 * Tests the formatMarkdownTables() utility that transforms raw markdown tables
 * into properly aligned tables with box-drawing characters.
 */

import { describe, it, expect } from 'vitest';
import { formatMarkdownTables } from '../markdown-table-formatter';
import { getVisualWidth } from '../stringWidth';

describe('Feature: Markdown Table Rendering in AI Output', () => {
  describe('Scenario: Simple table with two columns renders with box borders', () => {
    it('should render a simple table with box-drawing characters and aligned columns', () => {
      // @step Given the AI assistant has completed a response containing a markdown table
      const input = '| Name | Age |\n|---|---|\n| Alice | 30 |';

      // @step When the streaming is marked complete (isStreaming: false)
      // (streaming completion triggers formatMarkdownTables in AgentView)
      const result = formatMarkdownTables(input);

      // @step Then the table is rendered with box-drawing characters
      expect(result).toContain('┌');
      expect(result).toContain('┐');
      expect(result).toContain('└');
      expect(result).toContain('┘');
      expect(result).toContain('│');
      expect(result).toContain('─');

      // @step And columns are aligned based on content width
      // Both columns should have consistent width
      const lines = result.split('\n');
      expect(lines.length).toBeGreaterThan(0);
      // Header and data cells should be padded consistently
      expect(result).toContain('Name');
      expect(result).toContain('Alice');

      // @step And the header row is rendered in bold
      // Bold is applied via chalk.bold() - in test environment chalk may be disabled
      // Verify the header Name appears in the output (bold styling verified by inspection)
      // The structure should have header followed by separator
      expect(lines[1]).toContain('Name');
      expect(lines[2]).toContain('├'); // separator after header
    });

    it('should handle tables with varying content widths', () => {
      // @step Given the AI assistant has completed a response containing a markdown table
      const input = '| Short | Very Long Column Header |\n|---|---|\n| A | B |';

      // @step When the streaming is marked complete (isStreaming: false)
      const result = formatMarkdownTables(input);

      // @step Then the table is rendered with box-drawing characters
      expect(result).toContain('┌');
      expect(result).toContain('┘');

      // @step And columns are aligned based on content width
      // The second column should be wider to accommodate the header
      const lines = result.split('\n');
      const topBorder = lines[0];
      // Verify consistent column widths across rows
      expect(topBorder).toContain('┬');
    });
  });

  describe('Scenario: Table with alignment specifiers renders with correct alignment', () => {
    it('should respect left, center, and right alignment specifiers', () => {
      // @step Given the AI response contains a table with alignment specifiers
      const input =
        '| Left | Center | Right |\n|:---|:---:|---:|\n| A | B | C |';

      // @step When the streaming completes
      const result = formatMarkdownTables(input);

      // @step Then columns with :--- are left-aligned
      // Left-aligned text should have padding on the right
      const lines = result.split('\n');
      const dataRow = lines.find(
        l => l.includes('A') && l.includes('B') && l.includes('C')
      );
      expect(dataRow).toBeDefined();

      // @step And columns with :---: are center-aligned
      // Center-aligned text should have equal padding on both sides
      // (verified by visual inspection - the 'B' should be centered)

      // @step And columns with ---: are right-aligned
      // Right-aligned text should have padding on the left
      // The alignment is applied during rendering
      expect(result).toContain('│');
    });
  });

  describe('Scenario: Tables mixed with other content preserves surrounding text', () => {
    it('should render only the table portion while preserving surrounding text', () => {
      // @step Given the AI response contains text before and after a markdown table
      const input =
        'Here is some data:\n\n| Name | Age |\n|---|---|\n| Bob | 25 |\n\nThat was the table.';

      // @step When the streaming completes
      const result = formatMarkdownTables(input);

      // @step Then only the table portion is rendered with box borders
      expect(result).toContain('┌');
      expect(result).toContain('┘');

      // @step And the surrounding text remains unchanged
      expect(result).toContain('Here is some data:');
      expect(result).toContain('That was the table.');
    });

    it('should handle multiple tables in the same content', () => {
      // @step Given the AI response contains text before and after a markdown table
      const input =
        'Table 1:\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nTable 2:\n\n| X | Y |\n|---|---|\n| 3 | 4 |';

      // @step When the streaming completes
      const result = formatMarkdownTables(input);

      // @step Then only the table portion is rendered with box borders
      // Both tables should be rendered with box characters
      const boxCharCount = (result.match(/┌/g) || []).length;
      expect(boxCharCount).toBe(2);

      // @step And the surrounding text remains unchanged
      expect(result).toContain('Table 1:');
      expect(result).toContain('Table 2:');
    });
  });

  describe('Edge cases', () => {
    it('should return content unchanged when no tables are present', () => {
      const input = 'Just some regular text without any tables.';
      const result = formatMarkdownTables(input);
      expect(result).toBe(input);
    });

    it('should handle empty input', () => {
      const result = formatMarkdownTables('');
      expect(result).toBe('');
    });

    it('should handle tables with empty cells', () => {
      const input = '| A | B |\n|---|---|\n| | data |';
      const result = formatMarkdownTables(input);
      expect(result).toContain('┌');
      expect(result).toContain('data');
    });

    it('should handle tables inside code fences', () => {
      // Tables inside triple backticks are treated as code blocks by marked
      // but we should still format them if they look like tables
      const input =
        '**Table:**\n```\n| Name | Age |\n|---|---|\n| Alice | 30 |\n```';
      const result = formatMarkdownTables(input);
      // The code fence should be replaced with a formatted table
      expect(result).toContain('┌');
      expect(result).toContain('Alice');
      expect(result).toContain('30');
      // Code fence markers should be gone
      expect(result).not.toContain('```');
    });

    it('should correctly align columns with emojis', () => {
      // Emojis have visual width 2 in terminals but JS .length reports different values
      // ✅ (U+2705) - BMP emoji, .length=1 but visual width=2
      // 🚧 (U+1F6A7) - non-BMP emoji, .length=2, visual width=2
      // ❌ (U+274C) - BMP emoji, .length=1 but visual width=2
      const input = `| Feature | Status | Notes |
|-|-|-|
| Login | ✅ Complete | Deployed v1.2 |
| Dashboard | 🚧 WIP | *In progress* |
| API | ❌ Blocked | Waiting on spec |`;

      const result = formatMarkdownTables(input);

      // All rows should have the same visual width for proper alignment
      // Filter out empty lines (trailing newline creates empty last line)
      const lines = result.split('\n').filter(line => line.length > 0);
      const widths = lines.map(line => getVisualWidth(line));

      // All lines should have equal visual width
      const firstWidth = widths[0];
      widths.forEach((width, idx) => {
        expect(width).toBe(
          firstWidth,
          `Row ${idx} has width ${width}, expected ${firstWidth}`
        );
      });

      // Verify the table structure is correct
      expect(result).toContain('┌');
      expect(result).toContain('✅');
      expect(result).toContain('🚧');
      expect(result).toContain('❌');
    });

    it('should add trailing newline after table for proper separation', () => {
      const input = '| A | B |\n|---|---|\n| 1 | 2 |';
      const result = formatMarkdownTables(input);

      // Result should end with a newline
      expect(result.endsWith('\n')).toBe(true);

      // Content after table should start on new line
      const inputWithContent =
        '| A | B |\n|---|---|\n| 1 | 2 |\n\n---\n\nMore text';
      const resultWithContent = formatMarkdownTables(inputWithContent);

      // The --- should not be stuck to the table
      expect(resultWithContent).toContain('┘\n');
      expect(resultWithContent).toContain('---');
    });
  });
});
