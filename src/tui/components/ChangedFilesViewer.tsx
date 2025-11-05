/**
 * ChangedFilesViewer Component - dual-pane viewer for changed files and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 * - TUI-002: Checkpoint Viewer Three-Pane Layout (refactored to use FileDiffViewer)
 * - TUI-014: Remove file watching from TUI main screen and lazy-load changed files view
 */

import React, { useState, useMemo, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { FileDiffViewer, FileItem } from './FileDiffViewer';
import { logger } from '../../utils/logger';
import { useFspecStore } from '../store/fspecStore';

interface ChangedFilesViewerProps {
  onExit: () => void;
  terminalWidth?: number;
  terminalHeight?: number;
}

const ChangedFilesViewerComponent: React.FC<ChangedFilesViewerProps> = ({
  onExit,
}) => {
  // Lazy-load file status from store on mount (TUI-014)
  const stagedFiles = useFspecStore(state => state.stagedFiles);
  const unstagedFiles = useFspecStore(state => state.unstagedFiles);
  const loadFileStatus = useFspecStore(state => state.loadFileStatus);

  // Load file status on mount (lazy loading)
  useEffect(() => {
    void loadFileStatus();
  }, [loadFileStatus]);
  const [focusedPane, setFocusedPane] = useState<'files' | 'diff'>('files');
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);

  // Combine staged and unstaged files with status indicators
  const allFiles: FileItem[] = useMemo(() => {
    logger.info(`[ChangedFilesViewer] Recomputing allFiles (staged=${stagedFiles.length}, unstaged=${unstagedFiles.length})`);
    return [
      ...stagedFiles.map(f => ({ path: f.filepath, status: 'staged' as const, changeType: f.changeType })),
      ...unstagedFiles.map(f => ({ path: f.filepath, status: 'unstaged' as const, changeType: f.changeType })),
    ];
  }, [stagedFiles, unstagedFiles]);

  // Handle keyboard input
  useInput((input, key) => {
    if (key.escape) {
      onExit();
      return;
    }

    // Tab key or right arrow to switch focus forward between panes
    if (key.tab || key.rightArrow) {
      setFocusedPane(prev => (prev === 'files' ? 'diff' : 'files'));
      return;
    }

    // Left arrow to switch focus backward between panes
    if (key.leftArrow) {
      setFocusedPane(prev => (prev === 'files' ? 'diff' : 'files'));
      return;
    }

    // Up/down arrow key navigation handled by VirtualList when focused
  });

  // Custom render file item with status indicator for staged/unstaged
  const renderFileItem = (
    file: FileItem,
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';

    // Determine status icon and color based on change type
    const changeType = file.changeType || 'M';
    let statusIcon: string;
    let statusColor: string;

    switch (changeType) {
      case 'A': // Added
        statusIcon = 'A';
        statusColor = 'green';
        break;
      case 'M': // Modified
        statusIcon = 'M';
        statusColor = 'yellow';
        break;
      case 'D': // Deleted
        statusIcon = 'D';
        statusColor = 'red';
        break;
      case 'R': // Renamed
        statusIcon = 'R';
        statusColor = 'cyan';
        break;
      default:
        statusIcon = 'M';
        statusColor = 'yellow';
    }

    return (
      <Box width="100%">
        <Text color={isSelected ? 'cyan' : 'white'} wrap="truncate">
          {indicator} <Text color={statusColor}>{statusIcon}</Text> {file.path}
        </Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Use shared FileDiffViewer for dual-pane layout */}
      <FileDiffViewer
        files={allFiles}
        focusedPane={focusedPane}
        onFocusChange={setFocusedPane}
        onFileSelect={(file, index) => setSelectedFileIndex(index)}
        selectedFileIndex={selectedFileIndex}
        renderFileItem={renderFileItem}
      />

      {/* Footer */}
      <Box>
        <Text dimColor>
          ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll
        </Text>
      </Box>
    </Box>
  );
};

// Export the component (Zustand handles re-render optimization, no custom comparison needed)
export const ChangedFilesViewer = React.memo(ChangedFilesViewerComponent);
