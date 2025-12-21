/**
 * Debug test to analyze border rendering with VirtualList
 */

import React, { useState, useEffect, useMemo, useRef, useLayoutEffect } from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Box, Text, measureElement, type DOMElement } from 'ink';
import stringWidth from 'string-width';

// Simplified VirtualList for testing
const SCROLLBAR_CHARS = { square: '■', line: '│' };

interface VirtualListProps<T> {
  items: T[];
  renderItem: (item: T, index: number, isSelected: boolean) => React.ReactNode;
  keyExtractor?: (item: T, index: number) => string;
  showScrollbar?: boolean;
}

function TestVirtualList<T>({
  items,
  renderItem,
  keyExtractor = (_item: T, index: number) => String(index),
  showScrollbar = true,
}: VirtualListProps<T>): React.ReactElement {
  const visibleHeight = 10;
  const visibleItems = items.slice(0, visibleHeight);

  const renderScrollbar = (): React.ReactNode => {
    if (!showScrollbar || items.length <= visibleHeight) {
      return null;
    }
    const scrollbarChars = Array(visibleHeight).fill(SCROLLBAR_CHARS.line);
    scrollbarChars[0] = SCROLLBAR_CHARS.square;

    return (
      <Box flexDirection="column" marginLeft={1}>
        {scrollbarChars.map((char, i) => (
          <Text key={i} dimColor>{char}</Text>
        ))}
      </Box>
    );
  };

  return (
    <Box flexDirection="row" flexGrow={1}>
      <Box flexDirection="column" flexGrow={1}>
        {visibleItems.map((item, index) => (
          <Box key={keyExtractor(item, index)}>
            {renderItem(item, index, index === 0)}
          </Box>
        ))}
      </Box>
      {renderScrollbar()}
    </Box>
  );
}

interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
}

describe('Border rendering with VirtualList', () => {
  it('should render all borders correctly with VirtualList inside', () => {
    const terminalWidth = 80;
    const terminalHeight = 24;

    const conversationLines: ConversationLine[] = [
      { role: 'user', content: 'You: Hello' },
      { role: 'assistant', content: 'AI: Hi there!' },
      { role: 'tool', content: '[Tool call]' },
      { role: 'tool', content: '⚠ Agent interrupted' },  // Use base char (width 1) not emoji variant (width 2)
      { role: 'assistant', content: ' ' },
    ];

    const TestModal = () => (
      <Box
        flexDirection="column"
        width={terminalWidth}
        height={terminalHeight}
        borderStyle="double"
        borderColor="cyan"
      >
        {/* Header */}
        <Box
          borderStyle="single"
          borderBottom={true}
          borderTop={false}
          borderLeft={false}
          borderRight={false}
          paddingX={1}
        >
          <Box flexGrow={1}>
            <Text bold>Agent: claude</Text>
          </Box>
          <Box>
            <Text dimColor>tokens: 0↓ 0↑</Text>
          </Box>
        </Box>

        {/* Content with VirtualList */}
        <Box flexGrow={1} flexBasis={0}>
          <TestVirtualList
            items={conversationLines}
            renderItem={(line, _index, isSelected) => {
              const color = line.role === 'user' ? 'green' : line.role === 'tool' ? 'yellow' : 'white';
              return (
                <Box flexGrow={1}>
                  <Text color={isSelected ? 'cyan' : color}>{line.content}</Text>
                </Box>
              );
            }}
            showScrollbar={true}
          />
        </Box>

        {/* Footer */}
        <Box
          borderStyle="single"
          borderTop={true}
          borderBottom={false}
          borderLeft={false}
          borderRight={false}
          paddingX={1}
        >
          <Text>&gt; </Text>
          <Box flexGrow={1}>
            <Text dimColor>Type here...</Text>
          </Box>
        </Box>
      </Box>
    );

    const { lastFrame } = render(<TestModal />);
    const frame = lastFrame();
    const lines = frame.split('\n');

    console.log('\n=== RENDERED OUTPUT ===');
    lines.forEach((line, i) => {
      const stripped = line.replace(/\x1B\[[0-9;]*m/g, '');
      const width = stringWidth(stripped);
      const hasLeftBorder = stripped.startsWith('╔') || stripped.startsWith('║') || stripped.startsWith('╚');
      const hasRightBorder = stripped.endsWith('╗') || stripped.endsWith('║') || stripped.endsWith('╝');
      console.log(`Line ${i.toString().padStart(2)}: w=${width.toString().padStart(2)} L=${hasLeftBorder ? '✓' : '✗'} R=${hasRightBorder ? '✓' : '✗'} "${stripped.slice(0, 30)}...${stripped.slice(-10)}"`);
    });
    console.log('========================\n');

    // Check that all lines have proper borders
    const strippedLines = lines.map(l => l.replace(/\x1B\[[0-9;]*m/g, ''));

    // Find lines that should have right border but don't
    const linesWithoutRightBorder = strippedLines.filter((line, i) => {
      if (line.length === 0) return false;
      const lastChar = line[line.length - 1];
      const shouldHaveBorder = line.startsWith('╔') || line.startsWith('║') || line.startsWith('╚');
      const hasRightBorder = lastChar === '║' || lastChar === '╗' || lastChar === '╝';
      return shouldHaveBorder && !hasRightBorder;
    });

    console.log('Lines missing right border:', linesWithoutRightBorder.length);
    if (linesWithoutRightBorder.length > 0) {
      linesWithoutRightBorder.forEach(line => {
        console.log(`  Problem line: "${line.slice(-20)}" (last 20 chars)`);
      });
    }

    // The test: all border lines should have both left and right borders
    expect(linesWithoutRightBorder.length).toBe(0);
  });

  it('should correctly calculate warning sign width with string-width 8.x', () => {
    // string-width 8.x correctly handles emoji presentation selectors:
    // - ⚠ (U+26A0 alone) = width 1 (text presentation) <- USE THIS
    // - ⚠️ (U+26A0 + U+FE0F) = width 2 (emoji presentation) <- DON'T USE (terminal may render as 1)

    const baseChar = '⚠';
    const baseText = '⚠ Agent interrupted';

    console.log(`\n=== STRING-WIDTH 8.x TEST ===`);
    console.log(`Base ⚠ (U+26A0): stringWidth=${stringWidth(baseChar)}`);
    console.log(`Full text: "${baseText}" -> stringWidth=${stringWidth(baseText)}`);
    console.log(`=============================\n`);

    // Base warning sign without variation selector is width 1
    // This matches what terminals actually render
    expect(stringWidth(baseChar)).toBe(1);

    // Full text: warning(1) + space(1) + "Agent interrupted"(17) = 19
    expect(stringWidth(baseText)).toBe(19);
  });

  it('should render warning line without breaking borders', () => {
    const terminalWidth = 80;

    const TestBox = () => (
      <Box
        flexDirection="column"
        width={terminalWidth}
        height={5}
        borderStyle="double"
      >
        <Text>Normal text line</Text>
        <Text>⚠ Agent interrupted</Text>
        <Text>Another normal line</Text>
      </Box>
    );

    const { lastFrame } = render(<TestBox />);
    const frame = lastFrame();
    const lines = frame.split('\n');

    console.log('\n=== EMOJI LINE TEST ===');
    lines.forEach((line, i) => {
      const stripped = line.replace(/\x1B\[[0-9;]*m/g, '');
      console.log(`Line ${i}: width=${stringWidth(stripped)}, "${stripped}"`);
    });
    console.log('=======================\n');

    // All lines should be exactly 80 chars wide
    const strippedLines = lines.map(l => l.replace(/\x1B\[[0-9;]*m/g, ''));
    strippedLines.forEach((line, i) => {
      if (line.length > 0) {
        expect(stringWidth(line)).toBe(terminalWidth);
      }
    });
  });
});
