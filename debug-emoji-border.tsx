#!/usr/bin/env npx tsx
/**
 * Debug script to verify emoji border fix
 * Run with: npx tsx debug-emoji-border.tsx
 */

import React from 'react';
import { render, Box, Text } from 'ink';
import stringWidth from 'string-width';

const DebugModal: React.FC = () => {
  const terminalWidth = 80;
  const terminalHeight = 24;

  const lines = [
    { role: 'user', content: 'You: what is this project about?' },
    { role: 'assistant', content: 'AI: I will explore the project.' },
    { role: 'tool', content: '[Tool result preview]' },
    { role: 'tool', content: '-------' },
    { role: 'assistant', content: 'AI:' },
    // Use ⚠ (U+26A0) without emoji selector - width 1 in both string-width and terminal
    { role: 'tool', content: '⚠ Agent interrupted' },
    { role: 'assistant', content: ' ' },
  ];

  return (
    <Box
      flexDirection="column"
      width={terminalWidth}
      height={terminalHeight}
    >
      <Box
        flexDirection="column"
        flexGrow={1}
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
            <Text bold color="cyan">Agent: claude</Text>
          </Box>
          <Box>
            <Text dimColor>tokens: 0↓ 0↑</Text>
          </Box>
        </Box>

        {/* Content */}
        <Box flexGrow={1} flexDirection="column">
          {lines.map((line, i) => {
            const color = line.role === 'user' ? 'green' : line.role === 'tool' ? 'yellow' : 'white';
            return (
              <Box key={i}>
                <Text color={color}>{line.content}</Text>
              </Box>
            );
          })}
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
          <Text color="green">&gt; </Text>
          <Text dimColor>Type your message...</Text>
        </Box>
      </Box>
    </Box>
  );
};

// Render to stdout
render(<DebugModal />);

// Print analysis to stderr
setTimeout(() => {
  console.error('\n=== WIDTH ANALYSIS ===');
  console.error(`⚠ (U+26A0 base): stringWidth=${stringWidth('⚠')}`);
  console.error(`⚠️ (U+26A0 + U+FE0F): stringWidth=${stringWidth('⚠️')}`);
  console.error(`⚠ Agent interrupted: stringWidth=${stringWidth('⚠ Agent interrupted')}`);
  console.error('');
  console.error('Using ⚠ (without emoji selector) = width 1 everywhere');
  console.error('======================\n');
  process.exit(0);
}, 200);
