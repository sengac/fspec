/**
 * CheckpointViewer Component - dual-pane viewer for checkpoint files and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';

interface Checkpoint {
  name: string;
  files: string[];
}

interface CheckpointViewerProps {
  checkpoints: Checkpoint[];
  onExit: () => void;
}

export const CheckpointViewer: React.FC<CheckpointViewerProps> = ({
  checkpoints,
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<'files' | 'diff'>('files');
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [selectedCheckpointIndex, setSelectedCheckpointIndex] = useState(0);

  // File list minimum width in characters
  const fileListMinWidth = 30;

  // Get current checkpoint and files
  const currentCheckpoint = checkpoints[selectedCheckpointIndex];
  const files = currentCheckpoint?.files || [];
  const selectedFile = files[selectedFileIndex];

  // Generate mock diff content (TODO: load actual diff from git)
  const diffLines = selectedFile
    ? [
        `Diff for ${selectedFile}`,
        '',
        '@@ -1,3 +1,4 @@',
        '+import { foo } from "bar";',
        ' const x = 1;',
        '-const y = 2;',
        '+const y = 3;',
        ' const z = 4;',
      ]
    : [];

  // Handle keyboard input
  useInput((input, key) => {
    if (key.escape) {
      onExit();
      return;
    }

    // Tab key to switch focus between panes
    if (key.tab) {
      setFocusedPane(prev => (prev === 'files' ? 'diff' : 'files'));
      return;
    }

    // Arrow key navigation handled by VirtualList when focused
    // (VirtualList has its own useInput handler)
  });

  // Empty state
  if (checkpoints.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1}>
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane */}
          <Box flexDirection="column" minWidth={fileListMinWidth} flexBasis="25%" flexShrink={1} borderStyle="single" borderColor="cyan">
            <Text>No checkpoints available</Text>
          </Box>

          {/* Diff pane */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single">
            <Text>Select a checkpoint to view files</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render file item in list
  const renderFileItem = (file: string, index: number, isSelected: boolean): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';
    return (
      <Box width="100%">
        <Text color={isSelected ? 'cyan' : 'white'}>
          {indicator} {file}
        </Text>
      </Box>
    );
  };

  // Render diff line
  const renderDiffLine = (line: string, index: number, isSelected: boolean): React.ReactNode => {
    let color: 'white' | 'green' | 'red' | 'cyan' = 'white';
    if (line.startsWith('+')) {
      color = 'green';
    } else if (line.startsWith('-')) {
      color = 'red';
    } else if (line.startsWith('@@')) {
      color = 'cyan';
    }

    return (
      <Box width="100%">
        <Text color={color}>{line}</Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
        {/* Header */}
        <Box>
          <Text bold>
            Checkpoint: {currentCheckpoint.name} ({files.length} files)
          </Text>
        </Box>

        {/* Dual-pane layout */}
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane (left, ~25% width with 30 char minimum) */}
          <Box
            flexDirection="column"
            minWidth={fileListMinWidth}
            flexBasis="25%"
            flexShrink={1}
            borderStyle="single"
            borderColor={focusedPane === 'files' ? 'cyan' : 'gray'}
          >
            <Text bold>Files</Text>
            {focusedPane === 'files' ? (
              <Box flexGrow={1}>
                <VirtualList
                  items={files}
                  renderItem={renderFileItem}
                  showScrollbar={true}
                  onFocus={(file, index) => setSelectedFileIndex(index)}
                />
              </Box>
            ) : (
              <Box flexGrow={1} flexDirection="column">
                {files.map((file, index) => (
                  <Text key={index} dimColor>
                    {index === selectedFileIndex ? '> ' : '  '}
                    {file}
                  </Text>
                ))}
              </Box>
            )}
          </Box>

          {/* Diff pane (right, grows to fill remaining space) */}
          <Box
            flexDirection="column"
            flexGrow={1}
            borderStyle="single"
            borderColor={focusedPane === 'diff' ? 'cyan' : 'gray'}
          >
            <Text bold>Diff</Text>
            {focusedPane === 'diff' ? (
              <Box flexGrow={1}>
                <VirtualList
                  items={diffLines}
                  renderItem={renderDiffLine}
                  showScrollbar={true}
                />
              </Box>
            ) : (
              <Box flexGrow={1} flexDirection="column">
                {diffLines.map((line, index) => {
                  let color: 'white' | 'green' | 'red' | 'cyan' = 'white';
                  if (line.startsWith('+')) color = 'green';
                  else if (line.startsWith('-')) color = 'red';
                  else if (line.startsWith('@@')) color = 'cyan';
                  return (
                    <Text key={index} color={color} dimColor>
                      {line}
                    </Text>
                  );
                })}
              </Box>
            )}
          </Box>
        </Box>

        {/* Footer */}
        <Box>
          <Text dimColor>
            ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll
          </Text>
        </Box>
      </Box>
  );
};
