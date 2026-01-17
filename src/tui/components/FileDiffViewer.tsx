/**
 * FileDiffViewer Component - Shared dual-pane viewer for file list and diffs
 *
 * Coverage:
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 *
 * This component extracts the common file list + diff pane logic from
 * ChangedFilesViewer and CheckpointViewer to eliminate code duplication (DRY).
 */

import React, { useState, useEffect, useMemo, useRef } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { useFspecStore } from '../store/fspecStore';
import { logger } from '../../utils/logger';
import { Worker } from 'worker_threads';
import { parseDiff, DiffLine } from '../../git/diff-parser';
import { getWorkerPath } from '../../git/worker-path';

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

  // File list minimum width in characters
  const fileListMinWidth = 30;

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);
  const componentId = useRef(`FileDiffViewer-${Date.now()}`);

  // Initialize worker thread on mount
  useEffect(() => {
    const workerPath = getWorkerPath();

    try {
      workerRef.current = new Worker(workerPath);
    } catch (error) {
      logger.error(`[${componentId.current}] Failed to initialize worker: ${error}`);
    }

    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Load git diff when selected file changes using worker thread
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

    if (!workerRef.current) {
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    setIsLoadingDiff(true);

    const startTime = Date.now();
    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: { id: string; diff?: string; error?: string }) => {
      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        return;
      }

      if (response.error) {
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;

        // Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
    });

    // Cleanup function to cancel pending requests
    return () => {
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
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
          <Box flexGrow={1}>
            <VirtualList
              items={diffLines}
              renderItem={renderDiffLine}
              showScrollbar={focusedPane === 'diff'}
              isFocused={focusedPane === 'diff'}
            />
          </Box>
        </Box>
      </Box>
    </Box>
  );
};
