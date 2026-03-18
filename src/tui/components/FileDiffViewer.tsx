/**
 * FileDiffViewer Component - Shared dual-pane viewer for file list and diffs
 *
 * Coverage:
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 * - GIT-040: Replace diff-worker.ts with native Rust NAPI diff operations
 *
 * This component extracts the common file list + diff pane logic from
 * ChangedFilesViewer and CheckpointViewer to eliminate code duplication (DRY).
 * Diffs are loaded via synchronous NAPI calls to the Rust gitoxide backend.
 */

import React, { useState, useEffect, useMemo } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { useFspecStore } from '../store/fspecStore';
import { logger } from '../../utils/logger';
import { parseDiff } from '../../git/diff-parser';
import type { DiffLine } from '../../git/diff-parser';
import { getFileDiff } from '../../git/diff';

export interface FileItem {
  path: string;
  status: 'staged' | 'unstaged' | 'checkpoint';
  changeType?: 'A' | 'M' | 'D' | 'R';
}

export interface FileDiffViewerProps {
  files: FileItem[];
  focusedPane: 'files' | 'diff';
  onFocusChange: (pane: 'files' | 'diff') => void;
  onFileSelect: (file: FileItem, index: number) => void;
  selectedFileIndex?: number;
  renderFileItem?: (file: FileItem, index: number, isSelected: boolean) => React.ReactNode;
  diffLines?: DiffLine[]; // Optional: for testing or pre-parsed diffs
}

export const FileDiffViewer: React.FC<FileDiffViewerProps> = ({
  files,
  focusedPane,
  onFocusChange,
  onFileSelect,
  selectedFileIndex = 0,
  renderFileItem: customRenderFileItem,
  diffLines: externalDiffLines,
}) => {
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);

  const cwd = useFspecStore(state => state.cwd);

  // Load git diff when selected file changes via direct NAPI call
  useEffect(() => {
    const selectedFile = files[selectedFileIndex];

    if (!selectedFile) {
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    // Handle deleted files - show message instead of loading diff
    if (selectedFile.changeType === 'D') {
      setDiffContent('File was deleted');
      setIsLoadingDiff(false);
      return;
    }

    setIsLoadingDiff(true);

    try {
      const diff = getFileDiff(cwd, selectedFile.path);

      // Truncate large diffs to prevent UX hangs
      const MAX_DIFF_SIZE = 100000; // 100KB max
      let finalDiff = diff || 'No changes to display';
      if (finalDiff.length > MAX_DIFF_SIZE) {
        const truncatedDiff = finalDiff.substring(0, MAX_DIFF_SIZE);
        const linesShown = truncatedDiff.split('\n').length;
        const totalLines = finalDiff.split('\n').length;
        finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${finalDiff.length} chars)`;
      }

      setDiffContent(finalDiff);
    } catch (error) {
      logger.error(`Failed to load diff for ${selectedFile.path}: ${error}`);
      setDiffContent('Error loading diff');
    } finally {
      setIsLoadingDiff(false);
    }
  }, [selectedFileIndex, files, cwd]);

  // Parse diff content into structured DiffLine objects
  // Use external diff lines if provided (for testing), otherwise load from git
  const diffLines: DiffLine[] = useMemo(() => {
    if (externalDiffLines) {
      return externalDiffLines;
    }
    if (isLoadingDiff) {
      return [{ content: 'Loading diff...', type: 'context', changeGroup: null }];
    }
    if (!diffContent) {
      return [];
    }
    return parseDiff(diffContent);
  }, [externalDiffLines, diffContent, isLoadingDiff]);

  // Empty state
  if (files.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false} borderBottom={true}>
        <Box flexDirection="column" flexGrow={1}>
          {/* File list pane (33% height via flexGrow ratio) */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderBottom={true} borderLeft={false} borderRight={false}>
            <Text>No files</Text>
          </Box>

          {/* Diff pane (67% height via flexGrow ratio) */}
          <Box flexDirection="column" flexGrow={2}>
            <Text>No changes to display</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Default render file item with status indicator
  const defaultRenderFileItem = (
    file: FileItem,
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';
    const statusIcon = file.status === 'staged' ? '+' : file.status === 'unstaged' ? 'M' : '';
    const statusColor = file.status === 'staged' ? 'green' : file.status === 'unstaged' ? 'yellow' : 'white';

    return (
      <Box flexGrow={1}>
        <Text color={isSelected ? 'cyan' : 'white'} wrap="truncate">
          {indicator} {statusIcon && <Text color={statusColor}>{statusIcon}</Text>} {file.path}
        </Text>
      </Box>
    );
  };

  // Render diff line with syntax highlighting
  const renderDiffLine = (line: DiffLine, index: number, isSelected: boolean): React.ReactNode => {
    let textColor: 'white' | 'cyan' = 'white';
    let backgroundColor: string | undefined;

    // Determine colors based on line type
    if (line.type === 'hunk') {
      textColor = 'cyan';
    } else if (line.type === 'removed') {
      textColor = 'white';
      backgroundColor = '#8B0000'; // Dark red
    } else if (line.type === 'added') {
      textColor = 'white';
      backgroundColor = '#006400'; // Dark green
    }

    // Apply selection styling if focused
    const selectionColor = isSelected && focusedPane === 'diff' ? 'cyan' : textColor;
    const selectionInverse = isSelected && focusedPane === 'diff';

    return (
      <Box flexGrow={1}>
        <Text
          color={selectionInverse ? selectionColor : textColor}
          backgroundColor={backgroundColor}
          inverse={selectionInverse}
          wrap="truncate"
        >
          {line.content}
        </Text>
      </Box>
    );
  };

  const renderItem = customRenderFileItem || defaultRenderFileItem;

  return (
    <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false} borderBottom={true}>
      <Box flexDirection="column" flexGrow={1}>
        {/* File list pane (top, 33% height via flexGrow ratio) */}
        <Box
          flexDirection="column"
          flexGrow={1}
          flexBasis={0}
          borderStyle="single"
          borderTop={false}
          borderBottom={true}
          borderLeft={false}
          borderRight={false}
        >
          {/* File list heading */}
          <Box
            backgroundColor={focusedPane === 'files' ? 'green' : undefined}
            borderStyle="single"
            borderTop={false}
            borderLeft={false}
            borderRight={false}
            borderBottom={true}
          >
            <Text
              bold={focusedPane !== 'files'}
              color={focusedPane === 'files' ? 'black' : 'white'}
            >
              Files
            </Text>
          </Box>
          <VirtualList
            items={files}
            renderItem={renderItem}
            showScrollbar={focusedPane === 'files'}
            isFocused={focusedPane === 'files'}
            heightAdjustment={-1}
            onFocus={(file, index) => onFileSelect(file, index)}
          />
        </Box>

        {/* Diff pane (bottom, 67% height via flexGrow ratio) */}
        <Box
          flexDirection="column"
          flexGrow={2}
          flexBasis={0}
        >
          {/* Diff pane heading */}
          <Box
            backgroundColor={focusedPane === 'diff' ? 'green' : undefined}
            borderStyle="single"
            borderTop={false}
            borderLeft={false}
            borderRight={false}
            borderBottom={true}
          >
            <Text
              bold={focusedPane !== 'diff'}
              color={focusedPane === 'diff' ? 'black' : 'white'}
            >
              Diff
            </Text>
          </Box>
          <VirtualList
            items={diffLines}
            renderItem={renderDiffLine}
            showScrollbar={focusedPane === 'diff'}
            isFocused={focusedPane === 'diff'}
            selectionMode="scroll"
          />
        </Box>
      </Box>
    </Box>
  );
};
