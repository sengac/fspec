/**
 * ChangedFilesViewer Component - dual-pane viewer for changed files and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 * - TUI-002: Checkpoint Viewer Three-Pane Layout (refactored to use FileDiffViewer)
 */

import React, { useState, useMemo } from 'react';
import { Box, Text, useInput } from 'ink';
import { FileDiffViewer, FileItem } from './FileDiffViewer';
import { logger } from '../../utils/logger';

interface ChangedFilesViewerProps {
  stagedFiles: string[];
  unstagedFiles: string[];
  onExit: () => void;
}

const ChangedFilesViewerComponent: React.FC<ChangedFilesViewerProps> = ({
  stagedFiles,
  unstagedFiles,
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<'files' | 'diff'>('files');
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);

  // Combine staged and unstaged files with status indicators
  const allFiles: FileItem[] = useMemo(() => {
    logger.info(`[ChangedFilesViewer] Recomputing allFiles (staged=${stagedFiles.length}, unstaged=${unstagedFiles.length})`);
    return [
      ...stagedFiles.map(f => ({ path: f, status: 'staged' as const })),
      ...unstagedFiles.map(f => ({ path: f, status: 'unstaged' as const })),
    ];
  }, [stagedFiles, unstagedFiles]);

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

  // Custom render file item with status indicator for staged/unstaged
  const renderFileItem = (
    file: FileItem,
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

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <Box>
        <Text bold>
          Changed Files: {stagedFiles.length} staged, {unstagedFiles.length} unstaged
        </Text>
      </Box>

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

// Export the memoized component
export const ChangedFilesViewer = React.memo(ChangedFilesViewerComponent, (prevProps, nextProps) => {
  // Custom comparison function for React.memo
  // Return true if props are equal (skip re-render), false if different (re-render)

  logger.info(`[ChangedFilesViewer] React.memo comparison function CALLED`)

  // Compare array lengths first (fast check)
  if (prevProps.stagedFiles.length !== nextProps.stagedFiles.length) {
    logger.info(`[ChangedFilesViewer] React.memo: stagedFiles length changed (${prevProps.stagedFiles.length} → ${nextProps.stagedFiles.length})`);
    return false;
  }
  if (prevProps.unstagedFiles.length !== nextProps.unstagedFiles.length) {
    logger.info(`[ChangedFilesViewer] React.memo: unstagedFiles length changed (${prevProps.unstagedFiles.length} → ${nextProps.unstagedFiles.length})`);
    return false;
  }

  // Deep compare array contents
  const stagedEqual = prevProps.stagedFiles.every((file, index) => file === nextProps.stagedFiles[index]);
  const unstagedEqual = prevProps.unstagedFiles.every((file, index) => file === nextProps.unstagedFiles[index]);

  if (!stagedEqual) {
    logger.info(`[ChangedFilesViewer] React.memo: stagedFiles content changed`);
    return false;
  }
  if (!unstagedEqual) {
    logger.info(`[ChangedFilesViewer] React.memo: unstagedFiles content changed`);
    return false;
  }

  // onExit is a function, compare by reference (parent should memoize it)
  if (prevProps.onExit !== nextProps.onExit) {
    logger.info(`[ChangedFilesViewer] React.memo: onExit function changed (parent re-created it)`);
    return false;
  }

  // All props are equal, skip re-render
  logger.info(`[ChangedFilesViewer] React.memo: Props unchanged, SKIPPING RE-RENDER`);
  return true;
});
