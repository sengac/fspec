/**
 * ChangedFilesViewer Component - dual-pane viewer for changed files and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';
import { getFileDiff } from '../../git/diff';
import { useFspecStore } from '../store/fspecStore';

interface ChangedFilesViewerProps {
  stagedFiles: string[];
  unstagedFiles: string[];
  onExit: () => void;
}

export const ChangedFilesViewer: React.FC<ChangedFilesViewerProps> = ({
  stagedFiles,
  unstagedFiles,
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<'files' | 'diff'>('files');
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);

  const cwd = useFspecStore(state => state.cwd);

  // File list minimum width in characters
  const fileListMinWidth = 30;

  // Combine staged and unstaged files with status indicators
  const allFiles = [
    ...stagedFiles.map(f => ({ path: f, status: 'staged' as const })),
    ...unstagedFiles.map(f => ({ path: f, status: 'unstaged' as const })),
  ];

  const selectedFile = allFiles[selectedFileIndex];

  // Load real git diff when selected file changes
  useEffect(() => {
    if (!selectedFile) {
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    setIsLoadingDiff(true);
    void (async () => {
      try {
        const diff = await getFileDiff(cwd, selectedFile.path);
        setDiffContent(diff || 'No changes to display');
      } catch (error) {
        setDiffContent('Error loading diff');
      } finally {
        setIsLoadingDiff(false);
      }
    })();
  }, [selectedFile, cwd]);

  // Parse diff content into lines, show loading state
  const diffLines = isLoadingDiff
    ? ['Loading diff...']
    : (diffContent ? diffContent.split('\n') : []);

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
  });

  // Empty state
  if (allFiles.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1}>
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane */}
          <Box flexDirection="column" minWidth={fileListMinWidth} flexBasis="25%" flexShrink={1} borderStyle="single" borderColor="cyan">
            <Text>No changed files</Text>
          </Box>

          {/* Diff pane */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single">
            <Text>No changes to display</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render file item with status indicator
  const renderFileItem = (
    file: { path: string; status: 'staged' | 'unstaged' },
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';
    const statusIcon = file.status === 'staged' ? '+' : 'M';
    const statusColor = file.status === 'staged' ? 'green' : 'yellow';

    return (
      <Box width="100%">
        <Text color={isSelected ? 'cyan' : 'white'}>
          {indicator} <Text color={statusColor}>{statusIcon}</Text> {file.path}
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
            Changed Files: {stagedFiles.length} staged, {unstagedFiles.length} unstaged
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
                  items={allFiles}
                  renderItem={renderFileItem}
                  showScrollbar={true}
                  onFocus={(file, index) => setSelectedFileIndex(index)}
                />
              </Box>
            ) : (
              // Static list when not focused
              <Box flexGrow={1} flexDirection="column">
                {allFiles.map((file, index) => {
                  const statusIcon = file.status === 'staged' ? '+' : 'M';
                  const statusColor = file.status === 'staged' ? 'green' : 'yellow';

                  return (
                    <Text key={index} dimColor>
                      {index === selectedFileIndex ? '> ' : '  '}
                      <Text color={statusColor}>{statusIcon}</Text> {file.path}
                    </Text>
                  );
                })}
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
              // Static diff when not focused
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
