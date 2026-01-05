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

  it('should analyze Key Capabilities emojis for width issues', () => {
    // Emojis from the Key Capabilities section in the TUI
    const emojis = [
      { name: 'clipboard', char: '📋', base: '📋' },
      { name: 'memo', char: '📝', base: '📝' },
      { name: 'speech_bubble', char: '💬', base: '💬' },
      { name: 'pencil', char: '✏️', base: '✏' },  // Has variation selector
      { name: 'chart', char: '📊', base: '📊' },
      { name: 'folder', char: '📁', base: '📁' },
      { name: 'desktop', char: '🖥️', base: '🖥' },  // Has variation selector
      { name: 'wrench', char: '🔧', base: '🔧' },
    ];

    console.log('\n=== KEY CAPABILITIES EMOJI ANALYSIS ===');
    console.log('Analyzing emojis with and without variation selectors (U+FE0F):\n');

    emojis.forEach(e => {
      const charCodePoints = [...e.char].map(c => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0'));
      const baseCodePoints = [...e.base].map(c => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0'));
      const charWidth = stringWidth(e.char);
      const baseWidth = stringWidth(e.base);
      const hasVariationSelector = e.char !== e.base;

      console.log(`${e.name}:`);
      console.log(`  With VS:    "${e.char}" → width=${charWidth}, codepoints=[${charCodePoints.join(', ')}]`);
      if (hasVariationSelector) {
        console.log(`  Without VS: "${e.base}" → width=${baseWidth}, codepoints=[${baseCodePoints.join(', ')}]`);
        if (charWidth !== baseWidth) {
          console.log(`  ⚠️  WIDTH MISMATCH: ${charWidth} vs ${baseWidth}`);
        }
      }
      console.log('');
    });
    console.log('==========================================\n');

    // Test that we understand the width calculations
    expect(stringWidth('📋')).toBe(2);  // Standard emoji
    expect(stringWidth('📝')).toBe(2);  // Standard emoji
  });

  it('should test pencil and desktop emojis in bordered box', () => {
    const terminalWidth = 80;

    // Test the two emojis with variation selectors
    const TestBox = () => (
      <Box
        flexDirection="column"
        width={terminalWidth}
        height={12}
        borderStyle="double"
      >
        <Text>- 📋 Project Management: Kanban board</Text>
        <Text>- 📝 Specification Management: Create feature files</Text>
        <Text>- 💬 Example Mapping: Facilitate discovery</Text>
        <Text>- ✏️ TDD Enforcement: Prevents writing code</Text>
        <Text>- 📊 Coverage Tracking: Complete traceability</Text>
        <Text>- 📁 Git Checkpoints: Automatic checkpoints</Text>
        <Text>- 🖥️ Interactive TUI: Live Kanban board</Text>
        <Text>- 🔧 Virtual Hooks: Custom validation</Text>
      </Box>
    );

    const { lastFrame } = render(<TestBox />);
    const frame = lastFrame();
    const lines = frame.split('\n');

    console.log('\n=== KEY CAPABILITIES BOX RENDERING ===');
    lines.forEach((line, i) => {
      const stripped = line.replace(/\x1B\[[0-9;]*m/g, '');
      const width = stringWidth(stripped);
      const hasRightBorder = stripped.endsWith('║') || stripped.endsWith('╗') || stripped.endsWith('╝');
      const status = width === terminalWidth ? '✓' : `✗ (off by ${terminalWidth - width})`;
      console.log(`Line ${i.toString().padStart(2)}: w=${width.toString().padStart(2)} ${status} R=${hasRightBorder ? '✓' : '✗'} "${stripped}"`);
    });
    console.log('========================================\n');

    // Check all lines are correct width
    const strippedLines = lines.map(l => l.replace(/\x1B\[[0-9;]*m/g, ''));
    const problemLines = strippedLines.filter((line, i) => {
      if (line.length === 0) return false;
      return stringWidth(line) !== terminalWidth;
    });

    if (problemLines.length > 0) {
      console.log('PROBLEM LINES:');
      problemLines.forEach(line => {
        console.log(`  "${line}"`);
        // Find the emoji in this line
        const emojis = ['📋', '📝', '💬', '✏️', '📊', '📁', '🖥️', '🔧'];
        emojis.forEach(e => {
          if (line.includes(e)) {
            const codePoints = [...e].map(c => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0'));
            console.log(`    Contains: "${e}" [${codePoints.join(', ')}] width=${stringWidth(e)}`);
          }
        });
      });
    }

    expect(problemLines.length).toBe(0);
  });

  it('should compare terminal rendering vs string-width for variation selector emojis', () => {
    // Deep dive into pencil and desktop emojis
    const testCases = [
      { name: 'pencil with VS', emoji: '✏️' },      // U+270F + U+FE0F
      { name: 'pencil without VS', emoji: '✏' },    // U+270F only
      { name: 'desktop with VS', emoji: '🖥️' },     // U+1F5A5 + U+FE0F
      { name: 'desktop without VS', emoji: '🖥' },   // U+1F5A5 only
      { name: 'warning with VS', emoji: '⚠️' },     // U+26A0 + U+FE0F (known issue)
      { name: 'warning without VS', emoji: '⚠' },   // U+26A0 only (our fix)
    ];

    console.log('\n=== VARIATION SELECTOR DEEP DIVE ===');
    console.log('U+FE0F = Variation Selector-16 (emoji presentation)\n');

    testCases.forEach(tc => {
      const codePoints = [...tc.emoji].map(c => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0'));
      const width = stringWidth(tc.emoji);
      const byteLength = Buffer.from(tc.emoji).length;
      const charLength = tc.emoji.length;  // JS string length (UTF-16 code units)

      console.log(`${tc.name}:`);
      console.log(`  Emoji: "${tc.emoji}"`);
      console.log(`  Code points: [${codePoints.join(', ')}]`);
      console.log(`  string-width: ${width}`);
      console.log(`  JS .length: ${charLength}`);
      console.log(`  Byte length (UTF-8): ${byteLength}`);
      console.log('');
    });

    console.log('KEY INSIGHT:');
    console.log('- U+FE0F (VS16) requests emoji presentation (colorful, typically width 2)');
    console.log('- Without VS16, char may render as text (monochrome, width 1 or 2 depends on char)');
    console.log('- Terminal behavior varies - some honor VS16, others ignore it');
    console.log('- string-width tries to predict terminal width but may not match actual rendering');
    console.log('=====================================\n');

    // Verify our understanding
    expect(stringWidth('⚠')).toBe(1);   // Base warning = width 1
    expect(stringWidth('⚠️')).toBe(2);  // Emoji warning = width 2
  });

  it('should reproduce pencil and desktop emoji bug with VirtualList component', () => {
    // FULL COMPONENT REPRODUCTION - same as warning sign test
    // Using ✏️ and 🖥️ emojis with variation selectors
    const terminalWidth = 80;
    const terminalHeight = 24;

    const conversationLines: ConversationLine[] = [
      { role: 'user', content: 'You: what is fspec?' },
      { role: 'assistant', content: 'AI: fspec is a project management tool with:' },
      { role: 'assistant', content: '- 📋 Project Management: Kanban board' },
      { role: 'assistant', content: '- 📝 Specification Management: Gherkin files' },
      { role: 'assistant', content: '- ✏️ TDD Enforcement: Prevents code before tests' },  // BUG: ✏️ with VS16
      { role: 'assistant', content: '- 🖥️ Interactive TUI: Live Kanban board' },           // BUG: 🖥️ with VS16
      { role: 'assistant', content: '- 🔧 Virtual Hooks: Custom validation' },
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

    console.log('\n=== PENCIL/DESKTOP EMOJI BUG REPRODUCTION ===');
    console.log('Testing ✏️ (U+270F+U+FE0F) and 🖥️ (U+1F5A5+U+FE0F) in VirtualList\n');

    lines.forEach((line, i) => {
      const stripped = line.replace(/\x1B\[[0-9;]*m/g, '');
      const width = stringWidth(stripped);
      const hasLeftBorder = stripped.startsWith('╔') || stripped.startsWith('║') || stripped.startsWith('╚');
      const hasRightBorder = stripped.endsWith('╗') || stripped.endsWith('║') || stripped.endsWith('╝');

      // Check if this line contains the problematic emojis
      const hasPencil = stripped.includes('✏');
      const hasDesktop = stripped.includes('🖥');
      const marker = (hasPencil || hasDesktop) ? ' ← EMOJI LINE' : '';

      console.log(`Line ${i.toString().padStart(2)}: w=${width.toString().padStart(2)} L=${hasLeftBorder ? '✓' : '✗'} R=${hasRightBorder ? '✓' : '✗'} "${stripped.slice(0, 50)}..."${marker}`);
    });
    console.log('==============================================\n');

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
        console.log(`  Problem line: "${line.slice(-30)}" (last 30 chars)`);
        // Identify which emoji caused the issue
        if (line.includes('✏')) console.log(`    Contains ✏️ (pencil with VS16)`);
        if (line.includes('🖥')) console.log(`    Contains 🖥️ (desktop with VS16)`);
      });
    }

    // The test: all border lines should have both left and right borders
    expect(linesWithoutRightBorder.length).toBe(0);
  });

  it('should demonstrate the width discrepancy that causes the bug', () => {
    // THE ACTUAL BUG: string-width reports one width, terminal renders another
    //
    // For characters with Emoji_Presentation=No (like ✏, 🖥, ⚠):
    // - Adding U+FE0F REQUESTS emoji presentation (width 2)
    // - string-width correctly reports width 2 when U+FE0F is present
    // - BUT many terminals IGNORE U+FE0F and render as text (width 1)
    //
    // This creates a mismatch:
    // - Ink calculates padding using string-width (thinks emoji is width 2)
    // - Terminal renders the emoji as width 1
    // - Result: line appears 1 character shorter than expected, border misaligned

    const testLines = [
      { text: '- ✏️ TDD Enforcement', emoji: '✏️', name: 'pencil+VS16' },
      { text: '- 🖥️ Interactive TUI', emoji: '🖥️', name: 'desktop+VS16' },
      { text: '- ✏ TDD Enforcement', emoji: '✏', name: 'pencil (base)' },
      { text: '- 🖥 Interactive TUI', emoji: '🖥', name: 'desktop (base)' },
    ];

    console.log('\n=== WIDTH DISCREPANCY DEMONSTRATION ===');
    console.log('This shows WHY the bug happens:\n');

    testLines.forEach(line => {
      const swWidth = stringWidth(line.text);
      const emojiSW = stringWidth(line.emoji);
      const codePoints = [...line.emoji].map(c => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0'));

      // Count actual visible characters (what terminal might render)
      // Characters with Emoji_Presentation=No render as width 1 in many terminals
      const hasVS16 = line.emoji.includes('\uFE0F');
      const terminalEmojiWidth = hasVS16 ? 1 : emojiSW;  // Terminal ignores VS16
      const terminalWidth = swWidth - emojiSW + terminalEmojiWidth;

      console.log(`${line.name}:`);
      console.log(`  Text: "${line.text}"`);
      console.log(`  Emoji codepoints: [${codePoints.join(', ')}]`);
      console.log(`  string-width says: ${swWidth} chars (emoji=${emojiSW})`);
      if (hasVS16) {
        console.log(`  Terminal renders:  ${terminalWidth} chars (emoji=${terminalEmojiWidth}) ← IGNORES U+FE0F!`);
        console.log(`  DISCREPANCY: ${swWidth - terminalWidth} char(s) - border will be off!`);
      } else {
        console.log(`  Terminal renders:  ${terminalWidth} chars (emoji=${terminalEmojiWidth}) ← MATCHES`);
      }
      console.log('');
    });

    console.log('CONCLUSION:');
    console.log('- ✏️ and 🖥️ (with U+FE0F): string-width=2, terminal=1 → OFF BY 1');
    console.log('- ✏ and 🖥 (without U+FE0F): string-width=1, terminal=1 → CORRECT');
    console.log('');
    console.log('FIX: Strip U+FE0F from text-default emoji characters');
    console.log('==========================================\n');

    // Verify the discrepancy exists
    expect(stringWidth('✏️')).toBe(2);  // string-width says 2
    expect(stringWidth('✏')).toBe(1);   // base char is 1
    // Terminal renders ✏️ as 1, not 2 - that's the bug!

    expect(stringWidth('🖥️')).toBe(2);  // string-width says 2
    expect(stringWidth('🖥')).toBe(1);   // base char is 1
    // Terminal renders 🖥️ as 1, not 2 - that's the bug!
  });

  it('should verify normalizeEmojiWidth function strips ALL VS16', () => {
    // Test the normalization function that fixes the bug
    // This function is used in AgentView to normalize incoming text

    // The fix: strip ALL U+FE0F variation selectors
    function normalizeEmojiWidth(text: string): string {
      return text.replace(/\uFE0F/g, '');
    }

    console.log('\n=== NORMALIZATION FUNCTION TEST ===');
    console.log('Stripping ALL U+FE0F variation selectors\n');

    const testCases = [
      { input: '✏️ TDD Enforcement', name: 'pencil' },
      { input: '🖥️ Interactive TUI', name: 'desktop' },
      { input: '⚠️ Warning message', name: 'warning' },
      { input: '☁️ Cloud storage', name: 'cloud' },
      { input: '❤️ Love it', name: 'heart' },
      { input: '✨ Sparkles', name: 'sparkles' },
      { input: '⚡ Lightning', name: 'lightning' },
      { input: '📋 No VS16 here', name: 'clipboard (emoji-default)' },
    ];

    testCases.forEach(tc => {
      const result = normalizeEmojiWidth(tc.input);
      const inputWidth = stringWidth(tc.input);
      const resultWidth = stringWidth(result);
      const hasVS16 = tc.input.includes('\uFE0F');

      console.log(`${tc.name}:`);
      console.log(`  Input:  "${tc.input}" (width=${inputWidth})${hasVS16 ? ' [has VS16]' : ''}`);
      console.log(`  Output: "${result}" (width=${resultWidth})`);
      if (hasVS16) {
        console.log(`  Width change: ${inputWidth} -> ${resultWidth} (${inputWidth - resultWidth} char saved)`);
      }
      console.log('');
    });

    // Verify the fix resolves the width discrepancy for ALL text-default emojis
    const textDefaultEmojis = ['✏️', '🖥️', '⚠️', '☁️', '❤️', '✨', '⚡'];
    console.log('WIDTH FIX VERIFICATION (text-default emojis):');
    textDefaultEmojis.forEach(emoji => {
      const fixed = normalizeEmojiWidth(emoji);
      const hasVS16 = emoji.includes('\uFE0F');
      if (hasVS16) {
        console.log(`  ${emoji} -> ${fixed}: width ${stringWidth(emoji)} -> ${stringWidth(fixed)}`);
        // Text-default with VS16: width 2 -> 1
        expect(stringWidth(emoji)).toBe(2);
        expect(stringWidth(fixed)).toBe(1);
      }
    });
    console.log('  All text-default emojis now have consistent width!');
    console.log('====================================\n');
  });

  it('should work correctly with base emojis (without VS16) in VirtualList', () => {
    // THE FIX: Use base characters without variation selector
    const terminalWidth = 80;
    const terminalHeight = 24;

    const conversationLines: ConversationLine[] = [
      { role: 'user', content: 'You: what is fspec?' },
      { role: 'assistant', content: 'AI: fspec is a project management tool with:' },
      { role: 'assistant', content: '- 📋 Project Management: Kanban board' },
      { role: 'assistant', content: '- 📝 Specification Management: Gherkin files' },
      { role: 'assistant', content: '- ✏ TDD Enforcement: Prevents code before tests' },  // FIX: ✏ without VS16
      { role: 'assistant', content: '- 🖥 Interactive TUI: Live Kanban board' },           // FIX: 🖥 without VS16
      { role: 'assistant', content: '- 🔧 Virtual Hooks: Custom validation' },
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

    console.log('\n=== FIXED VERSION (without VS16) ===');
    console.log('Testing ✏ (U+270F) and 🖥 (U+1F5A5) without variation selectors\n');

    lines.forEach((line, i) => {
      const stripped = line.replace(/\x1B\[[0-9;]*m/g, '');
      const width = stringWidth(stripped);
      const hasLeftBorder = stripped.startsWith('╔') || stripped.startsWith('║') || stripped.startsWith('╚');
      const hasRightBorder = stripped.endsWith('╗') || stripped.endsWith('║') || stripped.endsWith('╝');

      const hasPencil = stripped.includes('✏');
      const hasDesktop = stripped.includes('🖥');
      const marker = (hasPencil || hasDesktop) ? ' ← EMOJI LINE (FIXED)' : '';

      console.log(`Line ${i.toString().padStart(2)}: w=${width.toString().padStart(2)} L=${hasLeftBorder ? '✓' : '✗'} R=${hasRightBorder ? '✓' : '✗'} "${stripped.slice(0, 50)}..."${marker}`);
    });
    console.log('====================================\n');

    // Check that all lines have proper borders
    const strippedLines = lines.map(l => l.replace(/\x1B\[[0-9;]*m/g, ''));

    const linesWithoutRightBorder = strippedLines.filter((line, i) => {
      if (line.length === 0) return false;
      const lastChar = line[line.length - 1];
      const shouldHaveBorder = line.startsWith('╔') || line.startsWith('║') || line.startsWith('╚');
      const hasRightBorder = lastChar === '║' || lastChar === '╗' || lastChar === '╝';
      return shouldHaveBorder && !hasRightBorder;
    });

    console.log('Lines missing right border:', linesWithoutRightBorder.length);

    // With the fix, all borders should be correct
    expect(linesWithoutRightBorder.length).toBe(0);
  });
});
